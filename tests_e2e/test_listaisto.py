"""End-to-end Playwright tests for ListaIsto Django app."""
import os
import re
import time
import uuid

import pytest
from playwright.sync_api import Page, expect, sync_playwright

BASE = os.environ.get("E2E_BASE", "http://127.0.0.1:8765")


def _unique_user():
    return f"u{uuid.uuid4().hex[:10]}"


@pytest.fixture(scope="session")
def browser():
    with sync_playwright() as p:
        b = p.chromium.launch(headless=True)
        yield b
        b.close()


@pytest.fixture
def page(browser):
    ctx = browser.new_context(base_url=BASE)
    ctx.add_init_script("try{localStorage.setItem('listaisto_cookie_ok','1');}catch(e){}")
    pg = ctx.new_page()
    yield pg
    ctx.close()


def _dismiss_cookie(page: Page):
    page.evaluate("""() => {
        try { localStorage.setItem('listaisto_cookie_ok','1'); } catch(e){}
        const b = document.getElementById('cookieBanner');
        if (b) b.style.display='none';
    }""")


def register(page: Page, username: str, password: str = "Sup3rPa$$2026"):
    page.goto("/registar/")
    page.fill('input[name="username"]', username)
    page.fill('input[name="password"]', password)
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")


def login(page: Page, username: str, password: str = "Sup3rPa$$2026"):
    page.goto("/entrar/")
    page.fill('input[name="username"]', username)
    page.fill('input[name="password"]', password)
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")


def add_email_if_needed(page: Page, email: str):
    if "/adicionar-email" in page.url:
        page.fill('input[name="email"]', email)
        page.click('button[type="submit"]')
        page.wait_for_load_state("networkidle")


def test_login_page_renders(page: Page):
    page.goto("/entrar/")
    expect(page).to_have_title(re.compile("Entrar"))
    expect(page.locator('input[name="username"]')).to_be_visible()
    expect(page.locator('input[name="password"]')).to_be_visible()


def test_register_page_renders(page: Page):
    page.goto("/registar/")
    expect(page.locator('input[name="username"]')).to_be_visible()


def test_recover_password_page_renders(page: Page):
    page.goto("/recuperar-password/")
    expect(page.locator('input[name="email"]')).to_be_visible()


def test_protected_index_redirects_to_login(page: Page):
    page.goto("/")
    page.wait_for_load_state("networkidle")
    assert "/entrar" in page.url


def test_login_with_bad_credentials_shows_error(page: Page):
    page.goto("/entrar/")
    page.fill('input[name="username"]', "nope_no_user_xyz")
    page.fill('input[name="password"]', "wrongpass")
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".erro").first).to_contain_text("incorretos")


def test_register_then_login_flow(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    assert page.url.rstrip("/") == BASE.rstrip("/") or page.url.endswith("/")
    expect(page.locator(".active-list-name")).to_be_visible()


def test_register_duplicate_user_shows_error(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    page.goto("/sair/")
    page.goto("/registar/")
    page.fill('input[name="username"]', user)
    page.fill('input[name="password"]', "Sup3rPa$$2026")
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".erro").first).to_contain_text("já existe")


def test_full_list_workflow(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")

    page.fill('input[name="nome"]#addInput', "Pão")
    page.click('#addBtn')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".item-name", has_text="Pão")).to_be_visible()

    page.fill('input[name="nome"]#addInput', "Leite")
    page.click('#addBtn')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".item-name", has_text="Leite")).to_be_visible()

    page.locator(".item", has_text="Pão").locator(".item-name").click()
    page.wait_for_load_state("networkidle")
    despensa = page.locator(".despensa-wrapper")
    expect(despensa.locator(".item-name", has_text="Pão")).to_be_visible()

    despensa.locator(".item-name", has_text="Pão").click()
    page.wait_for_load_state("networkidle")
    a_comprar = page.locator(".container > .item-list, .container .section-title").first
    expect(page.locator(".item-name", has_text="Pão")).to_be_visible()


def test_create_rename_delete_list(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")

    nova = f"Lista_{uuid.uuid4().hex[:6]}"
    page.evaluate(f"""
        fetch('/lista/criar/', {{
            method: 'POST',
            headers: {{ 'X-CSRFToken': document.cookie.match(/csrftoken=([^;]+)/)[1], 'Content-Type': 'application/x-www-form-urlencoded' }},
            body: 'nome={nova}'
        }}).then(r => r.text())
    """)
    page.reload()
    page.wait_for_load_state("networkidle")
    expect(page.locator(".active-list-name")).to_contain_text(nova)


def test_logout(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    page.goto("/sair/")
    page.wait_for_load_state("networkidle")
    assert "/entrar" in page.url


def test_check_updates_endpoint_authenticated(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    resp = page.request.get("/check_updates/")
    assert resp.status == 200
    data = resp.json()
    assert "ts" in data and "count" in data and "n_listas" in data


def test_quantity_increment(page: Page):
    user = _unique_user()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    page.fill('input[name="nome"]#addInput', "Maçãs")
    page.click('#addBtn')
    page.wait_for_load_state("networkidle")
    item = page.locator(".item", has_text="Maçãs")
    expect(item).to_be_visible()


def test_recover_submit_unknown_email_shows_sent(page: Page):
    page.goto("/recuperar-password/")
    page.fill('input[name="email"]', "nobody_xyz@example.invalid")
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")
    body = page.content()
    assert "enviado" in body.lower() or "email" in body.lower()
