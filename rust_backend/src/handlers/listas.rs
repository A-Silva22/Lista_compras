use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use chrono::Duration;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{auth, AppState};

const ACTIVE_KEY: &str = "lista_ativa";

async fn require_user(session: &Session) -> Result<auth::CurrentUser, Response> {
    auth::current(session).await.ok_or_else(|| Redirect::to("/entrar/").into_response())
}

pub async fn active_lista_id(session: &Session) -> Option<i32> {
    session.get::<i32>(ACTIVE_KEY).await.ok().flatten()
}

pub async fn set_active(session: &Session, lista_id: i32) {
    let _ = session.insert(ACTIVE_KEY, lista_id).await;
}

pub async fn clear_active(session: &Session) {
    let _ = session.remove::<i32>(ACTIVE_KEY).await;
}

#[derive(Deserialize)]
pub struct CriarForm { pub nome: String }

pub async fn criar(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CriarForm>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let nome = form.nome.trim();
    if nome.is_empty() { return Redirect::to("/home").into_response(); }
    match state.db.create_lista(nome, user.id).await {
        Ok(id) => set_active(&session, id).await,
        Err(e) => tracing::error!("create_lista: {:?}", e),
    }
    Redirect::to("/home").into_response()
}

pub async fn selecionar(
    State(state): State<AppState>,
    session: Session,
    Path(lista_id): Path<i32>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if let Ok(Some(_)) = state.db.get_accessible_lista(lista_id, user.id).await {
        set_active(&session, lista_id).await;
    }
    Redirect::to("/home").into_response()
}

pub async fn apagar(
    State(state): State<AppState>,
    session: Session,
    Path(lista_id): Path<i32>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if let Err(e) = state.db.delete_lista(lista_id, user.id).await {
        tracing::warn!("delete_lista: {:?}", e);
    }
    if active_lista_id(&session).await == Some(lista_id) {
        clear_active(&session).await;
    }
    Redirect::to("/home").into_response()
}

#[derive(Deserialize)]
pub struct AdicionarArtigoForm {
    pub nome: String,
    #[serde(default)]
    pub quantidade: String,
}

pub async fn adicionar_artigo(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<AdicionarArtigoForm>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let nome = form.nome.trim();
    if nome.is_empty() { return Redirect::to("/home").into_response(); }
    let q = form.quantidade.trim();
    let qty = if q.is_empty() { "1" } else { q };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Redirect::to("/home").into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    // Avoid duplicates: if an article with the same name (case-insensitive)
    // already exists, move it back to "comprar" instead of inserting a copy.
    match state.db.match_artigo_in_list(lista_id, nome).await {
        Ok(Some(existing)) => {
            if let Err(e) = state.db.toggle_artigo(existing.id, lista_id, Some("comprar")).await {
                tracing::warn!("toggle existing artigo: {:?}", e);
            }
        }
        Ok(None) => {
            if let Err(e) = state.db.add_artigo(lista_id, nome, qty).await {
                tracing::error!("add_artigo: {:?}", e);
            }
        }
        Err(e) => tracing::error!("match_artigo_in_list: {:?}", e),
    }
    Redirect::to("/home").into_response()
}

#[derive(Deserialize)]
pub struct ToggleForm {
    #[serde(default)]
    pub destino: String,
}

#[derive(Serialize)]
pub struct ToggleResponse {
    pub ok: bool,
    pub comprar: bool,
}

pub async fn toggle_artigo(
    State(state): State<AppState>,
    session: Session,
    Path(artigo_id): Path<i32>,
    headers: axum::http::HeaderMap,
    Form(form): Form<ToggleForm>,
) -> Response {
    // A browser fetch sets `X-Requested-With: XMLHttpRequest` and gets JSON
    // back (so the item can move in-place); a plain form post still redirects.
    let is_xhr = headers
        .get("x-requested-with")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("XMLHttpRequest"))
        .unwrap_or(false);

    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Redirect::to("/home").into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    let destino = match form.destino.as_str() {
        "" => None,
        s => Some(s),
    };
    let comprar = match state.db.toggle_artigo(artigo_id, lista_id, destino).await {
        Ok(v) => v.unwrap_or(false),
        Err(e) => {
            tracing::warn!("toggle_artigo: {:?}", e);
            false
        }
    };
    if is_xhr {
        return Json(ToggleResponse { ok: true, comprar }).into_response();
    }
    Redirect::to("/home").into_response()
}

#[derive(Deserialize)]
pub struct EditarForm {
    pub nome: String,
    #[serde(default)]
    pub quantidade: String,
}

pub async fn editar_artigo(
    State(state): State<AppState>,
    session: Session,
    Path(artigo_id): Path<i32>,
    Form(form): Form<EditarForm>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Redirect::to("/home").into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    let nome = form.nome.trim();
    if nome.is_empty() {
        return Redirect::to("/home").into_response();
    }
    let q = form.quantidade.trim();
    let qty = if q.is_empty() { "1" } else { q };
    if let Err(e) = state.db.edit_artigo(artigo_id, lista_id, nome, qty).await {
        tracing::warn!("edit_artigo: {:?}", e);
    }
    Redirect::to("/home").into_response()
}

pub async fn apagar_artigo(
    State(state): State<AppState>,
    session: Session,
    Path(artigo_id): Path<i32>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Redirect::to("/home").into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    if let Err(e) = state.db.delete_artigo(artigo_id, lista_id).await {
        tracing::warn!("delete_artigo: {:?}", e);
    }
    Redirect::to("/home").into_response()
}

pub async fn quantidade(
    State(state): State<AppState>,
    session: Session,
    Path((artigo_id, direcao)): Path<(i32, String)>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Redirect::to("/home").into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    if let Err(e) = state.db.update_quantidade(artigo_id, lista_id, &direcao).await {
        tracing::warn!("update_quantidade: {:?}", e);
    }
    Redirect::to("/home").into_response()
}

// ── Share with another user (POST form on share modal) ────────────────

#[derive(Deserialize)]
pub struct PartilharForm { pub username: String }

#[derive(Serialize)]
pub struct PartilharResponse {
    pub ok: bool,
    pub msg: String,
}

pub async fn partilhar(
    State(state): State<AppState>,
    session: Session,
    Path(lista_id): Path<i32>,
    headers: axum::http::HeaderMap,
    Form(form): Form<PartilharForm>,
) -> Response {
    let is_xhr = headers
        .get("x-requested-with")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("XMLHttpRequest"))
        .unwrap_or(false);
    fn reply(is_xhr: bool, ok: bool, msg: &str) -> Response {
        if is_xhr {
            Json(PartilharResponse { ok, msg: msg.to_owned() }).into_response()
        } else {
            Redirect::to("/home").into_response()
        }
    }

    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return reply(is_xhr, false, "Lista não encontrada.");
    }
    let target = form.username.trim();
    if target.is_empty() {
        return reply(is_xhr, false, "Introduza um nome de utilizador.");
    }
    if target == user.username {
        return reply(is_xhr, false, "Não pode partilhar consigo mesmo.");
    }
    let other = match state.db.user_by_username(target).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return reply(
                is_xhr,
                false,
                &format!("Utilizador «{}» não existe.", target),
            )
        }
        Err(e) => {
            tracing::error!("user_by_username: {:?}", e);
            return reply(is_xhr, false, "Erro interno. Tente de novo.");
        }
    };
    match state.db.add_partilha(lista_id, other.id).await {
        Ok(true) => reply(is_xhr, true, &format!("Lista partilhada com «{}».", target)),
        Ok(false) => reply(is_xhr, true, &format!("Já estava partilhada com «{}».", target)),
        Err(e) => {
            tracing::warn!("add_partilha: {:?}", e);
            reply(is_xhr, false, "Erro ao partilhar. Tente de novo.")
        }
    }
}

pub async fn remover_partilha(
    State(state): State<AppState>,
    session: Session,
    Path((lista_id, user_id)): Path<(i32, i32)>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    if let Err(e) = state.db.remove_partilha(lista_id, user_id).await {
        tracing::warn!("remove_partilha: {:?}", e);
    }
    Redirect::to("/home").into_response()
}

// ── Public link share (LinkPartilha) ──────────────────────────────────

#[derive(Deserialize)]
pub struct CriarLinkForm {
    #[serde(default = "default_duracao")]
    pub duracao: i64,
    #[serde(default = "default_unidade")]
    pub unidade: String,
    #[serde(default)]
    pub pode_adicionar: Option<String>,
    #[serde(default)]
    pub pode_editar: Option<String>,
    #[serde(default)]
    pub pode_apagar: Option<String>,
    #[serde(default)]
    pub pode_toggle: Option<String>,
}
fn default_duracao() -> i64 { 24 }
fn default_unidade() -> String { "horas".into() }

#[derive(Serialize)]
pub struct CriarLinkResponse {
    pub id: i32,
    pub token: String,
    pub url: String,
    pub expira_em_str: String,
}

pub async fn criar_link(
    State(state): State<AppState>,
    session: Session,
    Path(lista_id): Path<i32>,
    headers: axum::http::HeaderMap,
    Form(form): Form<CriarLinkForm>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    let dur = form.duracao.max(1);
    let delta = match form.unidade.as_str() {
        "dias" => Duration::days(dur),
        "minutos" => Duration::minutes(dur),
        _ => Duration::hours(dur),
    };
    let token = random_hex_32();
    let expira_em = chrono::Utc::now() + delta;
    let id = match state.db.create_link(
        lista_id, &token, expira_em,
        form.pode_adicionar.is_some(),
        form.pode_editar.is_some(),
        form.pode_apagar.is_some(),
        form.pode_toggle.is_some(),
    ).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("create_link: {:?}", e);
            return Redirect::to("/home").into_response();
        }
    };

    // Browser fetch with `X-Requested-With: XMLHttpRequest` gets JSON;
    // a plain form post still gets the redirect.
    let is_xhr = headers
        .get("x-requested-with")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("XMLHttpRequest"))
        .unwrap_or(false);
    if is_xhr {
        return Json(CriarLinkResponse {
            id,
            token: token.clone(),
            url: format!("/link/{}", token),
            expira_em_str: expira_em.format("%d/%m/%Y %H:%M").to_string(),
        })
        .into_response();
    }
    Redirect::to("/home").into_response()
}

pub async fn apagar_link(
    State(state): State<AppState>,
    session: Session,
    Path((lista_id, link_id)): Path<(i32, i32)>,
) -> Response {
    let user = match require_user(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Redirect::to("/home").into_response();
    }
    if let Err(e) = state.db.delete_link(link_id, lista_id).await {
        tracing::warn!("delete_link: {:?}", e);
    }
    Redirect::to("/home").into_response()
}

pub fn random_hex_32() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut s = String::with_capacity(32);
    for b in bytes { s.push_str(&format!("{:02x}", b)); }
    s
}

// ── Exact-match check on the active list (duplicate detection) ─────────

#[derive(Serialize)]
pub struct MatchResponse {
    pub found: bool,
    pub id: Option<i32>,
    pub nome: Option<String>,
    pub quantidade: Option<String>,
    pub na_despensa: Option<bool>,
}

pub async fn match_artigo(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Query(qp): axum::extract::Query<SearchQuery>,
) -> Response {
    let user = match auth::current(&session).await {
        Some(u) => u,
        None => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    let Some(lista_id) = active_lista_id(&session).await else {
        return Json(MatchResponse { found: false, id: None, nome: None, quantidade: None, na_despensa: None }).into_response();
    };
    if state.db.get_accessible_lista(lista_id, user.id).await.ok().flatten().is_none() {
        return Json(MatchResponse { found: false, id: None, nome: None, quantidade: None, na_despensa: None }).into_response();
    }
    let q = qp.q.trim();
    if q.is_empty() {
        return Json(MatchResponse { found: false, id: None, nome: None, quantidade: None, na_despensa: None }).into_response();
    }
    match state.db.match_artigo_in_list(lista_id, q).await {
        Ok(Some(a)) => Json(MatchResponse {
            found: true,
            id: Some(a.id),
            nome: Some(a.nome),
            quantidade: Some(a.quantidade),
            na_despensa: Some(a.comprar == 0),
        })
        .into_response(),
        _ => Json(MatchResponse { found: false, id: None, nome: None, quantidade: None, na_despensa: None }).into_response(),
    }
}

// ── Search artigos for autocomplete (JSON, used by add-bar search mode) ──

#[derive(Deserialize)]
pub struct SearchQuery { pub q: String }

#[derive(Serialize)]
pub struct SearchResponse { pub suggestions: Vec<String> }

pub async fn search_artigos(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Query(qp): axum::extract::Query<SearchQuery>,
) -> Response {
    let user = match auth::current(&session).await {
        Some(u) => u,
        None => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    let q = qp.q.trim();
    if q.len() < 2 {
        return Json(SearchResponse { suggestions: vec![] }).into_response();
    }
    match state.db.search_artigos_for_user(user.id, q, 10).await {
        Ok(suggestions) => Json(SearchResponse { suggestions }).into_response(),
        Err(e) => {
            tracing::error!("search_artigos: {:?}", e);
            Json(SearchResponse { suggestions: vec![] }).into_response()
        }
    }
}
