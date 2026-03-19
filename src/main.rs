mod auth;
mod config;
mod db;
mod error;
mod routes;
mod templates;
mod setup;

use std::sync::Arc;

use axum::{Router, middleware};
use tower_http::{
  cors::{Any, CorsLayer},
  trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

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
  setup::seed_admin_user(&db, &config).await?;

  let raw_key = setup::get_secret_key(&config).await;
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
    .with_graceful_shutdown(setup::handle_graceful_shutdown())
    .await?;

  info!("Server shut down gracefully.");
  Ok(())
}
