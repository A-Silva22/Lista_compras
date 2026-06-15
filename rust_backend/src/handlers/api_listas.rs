//! JSON API for the React SPA: lists, items, shares, links, and the public
//! share-link view. Session-cookie auth (same `rust_sid` as the auth API).
//! Reuses the existing `Db` methods; mutations return the fresh list detail so
//! the client can update in a single round-trip.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    auth::{self, CurrentUser},
    db::{Artigo, LinkPartilha},
    handlers::listas::{self},
    AppState,
};

// ──────── helpers ────────

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn err(status: StatusCode, msg: &str) -> Response {
    (status, Json(ErrorBody { error: msg.to_owned() })).into_response()
}

async fn require(session: &Session) -> Result<CurrentUser, Response> {
    auth::current(session)
        .await
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "not authenticated"))
}

// ──────── DTOs ────────

#[derive(Serialize)]
struct ItemDto {
    id: i32,
    nome: String,
    quantidade: String,
    comprar: bool,
}
impl From<Artigo> for ItemDto {
    fn from(a: Artigo) -> Self {
        ItemDto { id: a.id, nome: a.nome, quantidade: a.quantidade, comprar: a.comprar != 0 }
    }
}

#[derive(Serialize)]
struct ListSummary {
    id: i32,
    nome: String,
    /// Shared with a user account (I shared it, or it was shared TO me).
    shared_users: bool,
    /// Carries at least one active share-link.
    shared_link: bool,
}

#[derive(Serialize)]
struct ListsResponse {
    lists: Vec<ListSummary>,
    active_id: i32,
}

#[derive(Serialize)]
struct PartilhaDto {
    utilizador_id: i32,
    username: String,
}

#[derive(Serialize)]
struct LinkDto {
    id: i32,
    token: String,
    url: String,
    expira_em_str: String,
    pode_adicionar: bool,
    pode_editar: bool,
    pode_apagar: bool,
    pode_toggle: bool,
}
impl From<LinkPartilha> for LinkDto {
    fn from(l: LinkPartilha) -> Self {
        LinkDto {
            id: l.id,
            url: format!("/link/{}", l.token),
            token: l.token,
            expira_em_str: l.expira_em.format("%d/%m/%Y %H:%M").to_string(),
            pode_adicionar: l.pode_adicionar != 0,
            pode_editar: l.pode_editar != 0,
            pode_apagar: l.pode_apagar != 0,
            pode_toggle: l.pode_toggle != 0,
        }
    }
}

#[derive(Serialize)]
struct ListDetail {
    id: i32,
    nome: String,
    is_owner: bool,
    a_comprar: Vec<ItemDto>,
    despensa: Vec<ItemDto>,
    partilhas: Vec<PartilhaDto>,
    links: Vec<LinkDto>,
}

/// Build the full detail of a list the user can access, or an error response.
async fn build_detail(state: &AppState, lista_id: i32, user: &CurrentUser) -> Result<ListDetail, Response> {
    let lista = match state.db.get_accessible_lista(lista_id, user.id).await {
        Ok(Some(l)) => l,
        Ok(None) => return Err(err(StatusCode::NOT_FOUND, "Lista não encontrada.")),
        Err(e) => {
            tracing::error!("get_accessible_lista: {:?}", e);
            return Err(err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno."));
        }
    };
    let (despensa, a_comprar) = state.db.articles_for_list(lista_id).await.unwrap_or_default();
    let is_owner = lista.dono_id == user.id;
    let partilhas = if is_owner {
        state.db.partilhas_for_lista(lista_id).await.unwrap_or_default()
            .into_iter().map(|p| PartilhaDto { utilizador_id: p.utilizador_id, username: p.username }).collect()
    } else { Vec::new() };
    let links = if is_owner {
        state.db.active_links_for_lista(lista_id).await.unwrap_or_default()
            .into_iter().map(LinkDto::from).collect()
    } else { Vec::new() };
    Ok(ListDetail {
        id: lista.id,
        nome: lista.nome,
        is_owner,
        a_comprar: a_comprar.into_iter().map(ItemDto::from).collect(),
        despensa: despensa.into_iter().map(ItemDto::from).collect(),
        partilhas,
        links,
    })
}

fn detail_response(d: Result<ListDetail, Response>) -> Response {
    match d {
        Ok(d) => Json(d).into_response(),
        Err(r) => r,
    }
}

// ──────── GET /api/lists ────────

pub async fn lists(State(state): State<AppState>, session: Session) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    let lists = state.db.lists_for_user(user.id).await.unwrap_or_default();
    let user_shared = state.db.user_shared_owned_ids(user.id).await.unwrap_or_default();
    let link_shared = state.db.link_shared_owned_ids(user.id).await.unwrap_or_default();

    // Resolve active list (session value if still visible, else first).
    let mut active_id = listas::active_lista_id(&session).await;
    if let Some(id) = active_id {
        if !lists.iter().any(|l| l.id == id) { active_id = None; }
    }
    if active_id.is_none() {
        if let Some(first) = lists.first() {
            listas::set_active(&session, first.id).await;
            active_id = Some(first.id);
        }
    }

    let summaries = lists.iter().map(|l| ListSummary {
        id: l.id,
        nome: l.nome.clone(),
        // Owned-by-someone-else (shared TO me) or I shared it with users → people.
        shared_users: l.dono_id != user.id || user_shared.contains(&l.id),
        shared_link: link_shared.contains(&l.id),
    }).collect();

    Json(ListsResponse { lists: summaries, active_id: active_id.unwrap_or(0) }).into_response()
}

// ──────── POST /api/lists ────────

#[derive(Deserialize)]
pub struct CreateListBody { pub nome: String }

pub async fn create_list(State(state): State<AppState>, session: Session, Json(body): Json<CreateListBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    let nome = body.nome.trim();
    if nome.is_empty() { return err(StatusCode::BAD_REQUEST, "Nome obrigatório."); }
    match state.db.create_lista(nome, user.id).await {
        Ok(id) => {
            listas::set_active(&session, id).await;
            detail_response(build_detail(&state, id, &user).await)
        }
        Err(e) => { tracing::error!("create_lista: {:?}", e); err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar lista.") }
    }
}

// ──────── POST /api/lists/:id/select ────────

pub async fn select_list(State(state): State<AppState>, session: Session, Path(id): Path<i32>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    match state.db.get_accessible_lista(id, user.id).await {
        Ok(Some(_)) => { listas::set_active(&session, id).await; detail_response(build_detail(&state, id, &user).await) }
        Ok(None) => err(StatusCode::NOT_FOUND, "Lista não encontrada."),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno."),
    }
}

// ──────── GET /api/lists/:id ────────

pub async fn list_detail(State(state): State<AppState>, session: Session, Path(id): Path<i32>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    listas::set_active(&session, id).await;
    detail_response(build_detail(&state, id, &user).await)
}

// ──────── DELETE /api/lists/:id ────────

pub async fn delete_list(State(state): State<AppState>, session: Session, Path(id): Path<i32>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if let Err(e) = state.db.delete_lista(id, user.id).await {
        tracing::warn!("delete_lista: {:?}", e);
    }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ──────── items ────────

#[derive(Deserialize)]
pub struct AddItemBody {
    pub nome: String,
    #[serde(default)]
    pub quantidade: String,
}

pub async fn add_item(State(state): State<AppState>, session: Session, Path(id): Path<i32>, Json(body): Json<AddItemBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    let nome = body.nome.trim();
    if nome.is_empty() { return err(StatusCode::BAD_REQUEST, "Nome obrigatório."); }
    let q = body.quantidade.trim();
    let qty = if q.is_empty() { "1" } else { q };
    // Dedup: if it already exists, move it back to "comprar" instead of a copy.
    match state.db.match_artigo_in_list(id, nome).await {
        Ok(Some(existing)) => { let _ = state.db.toggle_artigo(existing.id, id, Some("comprar")).await; }
        Ok(None) => { if let Err(e) = state.db.add_artigo(id, nome, qty).await { tracing::error!("add_artigo: {:?}", e); } }
        Err(e) => tracing::error!("match_artigo_in_list: {:?}", e),
    }
    detail_response(build_detail(&state, id, &user).await)
}

#[derive(Deserialize)]
pub struct ToggleBody {
    #[serde(default)]
    pub destino: String,
}

pub async fn toggle_item(State(state): State<AppState>, session: Session, Path((id, iid)): Path<(i32, i32)>, Json(body): Json<ToggleBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    let destino = match body.destino.as_str() { "" => None, s => Some(s) };
    if let Err(e) = state.db.toggle_artigo(iid, id, destino).await { tracing::warn!("toggle_artigo: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

#[derive(Deserialize)]
pub struct EditItemBody {
    pub nome: String,
    #[serde(default)]
    pub quantidade: String,
}

pub async fn edit_item(State(state): State<AppState>, session: Session, Path((id, iid)): Path<(i32, i32)>, Json(body): Json<EditItemBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    let nome = body.nome.trim();
    if nome.is_empty() { return err(StatusCode::BAD_REQUEST, "Nome obrigatório."); }
    let q = body.quantidade.trim();
    let qty = if q.is_empty() { "1" } else { q };
    if let Err(e) = state.db.edit_artigo(iid, id, nome, qty).await { tracing::warn!("edit_artigo: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

pub async fn delete_item(State(state): State<AppState>, session: Session, Path((id, iid)): Path<(i32, i32)>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    if let Err(e) = state.db.delete_artigo(iid, id).await { tracing::warn!("delete_artigo: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

#[derive(Deserialize)]
pub struct QtyBody { pub direcao: String }

pub async fn qty_item(State(state): State<AppState>, session: Session, Path((id, iid)): Path<(i32, i32)>, Json(body): Json<QtyBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    if let Err(e) = state.db.update_quantidade(iid, id, &body.direcao).await { tracing::warn!("update_quantidade: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

// ──────── search / match ────────

#[derive(Deserialize)]
pub struct SearchQuery { pub q: String }

#[derive(Serialize)]
struct SearchResponse { suggestions: Vec<String> }

pub async fn search(State(state): State<AppState>, session: Session, Query(qp): Query<SearchQuery>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    let q = qp.q.trim();
    if q.len() < 2 { return Json(SearchResponse { suggestions: vec![] }).into_response(); }
    let s = state.db.search_artigos_for_user(user.id, q, 8).await.unwrap_or_default();
    Json(SearchResponse { suggestions: s }).into_response()
}

#[derive(Serialize)]
struct MatchResponse {
    found: bool,
    id: Option<i32>,
    nome: Option<String>,
    na_despensa: Option<bool>,
}

pub async fn match_item(State(state): State<AppState>, session: Session, Path(id): Path<i32>, Query(qp): Query<SearchQuery>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_accessible_lista(id, user.id).await.ok().flatten().is_none() {
        return Json(MatchResponse { found: false, id: None, nome: None, na_despensa: None }).into_response();
    }
    let q = qp.q.trim();
    if q.is_empty() {
        return Json(MatchResponse { found: false, id: None, nome: None, na_despensa: None }).into_response();
    }
    match state.db.match_artigo_in_list(id, q).await {
        Ok(Some(a)) => Json(MatchResponse { found: true, id: Some(a.id), nome: Some(a.nome), na_despensa: Some(a.comprar == 0) }).into_response(),
        _ => Json(MatchResponse { found: false, id: None, nome: None, na_despensa: None }).into_response(),
    }
}

// ──────── share with user ────────

#[derive(Deserialize)]
pub struct ShareBody { pub username: String }

pub async fn share(State(state): State<AppState>, session: Session, Path(id): Path<i32>, Json(body): Json<ShareBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    let target = body.username.trim();
    if target.is_empty() { return err(StatusCode::BAD_REQUEST, "Introduza um nome de utilizador."); }
    if target == user.username { return err(StatusCode::BAD_REQUEST, "Não pode partilhar consigo mesmo."); }
    let other = match state.db.user_by_username(target).await {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::NOT_FOUND, &format!("Utilizador «{}» não existe.", target)),
        Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno."),
    };
    if let Err(e) = state.db.add_partilha(id, other.id).await {
        tracing::warn!("add_partilha: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao partilhar.");
    }
    detail_response(build_detail(&state, id, &user).await)
}

pub async fn unshare(State(state): State<AppState>, session: Session, Path((id, uid)): Path<(i32, i32)>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    if let Err(e) = state.db.remove_partilha(id, uid).await { tracing::warn!("remove_partilha: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

// ──────── share links ────────

#[derive(Deserialize)]
pub struct CreateLinkBody {
    #[serde(default = "d_dur")] pub duracao: i64,
    #[serde(default = "d_uni")] pub unidade: String,
    #[serde(default)] pub pode_adicionar: bool,
    #[serde(default)] pub pode_editar: bool,
    #[serde(default)] pub pode_apagar: bool,
    #[serde(default)] pub pode_toggle: bool,
}
fn d_dur() -> i64 { 24 }
fn d_uni() -> String { "horas".into() }

pub async fn create_link(State(state): State<AppState>, session: Session, Path(id): Path<i32>, Json(body): Json<CreateLinkBody>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    let dur = body.duracao.max(1);
    let delta = match body.unidade.as_str() {
        "dias" => chrono::Duration::days(dur),
        "minutos" => chrono::Duration::minutes(dur),
        _ => chrono::Duration::hours(dur),
    };
    let token = listas::random_hex_32();
    let expira_em = chrono::Utc::now() + delta;
    if let Err(e) = state.db.create_link(id, &token, expira_em, body.pode_adicionar, body.pode_editar, body.pode_apagar, body.pode_toggle).await {
        tracing::error!("create_link: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar link.");
    }
    detail_response(build_detail(&state, id, &user).await)
}

pub async fn delete_link(State(state): State<AppState>, session: Session, Path((id, lid)): Path<(i32, i32)>) -> Response {
    let user = match require(&session).await { Ok(u) => u, Err(r) => return r };
    if state.db.get_owned_lista(id, user.id).await.ok().flatten().is_none() {
        return err(StatusCode::NOT_FOUND, "Lista não encontrada.");
    }
    if let Err(e) = state.db.delete_link(lid, id).await { tracing::warn!("delete_link: {:?}", e); }
    detail_response(build_detail(&state, id, &user).await)
}

// ──────── public share-link view (no auth) ────────

#[derive(Serialize)]
struct PublicView {
    lista_nome: String,
    a_comprar: Vec<ItemDto>,
    despensa: Vec<ItemDto>,
    expira_em_str: String,
    pode_adicionar: bool,
    pode_editar: bool,
    pode_apagar: bool,
    pode_toggle: bool,
    already_logged_in: bool,
}

async fn active_link(state: &AppState, token: &str) -> Option<LinkPartilha> {
    let link = state.db.link_by_token(token).await.ok().flatten()?;
    if link.expira_em <= chrono::Utc::now() { return None; }
    Some(link)
}

async fn public_view(state: &AppState, link: &LinkPartilha, session: &Session) -> Response {
    let nome = state.db.lista_by_id(link.lista_id).await.ok().flatten().map(|l| l.nome).unwrap_or_default();
    let (despensa, a_comprar) = state.db.articles_for_list(link.lista_id).await.unwrap_or_default();
    Json(PublicView {
        lista_nome: nome,
        a_comprar: a_comprar.into_iter().map(ItemDto::from).collect(),
        despensa: despensa.into_iter().map(ItemDto::from).collect(),
        expira_em_str: link.expira_em.format("%d/%m/%Y %H:%M").to_string(),
        pode_adicionar: link.pode_adicionar != 0,
        pode_editar: link.pode_editar != 0,
        pode_apagar: link.pode_apagar != 0,
        pode_toggle: link.pode_toggle != 0,
        already_logged_in: auth::current(session).await.is_some(),
    }).into_response()
}

pub async fn public_get(State(state): State<AppState>, session: Session, Path(token): Path<String>) -> Response {
    match active_link(&state, &token).await {
        Some(link) => public_view(&state, &link, &session).await,
        None => err(StatusCode::NOT_FOUND, "Link expirado."),
    }
}

pub async fn public_add(State(state): State<AppState>, session: Session, Path(token): Path<String>, Json(body): Json<AddItemBody>) -> Response {
    let Some(link) = active_link(&state, &token).await else { return err(StatusCode::NOT_FOUND, "Link expirado."); };
    if link.pode_adicionar == 0 { return err(StatusCode::FORBIDDEN, "Sem permissão."); }
    let nome = body.nome.trim();
    if nome.is_empty() { return err(StatusCode::BAD_REQUEST, "Nome obrigatório."); }
    match state.db.match_artigo_in_list(link.lista_id, nome).await {
        Ok(Some(existing)) => { let _ = state.db.toggle_artigo(existing.id, link.lista_id, Some("comprar")).await; }
        Ok(None) => { let _ = state.db.add_artigo(link.lista_id, nome, "1").await; }
        Err(_) => {}
    }
    public_view(&state, &link, &session).await
}

pub async fn public_toggle(State(state): State<AppState>, session: Session, Path((token, iid)): Path<(String, i32)>, Json(body): Json<ToggleBody>) -> Response {
    let Some(link) = active_link(&state, &token).await else { return err(StatusCode::NOT_FOUND, "Link expirado."); };
    if link.pode_toggle == 0 { return err(StatusCode::FORBIDDEN, "Sem permissão."); }
    let destino = match body.destino.as_str() { "" => None, s => Some(s) };
    let _ = state.db.toggle_artigo(iid, link.lista_id, destino).await;
    public_view(&state, &link, &session).await
}

pub async fn public_delete(State(state): State<AppState>, session: Session, Path((token, iid)): Path<(String, i32)>) -> Response {
    let Some(link) = active_link(&state, &token).await else { return err(StatusCode::NOT_FOUND, "Link expirado."); };
    if link.pode_apagar == 0 { return err(StatusCode::FORBIDDEN, "Sem permissão."); }
    let _ = state.db.delete_artigo(iid, link.lista_id).await;
    public_view(&state, &link, &session).await
}

/// Stash a link token on the session, so login/registration can redeem it.
pub async fn public_stash(session: Session, Path(token): Path<String>) -> Response {
    let _ = session.insert("pending_link_token", &token).await;
    (StatusCode::NO_CONTENT, ()).into_response()
}
