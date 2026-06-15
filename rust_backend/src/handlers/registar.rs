use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{self, CurrentUser},
    hashing,
    templates::RegistarPage,
    AppState,
};

#[derive(Deserialize)]
pub struct RegistarQuery {
    #[serde(default)]
    pub next: String,
}

#[derive(Deserialize)]
pub struct RegistarForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub next: String,
}

pub async fn show(session: Session, Query(q): Query<RegistarQuery>) -> Response {
    if auth::current(&session).await.is_some() {
        return Redirect::to("/").into_response();
    }
    askama_axum::IntoResponse::into_response(RegistarPage { erro: "", next: &q.next })
}

pub async fn submit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<RegistarForm>,
) -> Response {
    let username = form.username.trim();
    let password = form.password.trim();

    if username.is_empty() || password.is_empty() {
        return render_error(&form.next, "Preencha todos os campos.");
    }
    if username.len() > 150 {
        return render_error(&form.next, "Nome demasiado longo.");
    }

    match state.db.username_taken(username).await {
        Ok(true) => return render_error(&form.next, "Este nome de utilizador já existe."),
        Err(e) => {
            tracing::error!("username_taken failed: {:?}", e);
            return render_error(&form.next, "Erro ao verificar utilizador. Tente de novo.");
        }
        Ok(false) => {}
    }

    let hash = hashing::make(password);
    let user_id = match state.db.create_user(username, &hash).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("create_user failed: {:?}", e);
            return render_error(&form.next, "Erro ao criar conta. Tente de novo.");
        }
    };

    if let Err(e) = state.db.create_default_lista(user_id).await {
        tracing::warn!("create_default_lista failed: {:?}", e);
    }

    let session_user = CurrentUser {
        id: user_id,
        username: username.to_owned(),
        email: String::new(),
    };
    if let Err(e) = auth::login(&session, &session_user).await {
        tracing::error!("session login failed: {:?}", e);
        return render_error(&form.next, "Erro de sessão. Tente novamente.");
    }

    Redirect::to("/adicionar_email/").into_response()
}

fn render_error(next: &str, msg: &str) -> Response {
    askama_axum::IntoResponse::into_response(RegistarPage { erro: msg, next })
}
