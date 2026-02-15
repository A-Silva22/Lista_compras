from django.urls import path
from . import views

urlpatterns = [
    path('', views.index, name='index'),
    path('registar/', views.registar, name='registar'),
    path('entrar/', views.entrar, name='entrar'),
    path('sair/', views.sair, name='sair'),
    path('lista/criar/', views.criar_lista, name='criar_lista'),
    path('lista/<int:pk>/selecionar/', views.selecionar_lista, name='selecionar_lista'),
    path('lista/<int:pk>/apagar/', views.apagar_lista, name='apagar_lista'),
    path('lista/<int:pk>/partilhar/', views.partilhar_lista, name='partilhar_lista'),
    path('lista/<int:pk>/partilha/<int:user_pk>/remover/', views.remover_partilha, name='remover_partilha'),
    path('adicionar/', views.adicionar, name='adicionar'),
    path('editar/<int:pk>/', views.editar, name='editar'),
    path('apagar/<int:pk>/', views.apagar, name='apagar'),
    path('toggle/<int:pk>/', views.toggle, name='toggle'),
    path('quantidade/<int:pk>/<str:direcao>/', views.quantidade_update, name='quantidade_update'),
    path('check_updates/', views.check_updates, name='check_updates'),
]
