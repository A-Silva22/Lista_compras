use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    auth::{self, CurrentUser},
    hashing::{self, verify, VerifyOutcome},
    AppState,
};

// ──────── Response shapes ────────

#[derive(Serialize)]
pub struct MeResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub needs_email: bool,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn err(status: StatusCode, msg: &str) -> Response {
    let mut r = (status, Json(ErrorBody { error: msg.to_owned() })).into_response();
    *r.status_mut() = status;
    r
}

fn ok_user(u: &CurrentUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: u.id,
        username: u.username.clone(),
        email: u.email.clone(),
        needs_email: u.email.is_empty(),
    })
}

// ──────── /api/me ────────

pub async fn me(session: Session) -> Response {
    match auth::current(&session).await {
        Some(u) => ok_user(&u).into_response(),
        None => err(StatusCode::UNAUTHORIZED, "not authenticated"),
    }
}

// ──────── /api/login ────────

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<LoginBody>,
) -> Response {
    let username = body.username.trim();
    let password = body.password.trim();
    if username.is_empty() || password.is_empty() {
        return err(StatusCode::BAD_REQUEST, "Preencha todos os campos.");
    }

    let user = match state.db.user_by_username(username).await {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::UNAUTHORIZED, "Nome de utilizador ou palavra-passe incorretos."),
        Err(e) => {
            tracing::error!("user_by_username: {:?}", e);
            return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno. Tente de novo.");
        }
    };

    match verify(&user.password, password) {
        VerifyOutcome::Plain => {}
        VerifyOutcome::Legacy => {
            let new_hash = hashing::make(password);
            if let Err(e) = state.db.update_password(user.id, &new_hash).await {
                tracing::warn!("rehash failed for user {}: {:?}", user.id, e);
            } else {
                tracing::info!("legacy_auth_path=migrate username={}", username);
            }
        }
        VerifyOutcome::Failed => {
            return err(StatusCode::UNAUTHORIZED, "Nome de utilizador ou palavra-passe incorretos.");
        }
    }

    if let Err(e) = state.db.touch_last_login(user.id).await {
        tracing::warn!("touch_last_login: {:?}", e);
    }

    let session_user = CurrentUser {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
    };
    if let Err(e) = auth::login(&session, &session_user).await {
        tracing::error!("session login failed: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro de sessão.");
    }
    consume_pending_link(&state, &session, user.id).await;
    ok_user(&session_user).into_response()
}

/// If the visitor previously hit /link/:token/guardar, the token is on
/// the session. After login/register we redeem it: add the user as a
/// share on that lista (unless they're the owner already).
async fn consume_pending_link(state: &AppState, session: &Session, user_id: i32) {
    let token: Option<String> = session.get("pending_link_token").await.ok().flatten();
    let Some(token) = token else { return };
    let _ = session.remove::<String>("pending_link_token").await;
    let Ok(Some(link)) = state.db.link_by_token(&token).await else { return };
    if link.expira_em <= chrono::Utc::now() { return }
    let Ok(Some(lista)) = state.db.lista_by_id(link.lista_id).await else { return };
    if lista.dono_id == user_id { return }
    if let Err(e) = state.db.add_partilha(link.lista_id, user_id).await {
        tracing::warn!("consume_pending_link add_partilha: {:?}", e);
    } else {
        tracing::info!("consume_pending_link: user {} added to lista {}", user_id, link.lista_id);
    }
}

// ──────── /api/register ────────

#[derive(Deserialize)]
pub struct RegisterBody {
    pub username: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<RegisterBody>,
) -> Response {
    let username = body.username.trim();
    let password = body.password.trim();
    if username.is_empty() || password.is_empty() {
        return err(StatusCode::BAD_REQUEST, "Preencha todos os campos.");
    }
    if username.len() > 150 {
        return err(StatusCode::BAD_REQUEST, "Nome demasiado longo.");
    }
    match state.db.username_taken(username).await {
        Ok(true) => return err(StatusCode::CONFLICT, "Este nome de utilizador já existe."),
        Err(e) => {
            tracing::error!("username_taken: {:?}", e);
            return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno.");
        }
        Ok(false) => {}
    }

    let hash = hashing::make(password);
    let user_id = match state.db.create_user(username, &hash).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("create_user: {:?}", e);
            return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar conta.");
        }
    };

    if let Err(e) = state.db.create_default_lista(user_id).await {
        tracing::warn!("create_default_lista: {:?}", e);
    }

    let session_user = CurrentUser {
        id: user_id,
        username: username.to_owned(),
        email: String::new(),
    };
    if let Err(e) = auth::login(&session, &session_user).await {
        tracing::error!("session login: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro de sessão.");
    }
    consume_pending_link(&state, &session, user_id).await;
    ok_user(&session_user).into_response()
}

// ──────── /api/email ────────

#[derive(Deserialize)]
pub struct EmailBody {
    pub email: String,
}

pub async fn set_email(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<EmailBody>,
) -> Response {
    let mut user = match auth::current(&session).await {
        Some(u) => u,
        None => return err(StatusCode::UNAUTHORIZED, "not authenticated"),
    };

    let email = body.email.trim();
    if email.is_empty() || !email.contains('@') || !email.contains('.') {
        return err(StatusCode::BAD_REQUEST, "Email inválido.");
    }

    match state.db.email_taken_by_other(email, user.id).await {
        Ok(true) => return err(StatusCode::CONFLICT, "Este email já está associado a outra conta."),
        Err(e) => {
            tracing::error!("email_taken: {:?}", e);
            return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno.");
        }
        Ok(false) => {}
    }

    if let Err(e) = state.db.update_email(user.id, email).await {
        tracing::error!("update_email: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao gravar email.");
    }
    user.email = email.to_owned();
    let _ = auth::login(&session, &user).await;

    let body_msg = format!(
        "Olá {},\n\nO email {} foi associado à sua conta no ListaIsto.\n\n— ListaIsto",
        user.username, email
    );
    let mailer = state.mailer.clone();
    let to = email.to_owned();
    tokio::spawn(async move {
        if let Err(e) = mailer.send_text(&to, "Email associado — ListaIsto", &body_msg).await {
            tracing::warn!("welcome email: {:?}", e);
        }
    });
    ok_user(&user).into_response()
}

// ──────── /api/password/change ────────

#[derive(Deserialize)]
pub struct ChangePwBody {
    pub old_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<ChangePwBody>,
) -> Response {
    let user = match auth::current(&session).await {
        Some(u) => u,
        None => return err(StatusCode::UNAUTHORIZED, "not authenticated"),
    };
    let old = body.old_password.trim();
    let new = body.new_password.trim();
    if new.len() < 4 {
        return err(StatusCode::BAD_REQUEST, "A nova palavra-passe é demasiado curta.");
    }
    let stored = match state.db.user_by_id(user.id).await {
        Ok(Some(u)) => u,
        _ => return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno."),
    };
    match verify(&stored.password, old) {
        VerifyOutcome::Plain | VerifyOutcome::Legacy => {}
        VerifyOutcome::Failed => return err(StatusCode::BAD_REQUEST, "Palavra-passe atual incorreta."),
    }
    let hash = hashing::make(new);
    if let Err(e) = state.db.update_password(user.id, &hash).await {
        tracing::error!("update_password: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao gravar.");
    }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ──────── /api/username ────────

#[derive(Deserialize)]
pub struct ChangeUsernameBody {
    pub username: String,
}

pub async fn change_username(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<ChangeUsernameBody>,
) -> Response {
    let mut user = match auth::current(&session).await {
        Some(u) => u,
        None => return err(StatusCode::UNAUTHORIZED, "not authenticated"),
    };
    let username = body.username.trim();
    if username.is_empty() {
        return err(StatusCode::BAD_REQUEST, "Introduza um nome de utilizador.");
    }
    if username.len() > 150 {
        return err(StatusCode::BAD_REQUEST, "Nome demasiado longo.");
    }
    if username == user.username {
        return ok_user(&user).into_response();
    }
    match state.db.username_taken_by_other(username, user.id).await {
        Ok(true) => return err(StatusCode::CONFLICT, "Este nome de utilizador já existe."),
        Err(e) => {
            tracing::error!("username_taken_by_other: {:?}", e);
            return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno.");
        }
        Ok(false) => {}
    }
    if let Err(e) = state.db.update_username(user.id, username).await {
        tracing::error!("update_username: {:?}", e);
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao gravar.");
    }
    user.username = username.to_owned();
    let _ = auth::login(&session, &user).await;
    ok_user(&user).into_response()
}

// ──────── /api/logout ────────

pub async fn logout(session: Session) -> Response {
    let _ = auth::logout(&session).await;
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ──────── /api/password/recover ────────

#[derive(Deserialize)]
pub struct RecoverBody {
    pub email: String,
}

#[derive(Serialize)]
struct RecoverResponse { sent: bool }

pub async fn recover(
    State(state): State<AppState>,
    Json(body): Json<RecoverBody>,
) -> Response {
    let email = body.email.trim();
    if email.is_empty() {
        return err(StatusCode::BAD_REQUEST, "Introduza o seu email.");
    }
    if let Ok(Some(user)) = state.db.user_by_email(email).await {
        // One-hour single-use token; email an absolute reset link.
        let token = crate::handlers::listas::random_hex_32();
        let expira = chrono::Utc::now() + chrono::Duration::hours(1);
        if let Err(e) = state.db.create_password_reset(user.id, &token, expira).await {
            tracing::error!("create_password_reset: {:?}", e);
        } else {
            let base = std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "https://listaisto.pt".into());
            let link = format!("{}/reset/{}", base.trim_end_matches('/'), token);
            let body_msg = format!(
                "Olá {},\n\nRecebemos um pedido para recuperar a tua palavra-passe.\n\n\
                 Abre este link (válido durante 1 hora) para definir uma nova:\n{}\n\n\
                 Se não foste tu, ignora este email.\n\n— ListaIsto",
                user.username, link
            );
            let mailer = state.mailer.clone();
            let to = email.to_owned();
            tokio::spawn(async move {
                if let Err(e) = mailer
                    .send_text(&to, "Recuperação de palavra-passe — ListaIsto", &body_msg)
                    .await
                {
                    tracing::warn!("recovery email: {:?}", e);
                }
            });
        }
    }
    // Always report success — don't reveal whether an email exists.
    Json(RecoverResponse { sent: true }).into_response()
}

// ──────── /api/password/reset ────────

#[derive(Deserialize)]
pub struct ResetBody {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize)]
struct ResetCheckResponse { valid: bool }

/// GET — validate a reset token on page load (does not consume it).
pub async fn reset_check(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Response {
    let valid = state.db.password_reset_valid(token.trim()).await.unwrap_or(false);
    Json(ResetCheckResponse { valid }).into_response()
}

pub async fn reset(
    State(state): State<AppState>,
    Json(body): Json<ResetBody>,
) -> Response {
    let new = body.new_password.trim();
    if new.len() < 4 {
        return err(StatusCode::BAD_REQUEST, "A palavra-passe é demasiado curta.");
    }
    match state.db.consume_password_reset(body.token.trim()).await {
        Ok(Some(user_id)) => {
            let hash = hashing::make(new);
            if let Err(e) = state.db.update_password(user_id, &hash).await {
                tracing::error!("reset update_password: {:?}", e);
                return err(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao gravar.");
            }
            (StatusCode::NO_CONTENT, ()).into_response()
        }
        Ok(None) => err(StatusCode::BAD_REQUEST, "Link inválido ou expirado."),
        Err(e) => {
            tracing::error!("consume_password_reset: {:?}", e);
            err(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno.")
        }
    }
}
