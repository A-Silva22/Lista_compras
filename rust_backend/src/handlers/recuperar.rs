use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{auth, templates::RecuperarPage, AppState};

#[derive(Deserialize)]
pub struct RecuperarForm {
    pub email: String,
}

pub async fn show(session: Session) -> Response {
    if auth::current(&session).await.is_some() {
        return Redirect::to("/").into_response();
    }
    askama_axum::IntoResponse::into_response(RecuperarPage { enviado: false, erro: "" })
}

pub async fn submit(
    State(state): State<AppState>,
    Form(form): Form<RecuperarForm>,
) -> Response {
    let email = form.email.trim();
    if email.is_empty() {
        return askama_axum::IntoResponse::into_response(RecuperarPage {
            enviado: false,
            erro: "Introduza o seu email.",
        });
    }

    // Always show success — never reveal whether email exists.
    if let Ok(Some(user)) = state.db.user_by_email(email).await {
        let body = format!(
            "Olá {},\n\nRecebemos um pedido para repor a sua palavra-passe.\n\n\
             Esta versão Rust ainda não emite o link — contacte o admin se precisar.\n\n— ListaIsto",
            user.username
        );
        let mailer = state.mailer.clone();
        let to = email.to_owned();
        tokio::spawn(async move {
            if let Err(e) = mailer
                .send_text(&to, "Recuperação de palavra-passe — ListaIsto", &body)
                .await
            {
                tracing::warn!("recovery email send failed: {:?}", e);
            }
        });
    }

    askama_axum::IntoResponse::into_response(RecuperarPage { enviado: true, erro: "" })
}
