import re
from django.shortcuts import render, redirect, get_object_or_404
from django.http import JsonResponse
from django.contrib.auth import login, logout, get_user_model
from django.contrib.auth.decorators import login_required
from django.contrib.auth.hashers import make_password
from django.db.models import Q, Count
from .models import Artigo, Lista, ListaPartilha
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

def registar(request):
    if request.user.is_authenticated:
        return redirect('index')
    erro = ''
    if request.method == 'POST':
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
            return redirect('index')
    return render(request, 'compras/registar.html', {'erro': erro})


def entrar(request):
    if request.user.is_authenticated:
        return redirect('index')
    erro = ''
    if request.method == 'POST':
        username = request.POST.get('username', '').strip()
        password_hash = request.POST.get('password_hash', '').strip()
        backend = HashedPasswordBackend()
        user = backend.authenticate(request, username=username, password_hash=password_hash)
        if user:
            login(request, user, backend='compras.backends.HashedPasswordBackend')
            return redirect('index')
        else:
            erro = 'Nome de utilizador ou palavra-passe incorretos.'
    return render(request, 'compras/entrar.html', {'erro': erro})


def sair(request):
    logout(request)
    return redirect('entrar')


# ── Main index ───────────────────────────────────────────────────

@login_required
def index(request):
    lista = _lista_ativa(request)
    listas = _listas_do_utilizador(request.user).annotate(
        n_partilhas=Count('partilhas')
    )
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
    return render(request, 'compras/index.html', {
        'despensa': despensa,
        'a_comprar': a_comprar,
        'lista_ativa': lista,
        'listas': listas,
        'partilhas': partilhas,
        'e_dono': e_dono,
    })


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
    return JsonResponse({'ts': ts, 'count': count})


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
