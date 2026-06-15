use std::{net::SocketAddr, sync::Arc};

use axum::{
    http::{header, HeaderValue},
    routing::{delete, get, post},
    Router,
};
use sqlx::mysql::MySqlPoolOptions;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_sessions::{cookie::SameSite, MemoryStore, SessionManagerLayer, Expiry};
use tower_sessions::cookie::time::Duration;

mod auth;
mod db;
mod handlers;
mod hashing;
mod mailer;
mod templates;

use db::Db;
use mailer::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub mailer: Arc<Mailer>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::from_filename("../.env").ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".into()),
        )
        .init();

    let db_user = std::env::var("DB_USER")?;
    let db_pass = std::env::var("DB_PASSWORD")?;
    let db_name = std::env::var("DB_NAME")?;
    let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".into());

    let dsn = format!(
        "mysql://{}:{}@{}:{}/{}",
        db_user,
        urlencoding(&db_pass),
        db_host,
        db_port,
        db_name
    );

    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&dsn)
        .await?;
    tracing::info!("connected to mariadb at {}:{}", db_host, db_port);

    let state = AppState {
        db: Db::new(pool),
        mailer: Arc::new(Mailer::from_env()?),
    };

    // App-owned schema: create user_email + password_reset and migrate any
    // existing emails out of Django's auth_user. Runs every boot (idempotent).
    state.db.ensure_schema().await?;
    tracing::info!("schema ensured: user_email + password_reset (emails migrated from auth_user)");

    let session_store = MemoryStore::default();
    // Cookie Secure flag is env-driven: defaults to true (safe for the HTTPS
    // production site); set COOKIE_SECURE=false for local HTTP development.
    let cookie_secure = std::env::var("COOKIE_SECURE")
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(true);
    tracing::info!("session cookie secure={}", cookie_secure);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cookie_secure)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_name("rust_sid")
        .with_expiry(Expiry::OnInactivity(Duration::days(14)));

    let app = Router::new()
        .route("/", get(handlers::index::show))
        .route("/home", get(handlers::index::show))
        .route("/home/", get(handlers::index::show))
        .route("/entrar", get(handlers::entrar::show).post(handlers::entrar::submit))
        .route("/entrar/", get(handlers::entrar::show).post(handlers::entrar::submit))
        .route("/registar", get(handlers::registar::show).post(handlers::registar::submit))
        .route("/registar/", get(handlers::registar::show).post(handlers::registar::submit))
        .route("/sair", get(handlers::sair::go).post(handlers::sair::go))
        .route("/sair/", get(handlers::sair::go).post(handlers::sair::go))
        .route(
            "/adicionar_email",
            get(handlers::adicionar_email::show).post(handlers::adicionar_email::submit),
        )
        .route(
            "/adicionar_email/",
            get(handlers::adicionar_email::show).post(handlers::adicionar_email::submit),
        )
        .route("/recuperar-password", get(handlers::recuperar::show).post(handlers::recuperar::submit))
        .route("/recuperar-password/", get(handlers::recuperar::show).post(handlers::recuperar::submit))
        // Lista + artigo CRUD (form-post style, redirect /home)
        .route("/lista/criar", post(handlers::listas::criar))
        .route("/lista/:id/selecionar", get(handlers::listas::selecionar).post(handlers::listas::selecionar))
        .route("/lista/:id/apagar", post(handlers::listas::apagar))
        .route("/lista/:id/partilhar", post(handlers::listas::partilhar))
        .route("/lista/:id/partilha/:user_id/apagar", post(handlers::listas::remover_partilha))
        .route("/lista/:id/link/criar", post(handlers::listas::criar_link))
        .route("/lista/:id/link/:link_id/apagar", post(handlers::listas::apagar_link))
        .route("/artigo/adicionar", post(handlers::listas::adicionar_artigo))
        .route("/artigo/:id/toggle", post(handlers::listas::toggle_artigo))
        .route("/artigo/:id/editar", post(handlers::listas::editar_artigo))
        .route("/artigo/:id/apagar", post(handlers::listas::apagar_artigo))
        .route("/artigo/:id/quantidade/:direcao", post(handlers::listas::quantidade))
        .route("/artigos/search", get(handlers::listas::search_artigos))
        .route("/artigos/match", get(handlers::listas::match_artigo))
        // Public share-link viewer
        .route("/link/:token", get(handlers::links_publicos::ver))
        .route("/link/:token/match", get(handlers::links_publicos::match_artigo))
        .route("/link/:token/guardar", post(handlers::links_publicos::guardar))
        .route("/link/:token/adicionar", post(handlers::links_publicos::adicionar))
        .route("/link/:token/:artigo_id/toggle", post(handlers::links_publicos::toggle))
        .route("/link/:token/:artigo_id/apagar", post(handlers::links_publicos::apagar))
        // JSON API for the React frontend
        .route("/api/me", get(handlers::api::me))
        .route("/api/login", post(handlers::api::login))
        .route("/api/register", post(handlers::api::register))
        .route("/api/email", post(handlers::api::set_email))
        .route("/api/logout", post(handlers::api::logout))
        .route("/api/password/recover", post(handlers::api::recover))
        .route("/api/password/reset", post(handlers::api::reset))
        .route("/api/password/reset/:token", get(handlers::api::reset_check))
        .route("/api/password/change", post(handlers::api::change_password))
        // JSON list/item/share API for the React SPA
        .route("/api/lists", get(handlers::api_listas::lists).post(handlers::api_listas::create_list))
        .route("/api/lists/:id", get(handlers::api_listas::list_detail).delete(handlers::api_listas::delete_list))
        .route("/api/lists/:id/select", post(handlers::api_listas::select_list))
        .route("/api/lists/:id/items", post(handlers::api_listas::add_item))
        .route("/api/lists/:id/items/:iid/toggle", post(handlers::api_listas::toggle_item))
        .route("/api/lists/:id/items/:iid/qty", post(handlers::api_listas::qty_item))
        .route("/api/lists/:id/items/:iid", post(handlers::api_listas::edit_item).delete(handlers::api_listas::delete_item))
        .route("/api/lists/:id/match", get(handlers::api_listas::match_item))
        .route("/api/lists/:id/share", post(handlers::api_listas::share))
        .route("/api/lists/:id/share/:uid", delete(handlers::api_listas::unshare))
        .route("/api/lists/:id/links", post(handlers::api_listas::create_link))
        .route("/api/lists/:id/links/:lid", delete(handlers::api_listas::delete_link))
        .route("/api/search", get(handlers::api_listas::search))
        // Public share-link JSON
        .route("/api/public/:token", get(handlers::api_listas::public_get))
        .route("/api/public/:token/stash", post(handlers::api_listas::public_stash))
        .route("/api/public/:token/items", post(handlers::api_listas::public_add))
        .route("/api/public/:token/items/:iid/toggle", post(handlers::api_listas::public_toggle))
        .route("/api/public/:token/items/:iid", delete(handlers::api_listas::public_delete))
        .route("/healthz", get(|| async { "ok" }))
        .nest_service("/static", ServeDir::new("static"))
        .layer(session_layer)
        // Disable browser caching for everything served by Rust. Templates
        // are reloaded between requests so stale CSS cannot persist.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::PRAGMA,
            HeaderValue::from_static("no-cache"),
        ))
        .with_state(state);

    let port: u16 = std::env::var("RUST_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8766);
    let bind_ip: std::net::IpAddr = std::env::var("RUST_BIND")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::net::IpAddr::from([127, 0, 0, 1]));
    let addr = SocketAddr::from((bind_ip, port));
    tracing::info!("rust backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => out.push(c),
            _ => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    out
}
