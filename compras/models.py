import uuid
from django.db import models
from django.conf import settings
from django.utils import timezone


class Lista(models.Model):
    nome = models.CharField(max_length=200)
    dono = models.ForeignKey(
        settings.AUTH_USER_MODEL,
        on_delete=models.CASCADE,
        related_name='listas_proprias',
    )
    criado_em = models.DateTimeField(auto_now_add=True)

    class Meta:
        ordering = ['nome']
        unique_together = ('nome', 'dono')

    def __str__(self):
        return self.nome

    def utilizadores_partilha(self):
        return self.partilhas.select_related('utilizador')


class ListaPartilha(models.Model):
    lista = models.ForeignKey(
        Lista,
        on_delete=models.CASCADE,
        related_name='partilhas',
    )
    utilizador = models.ForeignKey(
        settings.AUTH_USER_MODEL,
        on_delete=models.CASCADE,
        related_name='listas_partilhadas',
    )
    criado_em = models.DateTimeField(auto_now_add=True)

    class Meta:
        unique_together = ('lista', 'utilizador')

    def __str__(self):
        return f"{self.lista.nome} → {self.utilizador.username}"


class LinkPartilha(models.Model):
    lista = models.ForeignKey(
        Lista,
        on_delete=models.CASCADE,
        related_name='links_partilha',
    )
    token = models.UUIDField(default=uuid.uuid4, unique=True, editable=False)
    criado_em = models.DateTimeField(auto_now_add=True)
    expira_em = models.DateTimeField()
    pode_adicionar = models.BooleanField(default=False)
    pode_editar = models.BooleanField(default=False)
    pode_apagar = models.BooleanField(default=False)
    pode_toggle = models.BooleanField(default=False)

    class Meta:
        ordering = ['-criado_em']

    def __str__(self):
        return f"Link: {self.lista.nome} ({self.token})"

    @property
    def esta_ativo(self):
        return timezone.now() < self.expira_em


class Artigo(models.Model):
    lista = models.ForeignKey(
        Lista,
        on_delete=models.CASCADE,
        related_name='artigos',
        null=True,
        blank=True,
    )
    nome = models.CharField(max_length=500)
    quantidade = models.CharField(max_length=50, default='1x')
    comprar = models.BooleanField(default=False)
    criado_em = models.DateTimeField(auto_now_add=True)
    movido_em = models.DateTimeField(auto_now=True)

    class Meta:
        ordering = ['-movido_em']

    def __str__(self):
        return f"{self.quantidade} {self.nome}"
