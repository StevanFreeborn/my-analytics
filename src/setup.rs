use anyhow::Ok;
use axum_extra::extract::cookie::Key;
use sha2::{Digest, Sha512};
use tokio::signal;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{auth, config::Config, db::Database};

pub async fn seed_admin_user(db: &Database, config: &Config) -> anyhow::Result<()> {
  let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
    .fetch_one(&db.pool)
    .await?;

  if user_count != 0 {
    return Ok(());
  }

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

  return Ok(());
}

pub async fn get_secret_key(config: &Config) -> Key {
  match &config.secret_key {
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
  }
}

pub async fn handle_graceful_shutdown() {
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
