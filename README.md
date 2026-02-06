# 🛒 Let's Go Shopping

Aplicação web de lista de compras desenvolvida em **Python Django** com base de dados **MySQL**.

---

## Descrição

"Let's Go Shopping" é uma aplicação de gestão de lista de compras com duas secções:

- **📦 Despensa** — Artigos disponíveis em casa (lista com scroll)
- **🛍️ Artigos a comprar** — Artigos marcados para compra

### Funcionalidades

- **Adicionar** artigos com nome e quantidade (predefinido: `1x`)
- **Editar** nome e quantidade de qualquer artigo
- **Apagar** artigos com confirmação
- **Checkbox** — Marcar um artigo na despensa move-o para "Artigos a comprar"; desmarcar devolve-o à despensa
- Suporte a **caracteres especiais** e textos longos
- Interface **dark mode**, moderna e otimizada para **smartphone**

---

## Requisitos

- Python 3.13
- MySQL Server (acessível em `192.168.122.45:3307`)
- Dependências Python listadas em `requirements.txt`

---

## Estrutura do Projeto

```
lista_compras/
├── compras/                        # App Django principal
│   ├── migrations/                 # Migrações da base de dados
│   ├── templates/compras/
│   │   └── index.html              # Template principal (UI)
│   ├── models.py                   # Modelo Artigo
│   ├── views.py                    # Views (CRUD + toggle)
│   ├── urls.py                     # Rotas da app
│   └── admin.py
├── lista_compras/                  # Configuração do projeto Django
│   ├── settings.py                 # Configurações (BD, apps, idioma)
│   ├── urls.py                     # Rotas raiz
│   ├── wsgi.py
│   └── asgi.py
├── venv.lista/                     # Ambiente virtual Python
├── manage.py                       # CLI do Django
├── requirements.txt                # Dependências
└── README.md                       # Esta documentação
```

---

## Instalação e Configuração

### 1. Clonar/aceder ao projeto

```bash
cd lista_compras/
```

### 2. Criar ambiente virtual

```bash
python3.13 -m venv venv.lista
```

### 3. Ativar ambiente virtual

**Bash/Zsh:**
```bash
source venv.lista/bin/activate
```

**Fish:**
```fish
source venv.lista/bin/activate.fish
```

### 4. Instalar dependências

```bash
pip install -r requirements.txt
```

### 5. Configurar a base de dados MySQL

Certifique-se de que o servidor MySQL está acessível e crie a base de dados:

```sql
CREATE DATABASE lista_compras CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

Configuração em `lista_compras/settings.py`:

```python
DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.mysql',
        'NAME': 'lista_compras',
        'USER': 'root',
        'PASSWORD': '1234',
        'HOST': '192.168.122.45',
        'PORT': '3307',
        'OPTIONS': {
            'init_command': "SET sql_mode='STRICT_TRANS_TABLES'",
        }
    }
}
```

### 6. Aplicar migrações

```bash
python manage.py makemigrations compras
python manage.py migrate
```

### 7. Iniciar o servidor

```bash
python manage.py runserver 0.0.0.0:8000
```

Aceder em: **http://localhost:8000** ou **http://<IP_da_máquina>:8000** no smartphone.

---

## Modelo de Dados

### Artigo

| Campo       | Tipo          | Descrição                                      |
|-------------|---------------|------------------------------------------------|
| `id`        | BigAutoField  | Chave primária (automático)                    |
| `nome`      | CharField(500)| Nome do artigo                                 |
| `quantidade`| CharField(50) | Quantidade (predefinido: `1x`)                 |
| `comprar`   | BooleanField  | `False` = despensa, `True` = lista de compras  |
| `criado_em` | DateTimeField | Data/hora de criação (automático)              |

---

## Rotas (URLs)

| URL                  | Método | Descrição                          |
|----------------------|--------|------------------------------------|
| `/`                  | GET    | Página principal                   |
| `/adicionar/`        | POST   | Adicionar novo artigo              |
| `/editar/<id>/`      | POST   | Editar artigo existente            |
| `/apagar/<id>/`      | POST   | Apagar artigo                      |
| `/toggle/<id>/`      | POST   | Alternar entre despensa e compras  |

---

## Utilização no Smartphone

1. Ligar o smartphone à mesma rede que o servidor
2. Iniciar o servidor com `0.0.0.0:8000`
3. No browser do smartphone, aceder a `http://<IP_do_servidor>:8000`

A interface está otimizada para ecrãs pequenos:
- Formulário de adição com campo de quantidade à direita
- Botões de ação com tamanho adequado para toque
- Lista da despensa com scroll vertical
- Modais de edição e eliminação adaptados a mobile

---

## Tecnologias

- **Backend:** Django 4.2.28
- **Base de dados:** MySQL (mysqlclient 2.2.7)
- **Frontend:** HTML5, CSS3 (dark theme inline), JavaScript vanilla
- **Python:** 3.13
- **Pillow:** 12.1.0

---

## Notas

- O campo quantidade aceita texto livre (ex: `2x`, `500g`, `1L`, `3 unidades`)
- Nomes de artigos suportam caracteres especiais, acentos e emojis
- A ordenação dos artigos é alfabética por nome
- `ALLOWED_HOSTS = ['*']` está configurado para desenvolvimento — restringir em produção
