use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
  #[error("Database error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("Template error: {0}")]
  Template(#[from] minijinja::Error),

  #[error("Serialization error: {0}")]
  Json(#[from] serde_json::Error),

  #[error("Internal error: {0}")]
  Internal(#[from] anyhow::Error),

  #[error("Bad request: {0}")]
  BadRequest(String),

  #[error("Not found")]
  NotFound,

  #[error("Unauthorized")]
  Unauthorized,
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let status = match &self {
      AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
      AppError::NotFound => StatusCode::NOT_FOUND,
      AppError::Unauthorized => StatusCode::UNAUTHORIZED,
      _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    tracing::error!("Application error: {}", self);
    (status, self.to_string()).into_response()
  }
}
