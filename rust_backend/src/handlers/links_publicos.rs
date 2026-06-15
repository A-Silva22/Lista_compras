//! Public-link routes — anyone with the token can view (and optionally
//! mutate) the list according to the permissions stored on the LinkPartilha.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{db::LinkPartilha, templates::LinkPublicPage, AppState};

#[derive(Deserialize)]
pub struct MatchQuery { pub q: String }

#[derive(Serialize)]
pub struct LinkMatchResponse {
    pub found: bool,
    pub id: Option<i32>,
    pub nome: Option<String>,
    pub na_despensa: Option<bool>,
    pub pode_toggle: bool,
}

pub async fn match_artigo(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(qp): Query<MatchQuery>,
) -> Response {
    let Some(link) = active_link(&state, &token).await else {
        return (StatusCode::NOT_FOUND, "Link expirado.").into_response();
    };
    let pode_toggle = link.pode_toggle != 0;
    let q = qp.q.trim();
    if q.is_empty() {
        return Json(LinkMatchResponse {
            found: false, id: None, nome: None, na_despensa: None, pode_toggle,
        }).into_response();
    }
    match state.db.match_artigo_in_list(link.lista_id, q).await {
        Ok(Some(a)) => Json(LinkMatchResponse {
            found: true,
            id: Some(a.id),
            nome: Some(a.nome),
            na_despensa: Some(a.comprar == 0),
            pode_toggle,
        }).into_response(),
        _ => Json(LinkMatchResponse {
            found: false, id: None, nome: None, na_despensa: None, pode_toggle,
        }).into_response(),
    }
}

/// Returns the link if it is still active.
async fn active_link(state: &AppState, token: &str) -> Option<LinkPartilha> {
    let link = state.db.link_by_token(token).await.ok().flatten()?;
    if link.expira_em <= Utc::now() {
        return None;
    }
    Some(link)
}

pub async fn ver(
    State(state): State<AppState>,
    session: tower_sessions::Session,
    Path(token): Path<String>,
) -> Response {
    let Some(link) = active_link(&state, &token).await else {
        return (StatusCode::NOT_FOUND, "Link expirado ou inválido.").into_response();
    };
    let lista = match state.db.lista_by_id(link.lista_id).await {
        Ok(Some(l)) => l,
        _ => return (StatusCode::NOT_FOUND, "Lista não encontrada.").into_response(),
    };
    let (despensa, a_comprar) = state
        .db
        .articles_for_list(link.lista_id)
        .await
        .unwrap_or_default();
    let already_logged_in = crate::auth::current(&session).await.is_some();
    askama_axum::IntoResponse::into_response(LinkPublicPage {
        token: token.clone(),
        lista_nome: lista.nome,
        despensa,
        a_comprar,
        pode_adicionar: link.pode_adicionar != 0,
        pode_editar: link.pode_editar != 0,
        pode_apagar: link.pode_apagar != 0,
        pode_toggle: link.pode_toggle != 0,
        expira_em_str: link.expira_em.format("%d/%m/%Y %H:%M").to_string(),
        already_logged_in,
    })
}

#[derive(Deserialize, Default)]
pub struct GuardarForm {
    #[serde(default)]
    pub destino: String,
}

/// Stash the link token in the visitor's session and route to login or
/// registration. Used by the public viewer's "Entrar" / "Registar" buttons.
pub async fn guardar(
    session: tower_sessions::Session,
    Path(token): Path<String>,
    Form(form): Form<GuardarForm>,
) -> Response {
    let _ = session.insert("pending_link_token", &token).await;
    match form.destino.as_str() {
        "registar" => Redirect::to("/registar/").into_response(),
        _ => Redirect::to("/entrar/").into_response(),
    }
}

#[derive(Deserialize)]
pub struct AdicionarForm {
    pub nome: String,
    #[serde(default)]
    pub quantidade: String,
}

pub async fn adicionar(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Form(form): Form<AdicionarForm>,
) -> Response {
    let Some(link) = active_link(&state, &token).await else {
        return (StatusCode::NOT_FOUND, "Link expirado.").into_response();
    };
    if link.pode_adicionar == 0 {
        return Redirect::to(&format!("/link/{}", token)).into_response();
    }
    let nome = form.nome.trim();
    if !nome.is_empty() {
        let qty = form.quantidade.trim();
        let qty = if qty.is_empty() { "1" } else { qty };
        let _ = state.db.add_artigo(link.lista_id, nome, qty).await;
    }
    Redirect::to(&format!("/link/{}", token)).into_response()
}

#[derive(Deserialize)]
pub struct ToggleForm {
    #[serde(default)]
    pub destino: String,
}

pub async fn toggle(
    State(state): State<AppState>,
    Path((token, artigo_id)): Path<(String, i32)>,
    Form(form): Form<ToggleForm>,
) -> Response {
    let Some(link) = active_link(&state, &token).await else {
        return (StatusCode::NOT_FOUND, "Link expirado.").into_response();
    };
    if link.pode_toggle == 0 {
        return Redirect::to(&format!("/link/{}", token)).into_response();
    }
    let destino = match form.destino.as_str() {
        "" => None,
        s => Some(s),
    };
    let _ = state
        .db
        .toggle_artigo(artigo_id, link.lista_id, destino)
        .await;
    Redirect::to(&format!("/link/{}", token)).into_response()
}

pub async fn apagar(
    State(state): State<AppState>,
    Path((token, artigo_id)): Path<(String, i32)>,
) -> Response {
    let Some(link) = active_link(&state, &token).await else {
        return (StatusCode::NOT_FOUND, "Link expirado.").into_response();
    };
    if link.pode_apagar == 0 {
        return Redirect::to(&format!("/link/{}", token)).into_response();
    }
    let _ = state.db.delete_artigo(artigo_id, link.lista_id).await;
    Redirect::to(&format!("/link/{}", token)).into_response()
}
