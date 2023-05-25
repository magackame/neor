use super::is_checked;
use crate::session::SetSessionCookie;
use crate::types::id::Id;
use crate::types::user::{email::Email, password::Password};
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpResponse, ResponseError,
};
use argon2::{
    password_hash::{errors::Error as HashingError, PasswordHash, PasswordVerifier},
    Argon2,
};
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Invalid email or password")]
    InvalidEmailOrPassword,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub email: String,
    pub password: String,
    pub remember_me: Option<String>,
    pub back: Option<String>,
}

#[post("/api/sign-in")]
pub async fn service(
    state: Data<State>,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let email = Email::parse(request.email).map_err(|_| Error::InvalidEmailOrPassword)?;
    let password = Password::parse(request.password).map_err(|_| Error::InvalidEmailOrPassword)?;

    let remember_me = is_checked(request.remember_me);

    let user = User::fetch_by_email(&state.db_pool, &email)
        .await?
        .ok_or(Error::InvalidEmailOrPassword)?;

    let parsed_hash = PasswordHash::new(&user.password)?;

    Argon2::default()
        .verify_password(password.as_ref().as_bytes(), &parsed_hash)
        .map_err(|_| Error::InvalidEmailOrPassword)?;

    let location = request.back.unwrap_or("/".to_owned());

    let response = HttpResponse::SeeOther()
        .set_session(&state.domain, &user.session, remember_me)
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

#[derive(Debug)]
pub struct User {
    pub id: Id,
    pub password: String,
    pub session: String,
}

impl User {
    async fn fetch_by_email(db_pool: &MySqlPool, email: &Email) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "
            SELECT
                id,
                password,
                session
            FROM users
            WHERE
                email = ?
            ",
            email.as_ref()
        )
        .fetch_optional(db_pool)
        .await
    }
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl From<HashingError> for Error {
    fn from(_: HashingError) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn as_location(&self) -> String {
        format!("/sign-in?error={self}")
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .append_header((header::LOCATION, self.as_location()))
            .finish()
    }
}
