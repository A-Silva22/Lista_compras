use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{self, CurrentUser},
    hashing::{verify, VerifyOutcome},
    templates::EntrarPage,
    AppState,
};

#[derive(Deserialize)]
pub struct EntrarQuery {
    #[serde(default)]
    pub next: String,
}

#[derive(Deserialize)]
pub struct EntrarForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub next: String,
}

pub async fn show(
    session: Session,
    Query(q): Query<EntrarQuery>,
) -> Response {
    if auth::current(&session).await.is_some() {
        return Redirect::to("/").into_response();
    }
    let page = EntrarPage { erro: "", next: &q.next };
    askama_axum::IntoResponse::into_response(page)
}

pub async fn submit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<EntrarForm>,
) -> Response {
    if auth::current(&session).await.is_some() {
        return Redirect::to("/").into_response();
    }
    let username = form.username.trim();
    let password = form.password.trim();
    if username.is_empty() || password.is_empty() {
        return render_error(&form.next, "Preencha todos os campos.");
    }

    let user = match state.db.user_by_username(username).await {
        Ok(Some(u)) => u,
        _ => return render_error(&form.next, "Nome de utilizador ou palavra-passe incorretos."),
    };

    match verify(&user.password, password) {
        VerifyOutcome::Plain => {}
        VerifyOutcome::Legacy => {
            let new_hash = crate::hashing::make(password);
            if let Err(e) = state.db.update_password(user.id, &new_hash).await {
                tracing::warn!("password rehash failed for user {}: {:?}", user.id, e);
            } else {
                tracing::info!("legacy_auth_path=migrate username={}", username);
            }
        }
        VerifyOutcome::Failed => {
            return render_error(&form.next, "Nome de utilizador ou palavra-passe incorretos.");
        }
    }

    if let Err(e) = state.db.touch_last_login(user.id).await {
        tracing::warn!("touch_last_login failed: {:?}", e);
    }

    let session_user = CurrentUser {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
    };
    if let Err(e) = auth::login(&session, &session_user).await {
        tracing::error!("session login failed: {:?}", e);
        return render_error(&form.next, "Erro de sessão. Tente novamente.");
    }

    if user.email.is_empty() {
        return Redirect::to("/adicionar_email/").into_response();
    }

    let next = sanitize_next(&form.next);
    Redirect::to(next.as_deref().unwrap_or("/")).into_response()
}

fn render_error(next: &str, msg: &str) -> Response {
    let page = EntrarPage { erro: msg, next };
    askama_axum::IntoResponse::into_response(page)
}

/// Same-origin guard. Reject anything that isn't a relative path or
/// starts with a scheme/host. Mirrors Django's url_has_allowed_host_and_scheme
/// for the local-only-host case.
fn sanitize_next(next: &str) -> Option<String> {
    let n = next.trim();
    if n.is_empty() { return None; }
    if !n.starts_with('/') { return None; }
    if n.starts_with("//") { return None; }  // protocol-relative
    if n.contains("://") { return None; }
    Some(n.to_owned())
}
