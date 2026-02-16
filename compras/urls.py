from django.urls import path
from . import views

urlpatterns = [
    path('', views.index, name='index'),
    path('registar/', views.registar, name='registar'),
    path('entrar/', views.entrar, name='entrar'),
    path('sair/', views.sair, name='sair'),
    path('lista/criar/', views.criar_lista, name='criar_lista'),
    path('lista/<int:pk>/selecionar/', views.selecionar_lista, name='selecionar_lista'),
    path('lista/<int:pk>/renomear/', views.renomear_lista, name='renomear_lista'),
    path('lista/<int:pk>/apagar/', views.apagar_lista, name='apagar_lista'),
    path('lista/<int:pk>/partilhar/', views.partilhar_lista, name='partilhar_lista'),
    path('lista/<int:pk>/partilha/<int:user_pk>/remover/', views.remover_partilha, name='remover_partilha'),
    path('lista/<int:pk>/link/criar/', views.criar_link_partilha, name='criar_link_partilha'),
    path('lista/<int:pk>/link/<int:link_pk>/apagar/', views.apagar_link_partilha, name='apagar_link_partilha'),
    path('lista/<int:pk>/links/', views.listar_links_partilha, name='listar_links_partilha'),
    path('adicionar/', views.adicionar, name='adicionar'),
    path('editar/<int:pk>/', views.editar, name='editar'),
    path('apagar/<int:pk>/', views.apagar, name='apagar'),
    path('toggle/<int:pk>/', views.toggle, name='toggle'),
    path('quantidade/<int:pk>/<str:direcao>/', views.quantidade_update, name='quantidade_update'),
    path('check_updates/', views.check_updates, name='check_updates'),
    # Public link routes
    path('link/<uuid:token>/', views.ver_link, name='ver_link'),
    path('link/<uuid:token>/adicionar/', views.link_adicionar, name='link_adicionar'),
    path('link/<uuid:token>/toggle/<int:pk>/', views.link_toggle, name='link_toggle'),
    path('link/<uuid:token>/editar/<int:pk>/', views.link_editar, name='link_editar'),
    path('link/<uuid:token>/apagar/<int:pk>/', views.link_apagar, name='link_apagar'),
    path('link/<uuid:token>/quantidade/<int:pk>/<str:direcao>/', views.link_quantidade, name='link_quantidade'),
    path('link/<uuid:token>/check_updates/', views.link_check_updates, name='link_check_updates'),
]
