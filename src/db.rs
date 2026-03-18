use sqlx::{
  SqlitePool,
  sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{str::FromStr, time::Duration};

pub struct Database {
  pub pool: SqlitePool,
}

impl Database {
  pub async fn new(url: &str) -> anyhow::Result<Self> {
    if let Some(path) = url.strip_prefix("sqlite://./")
      && let Some(parent) = std::path::Path::new(path).parent()
      && !parent.as_os_str().is_empty()
    {
      tokio::fs::create_dir_all(parent).await?;
    }

    if let Some(path) = url.strip_prefix("sqlite:///")
      && let Some(parent) = std::path::Path::new(path).parent()
      && !parent.as_os_str().is_empty()
    {
      tokio::fs::create_dir_all(parent).await?;
    }

    let connect_options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
      .max_connections(10)
      .acquire_timeout(Duration::from_secs(3))
      .connect_with(connect_options)
      .await?;

    sqlx::query("PRAGMA journal_mode=WAL")
      .execute(&pool)
      .await?;
    sqlx::query("PRAGMA synchronous=NORMAL")
      .execute(&pool)
      .await?;
    sqlx::query("PRAGMA cache_size=-64000")
      .execute(&pool)
      .await?;
    sqlx::query("PRAGMA temp_store=MEMORY")
      .execute(&pool)
      .await?;
    sqlx::query("PRAGMA mmap_size=268435456")
      .execute(&pool)
      .await?;

    Ok(Self { pool })
  }

  pub async fn migrate(&self) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(&self.pool).await?;
    Ok(())
  }
}
