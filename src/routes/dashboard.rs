use axum::{
  Router,
  body::Body,
  extract::State,
  http::Response,
  response::{Html, IntoResponse},
  routing::get,
};
use minijinja::context;

use crate::{AppState, error::AppError};

pub fn router() -> Router<AppState> {
  Router::new().route("/", get(dashboard_page))
}

async fn dashboard_page(State(state): State<AppState>) -> Result<Response<Body>, AppError> {
  let html = state.templates.render("dashboard.html", context! {})?;
  Ok(Html(html).into_response())
}
