from django.urls import path
from . import views

urlpatterns = [
    path('', views.index, name='index'),
    path('adicionar/', views.adicionar, name='adicionar'),
    path('editar/<int:pk>/', views.editar, name='editar'),
    path('apagar/<int:pk>/', views.apagar, name='apagar'),
    path('toggle/<int:pk>/', views.toggle, name='toggle'),
    path('quantidade/<int:pk>/<str:direcao>/', views.quantidade_update, name='quantidade_update'),
    path('check_updates/', views.check_updates, name='check_updates'),
]
