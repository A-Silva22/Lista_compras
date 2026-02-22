import re
from datetime import timedelta
from django.shortcuts import render, redirect, get_object_or_404
from django.http import JsonResponse
from django.contrib.auth import login, logout, get_user_model
from django.contrib.auth.decorators import login_required
from django.contrib.auth.hashers import make_password
from django.db.models import Q, Count
from django.utils import timezone
from .models import Artigo, Lista, ListaPartilha, LinkPartilha
from .backends import HashedPasswordBackend

User = get_user_model()


# ── Helpers ──────────────────────────────────────────────────────

def _listas_do_utilizador(user):
    """Return all lists owned by or shared with user."""
    return Lista.objects.filter(
        Q(dono=user) | Q(partilhas__utilizador=user)
    ).distinct()


def _lista_ativa(request):
    """Return the currently-selected list (from session) or the first available."""
    user = request.user
    lista_id = request.session.get('lista_ativa')
    listas = _listas_do_utilizador(user)
    if lista_id:
        lista = listas.filter(pk=lista_id).first()
        if lista:
            return lista
    lista = listas.first()
    if lista:
        request.session['lista_ativa'] = lista.pk
    return lista


def _pode_aceder_lista(user, lista):
    """Check if user owns or has shared access to the list."""
    if lista.dono == user:
        return True
    return lista.partilhas.filter(utilizador=user).exists()


# ── Auth views ───────────────────────────────────────────────────

def _associar_link_lista(request, user):
    """If session has a pending link token, store it for the popup confirmation."""
    token = request.session.pop('link_token', None)
    if token:
        try:
            link = LinkPartilha.objects.select_related('lista').get(token=token)
            if link.esta_ativo and link.lista.dono != user:
                already_shared = ListaPartilha.objects.filter(
                    lista=link.lista, utilizador=user
                ).exists()
                if not already_shared:
                    request.session['pending_link_token'] = str(token)
                    request.session['pending_link_lista_nome'] = link.lista.nome
        except LinkPartilha.DoesNotExist:
            pass


def registar(request):
    if request.user.is_authenticated:
        return redirect('index')
    next_url = request.GET.get('next', '')
    erro = ''
    if request.method == 'POST':
        next_url = request.POST.get('next', '')
        username = request.POST.get('username', '').strip()
        password_hash = request.POST.get('password_hash', '').strip()
        if not username or not password_hash:
            erro = 'Preencha todos os campos.'
        elif User.objects.filter(username=username).exists():
            erro = 'Este nome de utilizador já existe.'
        else:
            user = User(username=username)
            user.password = make_password(password_hash)
            user.save()
            Lista.objects.create(nome='Casa', dono=user)
            backend = HashedPasswordBackend()
            user = backend.authenticate(request, username=username, password_hash=password_hash)
            if user:
                login(request, user, backend='compras.backends.HashedPasswordBackend')
                request.session.pop('link_token', None)
            if next_url:
                return redirect(next_url)
            return redirect('index')
    return render(request, 'compras/registar.html', {'erro': erro, 'next': next_url})


def entrar(request):
    if request.user.is_authenticated:
        return redirect('index')
    next_url = request.GET.get('next', '')
    erro = ''
    if request.method == 'POST':
        next_url = request.POST.get('next', '')
        username = request.POST.get('username', '').strip()
        password_hash = request.POST.get('password_hash', '').strip()
        backend = HashedPasswordBackend()
        user = backend.authenticate(request, username=username, password_hash=password_hash)
        if user:
            login(request, user, backend='compras.backends.HashedPasswordBackend')
            _associar_link_lista(request, user)
            if request.session.get('pending_link_token'):
                return redirect('index')
            if next_url:
                return redirect(next_url)
            return redirect('index')
        else:
            erro = 'Nome de utilizador ou palavra-passe incorretos.'
    return render(request, 'compras/entrar.html', {'erro': erro, 'next': next_url})


def sair(request):
    logout(request)
    return redirect('entrar')


# ── Main index ───────────────────────────────────────────────────

@login_required
def index(request):
    lista = _lista_ativa(request)
    listas = _listas_do_utilizador(request.user).annotate(
        n_partilhas=Count('partilhas')
    ).order_by('-criado_em')
    if lista:
        despensa = lista.artigos.filter(comprar=False)
        a_comprar = lista.artigos.filter(comprar=True)
        partilhas = lista.partilhas.select_related('utilizador')
        e_dono = lista.dono == request.user
    else:
        despensa = Artigo.objects.none()
        a_comprar = Artigo.objects.none()
        partilhas = ListaPartilha.objects.none()
        e_dono = False
    pending_link_token = request.session.get('pending_link_token')
    pending_link_nome = request.session.get('pending_link_lista_nome')
    return render(request, 'compras/index.html', {
        'despensa': despensa,
        'a_comprar': a_comprar,
        'lista_ativa': lista,
        'listas': listas,
        'partilhas': partilhas,
        'e_dono': e_dono,
        'pending_link_token': pending_link_token,
        'pending_link_nome': pending_link_nome,
    })


# ── Accept / reject shared link ──────────────────────────────────

@login_required
def responder_link(request):
    """Accept or reject a pending shared list from a link."""
    if request.method == 'POST':
        aceitar = request.POST.get('aceitar') == '1'
        token = request.session.pop('pending_link_token', None)
        request.session.pop('pending_link_lista_nome', None)
        if aceitar and token:
            try:
                link = LinkPartilha.objects.select_related('lista').get(token=token)
                if link.esta_ativo and link.lista.dono != request.user:
                    ListaPartilha.objects.get_or_create(
                        lista=link.lista, utilizador=request.user
                    )
                    request.session['lista_ativa'] = link.lista.pk
            except LinkPartilha.DoesNotExist:
                pass
    return redirect('index')


# ── List management ──────────────────────────────────────────────

@login_required
def criar_lista(request):
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        if nome:
            lista, created = Lista.objects.get_or_create(nome=nome, dono=request.user)
            request.session['lista_ativa'] = lista.pk
    return redirect('index')


@login_required
def selecionar_lista(request, pk):
    lista = get_object_or_404(Lista, pk=pk)
    if _pode_aceder_lista(request.user, lista):
        request.session['lista_ativa'] = lista.pk
    return redirect('index')


@login_required
def renomear_lista(request, pk):
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        if nome:
            lista.nome = nome
            lista.save()
    return redirect('index')


@login_required
def clonar_lista(request, pk):
    lista = get_object_or_404(Lista, pk=pk)
    if not _pode_aceder_lista(request.user, lista):
        return redirect('index')
    if request.method == 'POST':
        nome_base = lista.nome + ' (cópia)'
        nome = nome_base
        n = 1
        while Lista.objects.filter(nome=nome, dono=request.user).exists():
            n += 1
            nome = f"{nome_base} {n}"
        nova_lista = Lista.objects.create(nome=nome, dono=request.user)
        for artigo in lista.artigos.all():
            Artigo.objects.create(
                lista=nova_lista,
                nome=artigo.nome,
                quantidade=artigo.quantidade,
                comprar=artigo.comprar,
            )
        request.session['lista_ativa'] = nova_lista.pk
    return redirect('index')


@login_required
def apagar_lista(request, pk):
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    if request.method == 'POST':
        if request.session.get('lista_ativa') == lista.pk:
            request.session.pop('lista_ativa', None)
        lista.delete()
    return redirect('index')


@login_required
def partilhar_lista(request, pk):
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    if request.method == 'POST':
        nome_utilizador = request.POST.get('username', '').strip()
        is_ajax = request.headers.get('X-Requested-With') == 'XMLHttpRequest'
        if not nome_utilizador:
            if is_ajax:
                return JsonResponse({'ok': False, 'msg': 'Introduza um nome de utilizador.'})
            return redirect('index')
        if nome_utilizador == request.user.username:
            if is_ajax:
                return JsonResponse({'ok': False, 'msg': 'Não pode partilhar consigo mesmo.'})
            return redirect('index')
        try:
            outro = User.objects.get(username=nome_utilizador)
            _, created = ListaPartilha.objects.get_or_create(lista=lista, utilizador=outro)
            if is_ajax:
                if created:
                    return JsonResponse({'ok': True, 'msg': f'Lista partilhada com "{nome_utilizador}".'})
                else:
                    return JsonResponse({'ok': True, 'msg': f'A lista já está partilhada com "{nome_utilizador}".'})
        except User.DoesNotExist:
            if is_ajax:
                return JsonResponse({'ok': False, 'msg': f'Utilizador "{nome_utilizador}" não encontrado.'})
    return redirect('index')


@login_required
def remover_partilha(request, pk, user_pk):
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    if request.method == 'POST':
        ListaPartilha.objects.filter(lista=lista, utilizador_id=user_pk).delete()
    return redirect('index')


# ── Article CRUD (scoped to active list) ─────────────────────────

@login_required
def adicionar(request):
    lista = _lista_ativa(request)
    if request.method == 'POST' and lista and _pode_aceder_lista(request.user, lista):
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            if not quantidade:
                quantidade = '1'
            Artigo.objects.create(lista=lista, nome=nome, quantidade=quantidade, comprar=True)
    return redirect('index')


@login_required
def editar(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if artigo.lista and not _pode_aceder_lista(request.user, artigo.lista):
        return redirect('index')
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            artigo.nome = nome
            artigo.quantidade = quantidade if quantidade else '1'
            artigo.save()
    return redirect('index')


@login_required
def apagar(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if artigo.lista and not _pode_aceder_lista(request.user, artigo.lista):
        return redirect('index')
    if request.method == 'POST':
        artigo.delete()
    return redirect('index')


@login_required
def toggle(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if artigo.lista and not _pode_aceder_lista(request.user, artigo.lista):
        return redirect('index')
    if request.method == 'POST':
        destino = request.POST.get('destino')
        if destino == 'despensa':
            artigo.comprar = False
        elif destino == 'comprar':
            artigo.comprar = True
        else:
            artigo.comprar = not artigo.comprar
        artigo.save()
    return redirect('index')


@login_required
def check_updates(request):
    lista = _lista_ativa(request)
    if lista:
        qs = lista.artigos.all()
    else:
        qs = Artigo.objects.none()
    ultimo = qs.order_by('-movido_em').values_list('movido_em', flat=True).first()
    ts = ultimo.isoformat() if ultimo else ''
    count = qs.count()
    n_listas = _listas_do_utilizador(request.user).count()
    return JsonResponse({'ts': ts, 'count': count, 'n_listas': n_listas})


@login_required
def quantidade_update(request, pk, direcao):
    artigo = get_object_or_404(Artigo, pk=pk)
    if artigo.lista and not _pode_aceder_lista(request.user, artigo.lista):
        return redirect('index')
    if request.method == 'POST':
        match = re.match(r'^(\d+)', artigo.quantidade)
        num = int(match.group(1)) if match else 1
        if direcao == 'mais':
            num += 1
        elif direcao == 'menos' and num > 1:
            num -= 1
        artigo.quantidade = f"{num}"
        artigo.save()
    return redirect('index')


# ── Link sharing (public links) ──────────────────────────────────

@login_required
def criar_link_partilha(request, pk):
    """Owner creates a shared link with permissions and expiry."""
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    if request.method == 'POST':
        duracao = request.POST.get('duracao', '24')
        unidade = request.POST.get('unidade', 'horas')
        try:
            duracao = max(1, int(duracao))
        except (ValueError, TypeError):
            duracao = 24
        if unidade == 'dias':
            delta = timedelta(days=duracao)
        elif unidade == 'minutos':
            delta = timedelta(minutes=duracao)
        else:
            delta = timedelta(hours=duracao)
        link = LinkPartilha.objects.create(
            lista=lista,
            expira_em=timezone.now() + delta,
            pode_adicionar='pode_adicionar' in request.POST,
            pode_editar='pode_editar' in request.POST,
            pode_apagar='pode_apagar' in request.POST,
            pode_toggle='pode_toggle' in request.POST,
        )
        is_ajax = request.headers.get('X-Requested-With') == 'XMLHttpRequest'
        if is_ajax:
            url = request.build_absolute_uri(f'/link/{link.token}/')
            return JsonResponse({'ok': True, 'url': url, 'token': str(link.token)})
    return redirect('index')


@login_required
def apagar_link_partilha(request, pk, link_pk):
    """Owner deletes a shared link."""
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    link = get_object_or_404(LinkPartilha, pk=link_pk, lista=lista)
    if request.method == 'POST':
        link.delete()
        is_ajax = request.headers.get('X-Requested-With') == 'XMLHttpRequest'
        if is_ajax:
            return JsonResponse({'ok': True})
    return redirect('index')


@login_required
def listar_links_partilha(request, pk):
    """Return JSON list of active links for a list (AJAX)."""
    lista = get_object_or_404(Lista, pk=pk, dono=request.user)
    links = lista.links_partilha.filter(expira_em__gt=timezone.now())
    data = []
    for lnk in links:
        data.append({
            'id': lnk.pk,
            'url': request.build_absolute_uri(f'/link/{lnk.token}/'),
            'expira_em': lnk.expira_em.strftime('%d/%m/%Y %H:%M'),
            'pode_adicionar': lnk.pode_adicionar,
            'pode_editar': lnk.pode_editar,
            'pode_apagar': lnk.pode_apagar,
            'pode_toggle': lnk.pode_toggle,
        })
    return JsonResponse({'links': data})


def _get_link_or_404(token):
    """Get an active link or 404."""
    link = get_object_or_404(LinkPartilha, token=token)
    if not link.esta_ativo:
        from django.http import Http404
        raise Http404
    return link


def ver_link(request, token):
    """Public page: view a list via shared link."""
    link = _get_link_or_404(token)
    lista = link.lista
    despensa = lista.artigos.filter(comprar=False)
    a_comprar = lista.artigos.filter(comprar=True)
    # Store token in session so auth views can associate the list
    request.session['link_token'] = str(token)
    return render(request, 'compras/link.html', {
        'link': link,
        'lista': lista,
        'despensa': despensa,
        'a_comprar': a_comprar,
        'token': token,
    })


def link_adicionar(request, token):
    """Add item via shared link."""
    link = _get_link_or_404(token)
    if not link.pode_adicionar:
        return redirect('ver_link', token=token)
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            if not quantidade:
                quantidade = '1'
            Artigo.objects.create(lista=link.lista, nome=nome, quantidade=quantidade, comprar=True)
    return redirect('ver_link', token=token)


def link_toggle(request, token, pk):
    """Toggle item via shared link."""
    link = _get_link_or_404(token)
    if not link.pode_toggle:
        return redirect('ver_link', token=token)
    artigo = get_object_or_404(Artigo, pk=pk, lista=link.lista)
    if request.method == 'POST':
        destino = request.POST.get('destino')
        if destino == 'despensa':
            artigo.comprar = False
        elif destino == 'comprar':
            artigo.comprar = True
        else:
            artigo.comprar = not artigo.comprar
        artigo.save()
    return redirect('ver_link', token=token)


def link_editar(request, token, pk):
    """Edit item via shared link."""
    link = _get_link_or_404(token)
    if not link.pode_editar:
        return redirect('ver_link', token=token)
    artigo = get_object_or_404(Artigo, pk=pk, lista=link.lista)
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            artigo.nome = nome
            artigo.quantidade = quantidade if quantidade else '1'
            artigo.save()
    return redirect('ver_link', token=token)


def link_apagar(request, token, pk):
    """Delete item via shared link."""
    link = _get_link_or_404(token)
    if not link.pode_apagar:
        return redirect('ver_link', token=token)
    artigo = get_object_or_404(Artigo, pk=pk, lista=link.lista)
    if request.method == 'POST':
        artigo.delete()
    return redirect('ver_link', token=token)


def link_quantidade(request, token, pk, direcao):
    """Update quantity via shared link."""
    link = _get_link_or_404(token)
    if not link.pode_editar:
        return redirect('ver_link', token=token)
    artigo = get_object_or_404(Artigo, pk=pk, lista=link.lista)
    if request.method == 'POST':
        match = re.match(r'^(\d+)', artigo.quantidade)
        num = int(match.group(1)) if match else 1
        if direcao == 'mais':
            num += 1
        elif direcao == 'menos' and num > 1:
            num -= 1
        artigo.quantidade = f"{num}"
        artigo.save()
    return redirect('ver_link', token=token)


def link_check_updates(request, token):
    """Poll for updates on a link page."""
    link = _get_link_or_404(token)
    qs = link.lista.artigos.all()
    ultimo = qs.order_by('-movido_em').values_list('movido_em', flat=True).first()
    ts = ultimo.isoformat() if ultimo else ''
    count = qs.count()
    return JsonResponse({'ts': ts, 'count': count})
