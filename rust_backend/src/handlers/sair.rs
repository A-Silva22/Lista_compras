use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;

use crate::auth;

pub async fn go(session: Session) -> Response {
    if let Err(e) = auth::logout(&session).await {
        tracing::warn!("logout failed: {:?}", e);
    }
    Redirect::to("/entrar/").into_response()
}
