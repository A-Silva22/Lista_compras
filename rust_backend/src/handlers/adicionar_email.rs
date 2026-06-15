use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth,
    templates::AdicionarEmailPage,
    AppState,
};

#[derive(Deserialize)]
pub struct EmailForm {
    pub email: String,
}

pub async fn show(session: Session) -> Response {
    let user = match auth::current(&session).await {
        Some(u) => u,
        None => return Redirect::to("/entrar/").into_response(),
    };
    if !user.email.is_empty() {
        return Redirect::to("/").into_response();
    }
    askama_axum::IntoResponse::into_response(AdicionarEmailPage {
        erro: "",
        username: &user.username,
    })
}

pub async fn submit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<EmailForm>,
) -> Response {
    let mut user = match auth::current(&session).await {
        Some(u) => u,
        None => return Redirect::to("/entrar/").into_response(),
    };

    let email = form.email.trim();
    if email.is_empty() || !email.contains('@') {
        return render(&user.username, "Introduza um endereço de email válido.");
    }

    match state.db.email_taken_by_other(email, user.id).await {
        Ok(true) => return render(&user.username, "Este email já está associado a outra conta."),
        Err(e) => {
            tracing::error!("email_taken_by_other failed: {:?}", e);
            return render(&user.username, "Erro ao validar email. Tente de novo.");
        }
        Ok(false) => {}
    }

    if let Err(e) = state.db.update_email(user.id, email).await {
        tracing::error!("update_email failed: {:?}", e);
        return render(&user.username, "Erro ao gravar email. Tente de novo.");
    }

    user.email = email.to_owned();
    if let Err(e) = auth::login(&session, &user).await {
        tracing::error!("session refresh failed: {:?}", e);
    }

    // Send a confirmation email so the new account flow proves email works.
    let body = format!(
        "Olá {},\n\nO email {} foi associado à sua conta no ListaIsto.\n\nSe não fez esta alteração, contacte-nos.\n\n— ListaIsto",
        user.username, email
    );
    let mailer = state.mailer.clone();
    let to = email.to_owned();
    tokio::spawn(async move {
        if let Err(e) = mailer.send_text(&to, "Email associado — ListaIsto", &body).await {
            tracing::warn!("welcome email send failed: {:?}", e);
        }
    });

    Redirect::to("/").into_response()
}

fn render(username: &str, msg: &str) -> Response {
    askama_axum::IntoResponse::into_response(AdicionarEmailPage { erro: msg, username })
}
