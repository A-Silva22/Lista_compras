from django.db import models


class Artigo(models.Model):
    nome = models.CharField(max_length=500)
    quantidade = models.CharField(max_length=50, default='1x')
    comprar = models.BooleanField(default=False)
    criado_em = models.DateTimeField(auto_now_add=True)
    movido_em = models.DateTimeField(auto_now=True)

    class Meta:
        ordering = ['-movido_em']

    def __str__(self):
        return f"{self.quantidade} {self.nome}"
