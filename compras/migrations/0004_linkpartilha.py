import django.db.models.deletion
import uuid
from django.db import migrations, models


class Migration(migrations.Migration):

    dependencies = [
        ('compras', '0003_lista_artigo_lista_listapartilha'),
    ]

    operations = [
        migrations.CreateModel(
            name='LinkPartilha',
            fields=[
                ('id', models.BigAutoField(auto_created=True, primary_key=True, serialize=False, verbose_name='ID')),
                ('token', models.UUIDField(default=uuid.uuid4, editable=False, unique=True)),
                ('criado_em', models.DateTimeField(auto_now_add=True)),
                ('expira_em', models.DateTimeField()),
                ('pode_adicionar', models.BooleanField(default=False)),
                ('pode_editar', models.BooleanField(default=False)),
                ('pode_apagar', models.BooleanField(default=False)),
                ('pode_toggle', models.BooleanField(default=False)),
                ('lista', models.ForeignKey(on_delete=django.db.models.deletion.CASCADE, related_name='links_partilha', to='compras.lista')),
            ],
            options={
                'ordering': ['-criado_em'],
            },
        ),
    ]
