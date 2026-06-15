use serde::{Deserialize, Serialize};
use tower_sessions::Session;

const KEY: &str = "user_id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: i32,
    pub username: String,
    pub email: String,
}

pub async fn login(session: &Session, user: &CurrentUser) -> anyhow::Result<()> {
    session.insert(KEY, user).await?;
    Ok(())
}

pub async fn logout(session: &Session) -> anyhow::Result<()> {
    session.remove::<CurrentUser>(KEY).await?;
    session.flush().await?;
    Ok(())
}

pub async fn current(session: &Session) -> Option<CurrentUser> {
    session.get::<CurrentUser>(KEY).await.ok().flatten()
}
