use axum::{
  Form, Router,
  body::Body,
  extract::State,
  http::{Response, StatusCode},
  response::{Html, IntoResponse, Redirect},
  routing::{get, post},
};
use axum_extra::extract::PrivateCookieJar;
use minijinja::context;
use serde::Deserialize;

use crate::{
  AppState,
  auth::{CookieKey, make_session_cookie, remove_session_cookie, verify_password},
  error::AppError,
};

pub fn router() -> Router<AppState> {
  Router::new()
    .route("/login", get(login_page))
    .route("/login", post(login_submit))
    .route("/logout", post(logout))
}

async fn login_page(
  State(state): State<AppState>,
  jar: PrivateCookieJar<CookieKey>,
) -> Result<Response<Body>, AppError> {
  if jar.get(crate::auth::SESSION_COOKIE).is_some() {
    return Ok(Redirect::to("/").into_response());
  }

  let html = state.templates.render("login.html", context! {})?;
  Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct LoginForm {
  pub username: String,
  pub password: String,
}

async fn login_submit(
  State(state): State<AppState>,
  jar: PrivateCookieJar<CookieKey>,
  Form(form): Form<LoginForm>,
) -> Result<Response<Body>, AppError> {
  let row: Option<(String, String)> =
    sqlx::query_as("SELECT id, password_hash FROM users WHERE username = ?")
      .bind(&form.username)
      .fetch_optional(&state.db.pool)
      .await?;

  let valid = match row {
    Some((user_id, hash)) if verify_password(&form.password, &hash) => Some(user_id),
    _ => None,
  };

  match valid {
    Some(user_id) => {
      let jar = jar.add(make_session_cookie(&user_id));
      Ok((jar, Redirect::to("/")).into_response())
    }
    None => {
      let html = state.templates.render(
        "login.html",
        context! { error => "Invalid username or password." },
      )?;
      Ok((StatusCode::UNAUTHORIZED, Html(html)).into_response())
    }
  }
}

async fn logout(jar: PrivateCookieJar<CookieKey>) -> impl IntoResponse {
  let jar = jar.add(remove_session_cookie());
  (jar, Redirect::to("/login"))
}
