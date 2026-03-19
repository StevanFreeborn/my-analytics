mod auth;
mod config;
mod db;
mod error;
mod routes;
mod templates;

use std::sync::Arc;

use axum::{Router, middleware};
use axum_extra::extract::cookie::Key;
use sha2::{Digest, Sha512};
use tokio::signal;
use tower_http::{
  cors::{Any, CorsLayer},
  trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::{auth::CookieKey, config::Config, db::Database};

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
  pub db: Database,
  pub config: Config,
  pub templates: templates::Templates,
  pub cookie_key: CookieKey,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();

  tracing_subscriber::registry()
    .with(
      EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "my_analytics=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  let config = Config::from_env();
  let db = Database::new(&config.database_url).await?;

  db.migrate().await?;

  let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
    .fetch_one(&db.pool)
    .await?;

  if user_count == 0 {
    let password = match &config.admin_password {
      Some(p) => p.clone(),
      None => {
        let generated = Uuid::new_v4().to_string().replace('-', "");
        warn!(
          "No ADMIN_PASSWORD set. Generated admin password: {}",
          generated
        );
        generated
      }
    };

    let hash = auth::hash_password(&password)?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO users (id, username, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
      .bind(&id)
      .bind(&config.admin_username)
      .bind(&hash)
      .bind(&now)
      .bind(&now)
      .execute(&db.pool)
      .await?;

    info!("Created admin user '{}'", config.admin_username);
  }

  let raw_key: Key = match &config.secret_key {
    Some(k) => {
      let digest = Sha512::digest(k.as_bytes());
      let mut bytes = [0u8; 64];
      bytes.copy_from_slice(&digest);
      Key::from(&bytes)
    }
    None => {
      warn!(
        "No SECRET_KEY set. Generating a random cookie key — \
          users will be logged out on every restart. \
          Set SECRET_KEY to a long random string to persist sessions."
      );
      Key::generate()
    }
  };
  let cookie_key = CookieKey(raw_key);

  let templates = templates::Templates::new()?;

  let state: AppState = Arc::new(AppStateInner {
    db,
    config: config.clone(),
    templates,
    cookie_key,
  });

  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

  let public = Router::new().merge(routes::auth::router());

  let protected = Router::new()
    .merge(routes::dashboard::router())
    .route_layer(middleware::from_fn_with_state(
      state.clone(),
      auth::require_auth,
    ));

  let app = Router::new()
    .merge(public)
    .merge(protected)
    .layer(TraceLayer::new_for_http())
    .layer(cors)
    .with_state(state);

  let bind_addr = format!("{}:{}", config.host, config.port);
  let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
  info!("Starting server on http://{}", bind_addr);

  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  info!("Server shut down gracefully.");
  Ok(())
}

async fn shutdown_signal() {
  let ctrl_c = async {
    signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler");
  };

  #[cfg(unix)]
  let terminate = async {
    signal::unix::signal(signal::unix::SignalKind::terminate())
      .expect("failed to install SIGTERM handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
      _ = ctrl_c => { info!("Received Ctrl+C, shutting down..."); },
      _ = terminate => { info!("Received SIGTERM, shutting down..."); },
  }
}
