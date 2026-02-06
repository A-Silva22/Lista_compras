from django.shortcuts import render, redirect, get_object_or_404
from django.http import JsonResponse
from .models import Artigo


def index(request):
    despensa = Artigo.objects.filter(comprar=False)
    a_comprar = Artigo.objects.filter(comprar=True)
    return render(request, 'compras/index.html', {
        'despensa': despensa,
        'a_comprar': a_comprar,
    })


def adicionar(request):
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            if not quantidade:
                quantidade = '1'
            Artigo.objects.create(nome=nome, quantidade=quantidade, comprar=True)
    return redirect('index')


def editar(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if request.method == 'POST':
        nome = request.POST.get('nome', '').strip()
        quantidade = request.POST.get('quantidade', '1').strip()
        if nome:
            artigo.nome = nome
            artigo.quantidade = quantidade if quantidade else '1'
            artigo.save()
    return redirect('index')


def apagar(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if request.method == 'POST':
        artigo.delete()
    return redirect('index')


def toggle(request, pk):
    artigo = get_object_or_404(Artigo, pk=pk)
    if request.method == 'POST':
        artigo.comprar = not artigo.comprar
        artigo.save()
    return redirect('index')


def quantidade_update(request, pk, direcao):
    import re
    artigo = get_object_or_404(Artigo, pk=pk)
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
