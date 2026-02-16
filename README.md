# ListaIsto.

A collaborative shopping list web application built with **Django** and **MySQL**, containerized with **Docker Compose** and served via **Caddy** with HTTPS support.

---

## Description

**ListaIsto.** is a shopping list management app with two sections per list:

- **Pantry** — Items available at home
- **To Buy** — Items marked for purchase

### Features

- **Multiple lists** — Create, rename, delete, and switch between lists
- **Clone lists** — Duplicate any list (including all items) with one click
- **List sharing** — Share lists with other registered users for real-time collaboration
- **Link sharing** — Generate temporary public links with granular permissions (add, edit, delete, toggle)
- **Shared list popup** — When logging in via a shared link, a popup asks if you want to add the list to your collection
- **Add items** with name and quantity (default: `1x`)
- **Edit** name and quantity of any item
- **Delete** items with confirmation
- **Checkbox** — Checking a pantry item moves it to "To Buy"; unchecking returns it to the pantry
- **Real-time updates** — Automatic polling for changes made by other users
- **User authentication** — Registration, login, and profile management
- Support for **special characters** and long text
- **Dark mode** UI, modern and optimized for **mobile devices**
- **Custom favicon** with SVG, PNG, and ICO formats for sharp display across all browsers

---

## Requirements

- Docker & Docker Compose
- A `.env` file with database credentials and Django secret key

---

## Project Structure

```
lista_compras/
├── compras/                        # Main Django app
│   ├── migrations/                 # Database migrations
│   ├── templates/compras/
│   │   ├── index.html              # Main template (authenticated UI)
│   │   ├── entrar.html             # Login page
│   │   ├── registar.html           # Registration page
│   │   └── link.html               # Public shared link view
│   ├── static/compras/             # Static files (logo, favicons)
│   ├── models.py                   # Models (Lista, Artigo, ListaPartilha, LinkPartilha)
│   ├── views.py                    # Views (CRUD, sharing, auth, clone)
│   ├── urls.py                     # App routes
│   ├── backends.py                 # Custom auth backend
│   └── admin.py
├── lista_compras/                  # Django project settings
│   ├── settings.py                 # Settings (DB, apps, middleware)
│   ├── urls.py                     # Root URL config
│   ├── wsgi.py
│   └── asgi.py
├── Dockerfile                      # Web service container
├── Caddyfile                       # Caddy reverse proxy config
├── docker-compose.yml              # Multi-service orchestration
├── requirements.txt                # Python dependencies
└── README.md                       # This documentation
```

---

## Setup & Installation

### 1. Clone the repository

```bash
git clone <repo-url>
cd lista_compras/
```

### 2. Create a `.env` file

```env
SECRET_KEY=your-django-secret-key
DB_NAME=lista_compras
DB_USER=root
DB_PASSWORD=your-db-password
DB_HOST=db
DB_PORT=3306
```

### 3. Start the application

```bash
sudo docker compose up -d --build
```

This starts three services:
- **db** — MySQL database
- **web** — Django app served by Gunicorn + WhiteNoise for static files
- **caddy** — Reverse proxy with automatic HTTPS

### 4. Access the app

- **Local:** http://localhost
- **Production:** https://your-domain.com (configured in `Caddyfile`)

---

## Data Models

### Lista (List)

| Field       | Type           | Description                        |
|-------------|----------------|------------------------------------|
| `id`        | BigAutoField   | Primary key (auto)                 |
| `nome`      | CharField(200) | List name                          |
| `dono`      | ForeignKey     | Owner (User)                       |
| `criado_em` | DateTimeField  | Creation date (auto)               |

### Artigo (Item)

| Field       | Type           | Description                                  |
|-------------|----------------|----------------------------------------------|
| `id`        | BigAutoField   | Primary key (auto)                           |
| `lista`     | ForeignKey     | Parent list                                  |
| `nome`      | CharField(500) | Item name                                    |
| `quantidade`| CharField(50)  | Quantity (default: `1x`)                     |
| `comprar`   | BooleanField   | `False` = pantry, `True` = to buy            |
| `criado_em` | DateTimeField  | Creation date (auto)                         |
| `movido_em` | DateTimeField  | Last moved date (auto)                       |

### ListaPartilha (List Share)

| Field        | Type          | Description                        |
|--------------|---------------|------------------------------------|
| `lista`      | ForeignKey    | Shared list                        |
| `utilizador` | ForeignKey    | User with access                   |
| `criado_em`  | DateTimeField | Share date (auto)                  |

### LinkPartilha (Share Link)

| Field           | Type          | Description                        |
|-----------------|---------------|------------------------------------|
| `lista`         | ForeignKey    | Linked list                        |
| `token`         | UUIDField     | Unique public token                |
| `expira_em`     | DateTimeField | Expiration date                    |
| `pode_adicionar`| BooleanField  | Can add items                      |
| `pode_editar`   | BooleanField  | Can edit items                     |
| `pode_apagar`   | BooleanField  | Can delete items                   |
| `pode_toggle`   | BooleanField  | Can toggle items                   |

---

## Routes (URLs)

| URL                                      | Method | Description                        |
|------------------------------------------|--------|------------------------------------|
| `/`                                      | GET    | Main page                          |
| `/registar/`                             | POST   | Register new user                  |
| `/entrar/`                               | POST   | Login                              |
| `/sair/`                                 | POST   | Logout                             |
| `/lista/criar/`                          | POST   | Create new list                    |
| `/lista/<id>/selecionar/`               | GET    | Switch active list                 |
| `/lista/<id>/renomear/`                 | POST   | Rename list                        |
| `/lista/<id>/apagar/`                   | POST   | Delete list                        |
| `/lista/<id>/clonar/`                   | POST   | Clone list with all items          |
| `/lista/<id>/partilhar/`               | POST   | Share list with a user             |
| `/lista/<id>/link/criar/`              | POST   | Create a public share link         |
| `/responder-link/`                       | POST   | Accept/reject shared list popup    |
| `/adicionar/`                            | POST   | Add item                           |
| `/editar/<id>/`                          | POST   | Edit item                          |
| `/apagar/<id>/`                          | POST   | Delete item                        |
| `/toggle/<id>/`                          | POST   | Toggle item between pantry/to-buy  |
| `/link/<token>/`                         | GET    | View list via public link          |

---

## Tech Stack

- **Backend:** Django 4.2, Gunicorn, WhiteNoise
- **Database:** MySQL (mysqlclient)
- **Frontend:** HTML5, CSS3 (dark theme), vanilla JavaScript
- **Containerization:** Docker, Docker Compose
- **Reverse Proxy:** Caddy (automatic HTTPS)
- **Python:** 3.13

---

## Notes

- The quantity field accepts free text (e.g., `2x`, `500g`, `1L`, `3 units`)
- Item names support special characters, accents, and emojis
- Lists are sorted by most recent first in the hamburger menu
- Shared lists display a people icon next to the list name
- Public share links can be configured with an expiration date and granular permissions
- The clone feature creates an independent copy — changes to the clone do not affect the original
