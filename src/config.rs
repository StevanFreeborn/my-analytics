use std::env;

#[derive(Clone, Debug)]
pub struct Config {
  pub database_url: String,
  pub host: String,
  pub port: u16,
  pub admin_username: String,
  pub admin_password: Option<String>,
  pub secret_key: Option<String>,
}

impl Config {
  pub fn from_env() -> Self {
    Self {
      database_url: env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./data/my-analytics.db".to_string()),
      host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
      port: env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000),
      admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
      admin_password: env::var("ADMIN_PASSWORD").ok(),
      secret_key: env::var("SECRET_KEY").ok(),
    }
  }
}
