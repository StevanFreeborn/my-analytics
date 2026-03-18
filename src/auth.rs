use argon2::{
  Argon2,
  password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use axum::{
  body::Body,
  extract::Request,
  middleware::Next,
  response::{IntoResponse, Redirect, Response},
};

use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar, SameSite};

use crate::AppState;

pub const SESSION_COOKIE: &str = "session";

#[derive(Clone)]
pub struct CookieKey(pub Key);

impl axum::extract::FromRef<AppState> for CookieKey {
  fn from_ref(state: &AppState) -> Self {
    state.cookie_key.clone()
  }
}

impl From<CookieKey> for Key {
  fn from(k: CookieKey) -> Self {
    k.0
  }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
  let salt = SaltString::generate(&mut OsRng);
  let hash = Argon2::default()
    .hash_password(password.as_bytes(), &salt)
    .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
    .to_string();

  Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
  let Ok(parsed) = PasswordHash::new(hash) else {
    return false;
  };

  Argon2::default()
    .verify_password(password.as_bytes(), &parsed)
    .is_ok()
}

pub async fn require_auth(
  jar: PrivateCookieJar<CookieKey>,
  request: Request<Body>,
  next: Next,
) -> Response {
  if jar.get(SESSION_COOKIE).is_some() {
    return next.run(request).await;
  }

  Redirect::to("/login").into_response()
}

pub fn make_session_cookie(user_id: &str) -> Cookie<'static> {
  Cookie::build((SESSION_COOKIE, user_id.to_owned()))
    .path("/")
    .http_only(true)
    .same_site(SameSite::Lax)
    .build()
}

pub fn remove_session_cookie() -> Cookie<'static> {
  let mut c = Cookie::build((SESSION_COOKIE, ""))
    .path("/")
    .http_only(true)
    .build();

  c.make_removal();

  c
}
