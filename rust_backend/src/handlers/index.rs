use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::{
    auth,
    db::{Artigo, Lista, PartilhaUser},
    handlers::listas,
    templates::{IndexPage, LinkRow, ListEntry},
    AppState,
};

pub async fn show(State(state): State<AppState>, session: Session) -> Response {
    let user = match auth::current(&session).await {
        Some(u) => u,
        None => return Redirect::to("/entrar/").into_response(),
    };
    if user.email.is_empty() {
        return Redirect::to("/adicionar_email/").into_response();
    }

    let lists: Vec<Lista> = match state.db.lists_for_user(user.id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("lists_for_user: {:?}", e);
            Vec::new()
        }
    };

    // Resolve active list: session value if still owned, else first list.
    let mut active_id = listas::active_lista_id(&session).await;
    if let Some(id) = active_id {
        if !lists.iter().any(|l| l.id == id) {
            listas::clear_active(&session).await;
            active_id = None;
        }
    }
    if active_id.is_none() {
        if let Some(first) = lists.first() {
            listas::set_active(&session, first.id).await;
            active_id = Some(first.id);
        }
    }

    let mut despensa: Vec<Artigo> = Vec::new();
    let mut a_comprar: Vec<Artigo> = Vec::new();
    let mut partilhas: Vec<PartilhaUser> = Vec::new();
    let mut link_rows: Vec<LinkRow> = Vec::new();
    let active_nome = if let Some(id) = active_id {
        match state.db.articles_for_list(id).await {
            Ok((d, c)) => { despensa = d; a_comprar = c; }
            Err(e) => tracing::error!("articles_for_list: {:?}", e),
        }
        partilhas = state.db.partilhas_for_lista(id).await.unwrap_or_default();
        let links = state.db.active_links_for_lista(id).await.unwrap_or_default();
        link_rows = links
            .into_iter()
            .map(|l| LinkRow {
                id: l.id,
                url: format!("/link/{}", l.token),
                expira_em_str: l.expira_em.format("%d/%m/%Y %H:%M").to_string(),
                pode_adicionar: l.pode_adicionar != 0,
                pode_editar: l.pode_editar != 0,
                pode_apagar: l.pode_apagar != 0,
                pode_toggle: l.pode_toggle != 0,
            })
            .collect();
        lists.iter().find(|l| l.id == id).map(|l| l.nome.clone()).unwrap_or_default()
    } else {
        String::new()
    };

    // Mark which lists are shared (owned-but-shared, or shared to this user)
    // so the picker can show an icon next to them.
    let shared_ids = state.db.shared_owned_list_ids(user.id).await.unwrap_or_default();
    let list_entries: Vec<ListEntry> = lists
        .iter()
        .map(|l| ListEntry {
            id: l.id,
            nome: l.nome.clone(),
            shared: l.dono_id != user.id || shared_ids.contains(&l.id),
        })
        .collect();

    askama_axum::IntoResponse::into_response(IndexPage {
        username: user.username.clone(),
        email: user.email.clone(),
        lists: list_entries,
        active_id: active_id.unwrap_or(0),
        active_nome,
        despensa,
        a_comprar,
        partilhas,
        links: link_rows,
    })
}
