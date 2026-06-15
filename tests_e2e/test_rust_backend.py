"""End-to-end Playwright tests for Rust (Axum) backend."""
import os
import re
import uuid

import pytest
from playwright.sync_api import Page, expect, sync_playwright

BASE = os.environ.get("RUST_BASE", "http://127.0.0.1:8766")


def _u():
    return f"r{uuid.uuid4().hex[:10]}"


@pytest.fixture(scope="session")
def browser():
    with sync_playwright() as p:
        b = p.chromium.launch(headless=True)
        yield b
        b.close()


@pytest.fixture
def page(browser):
    ctx = browser.new_context(base_url=BASE)
    pg = ctx.new_page()
    yield pg
    ctx.close()


def register(page: Page, username: str, password: str = "Sup3rPa$$2026"):
    page.goto("/registar/")
    page.fill('input[name="username"]', username)
    page.fill('input[name="password"]', password)
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")


def add_email_if_needed(page: Page, email: str):
    if "adicionar_email" in page.url or "adicionar-email" in page.url:
        page.fill('input[name="email"]', email)
        page.click('button[type="submit"]')
        page.wait_for_load_state("networkidle")


def test_healthz(page: Page):
    resp = page.request.get("/healthz")
    assert resp.status == 200
    assert resp.text() == "ok"


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


def test_register_then_redirect(page: Page):
    user = _u()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    expect(page.locator(".active-list-name")).to_be_visible()


def test_register_duplicate_user_shows_error(page: Page):
    user = _u()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    page.goto("/sair/")
    page.goto("/registar/")
    page.fill('input[name="username"]', user)
    page.fill('input[name="password"]', "Sup3rPa$$2026")
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".erro").first).to_contain_text("já existe")


def test_logout(page: Page):
    user = _u()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")
    page.goto("/sair/")
    page.wait_for_load_state("networkidle")
    assert "/entrar" in page.url


def test_full_list_workflow(page: Page):
    user = _u()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")

    page.fill('#addInput', "Pão")
    page.click('#addBtn')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".item", has_text="Pão").first).to_be_visible()

    page.fill('#addInput', "Leite")
    page.click('#addBtn')
    page.wait_for_load_state("networkidle")
    expect(page.locator(".item", has_text="Leite").first).to_be_visible()

    pao = page.locator(".item", has_text="Pão").first
    pao.locator(".item-name").click()
    page.wait_for_load_state("networkidle")
    expect(page.locator(".item", has_text="Pão").first).to_be_visible()


def test_create_list(page: Page):
    user = _u()
    register(page, user)
    add_email_if_needed(page, f"{user}@test.local")

    nova = f"Lista_{uuid.uuid4().hex[:6]}"
    page.evaluate(f"""
        fetch('/lista/criar', {{
            method: 'POST',
            headers: {{ 'Content-Type': 'application/x-www-form-urlencoded' }},
            body: 'nome={nova}',
            credentials: 'include'
        }}).then(r => r.text())
    """)
    page.reload()
    page.wait_for_load_state("networkidle")
    expect(page.locator(".active-list-name")).to_contain_text(nova)


def test_recover_submit_unknown_email(page: Page):
    page.goto("/recuperar-password/")
    page.fill('input[name="email"]', "nobody_xyz@example.invalid")
    page.click('button[type="submit"]')
    page.wait_for_load_state("networkidle")
    body = page.content()
    assert "enviado" in body.lower() or "email" in body.lower()


def test_api_me_unauthenticated(page: Page):
    resp = page.request.get("/api/me")
    assert resp.status in (200, 401, 403)


def test_api_register_login_flow(page: Page):
    user = _u()
    pwd = "Sup3rPa$$2026"
    r = page.request.post(
        "/api/register",
        data={"username": user, "password": pwd},
        headers={"Content-Type": "application/json"},
    )
    assert r.status in (200, 201), r.text()
    me = r.json()
    assert me["username"] == user

    page.request.post("/api/logout")

    r2 = page.request.post(
        "/api/login",
        data={"username": user, "password": pwd},
        headers={"Content-Type": "application/json"},
    )
    assert r2.status == 200, r2.text()
    me2 = r2.json()
    assert me2["username"] == user


def test_api_login_bad_password(page: Page):
    r = page.request.post(
        "/api/login",
        data={"username": "nope_xyz", "password": "wrong"},
        headers={"Content-Type": "application/json"},
    )
    assert r.status == 401
    body = r.json()
    assert "incorretos" in body.get("error", "").lower()
