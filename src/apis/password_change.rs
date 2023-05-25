use crate::types::user::{
    hashed_password::HashedPassword, password::Password, password_pair::PasswordPair,
};
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpResponse, ResponseError,
};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::Error as SmtpError;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Invalid code")]
    InvalidCode,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Passwords do not match")]
    PasswordsDoNotMatch,
    #[error("Failed to hash password")]
    FailedToHashPassword,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Failed to send email")]
    FailedToSendEmail,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub code: String,
    pub password: String,
    pub password_repeat: String,
}

#[post("/api/password-change")]
pub async fn service(
    state: Data<State>,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let password = Password::parse(request.password).map_err(|_| Error::InvalidPassword)?;
    let password_repeat =
        Password::parse(request.password_repeat).map_err(|_| Error::InvalidPassword)?;

    let password_pair =
        PasswordPair::parse(password, password_repeat).map_err(|_| Error::PasswordsDoNotMatch)?;

    let hashed_password =
        HashedPassword::hash(&password_pair).map_err(|_| Error::FailedToHashPassword)?;

    let Some(email) = fetch_user_email(&state.db_pool, &request.code).await? else {
        return Err(Error::InvalidCode);
    };

    if !update_user_password(&state.db_pool, &request.code, &hashed_password).await? {
        return Err(Error::InvalidCode);
    }

    send_email(
        &state.domain,
        state.email_from.clone(),
        &state.mailer,
        &email,
    )
    .await?;

    let response = HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            "/sign-in?message=Successfully changed password",
        ))
        .finish();

    Ok(response)
}

async fn send_email(
    domain: &str,
    from: Mailbox,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
    to: &str,
) -> Result<(), Error> {
    let body = format!("You password has been successfully changed!\n\nIf you did not change your password IMMEDIATELY reset your password at https://{domain}/password-reset in order to secure your account");

    let to = format!("<{}>", to)
        .parse()
        .map_err(|_| Error::InvalidEmail)?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("neor password reset")
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .unwrap();

    mailer.send(email).await?;

    Ok(())
}

async fn update_user_password(
    db_pool: &MySqlPool,
    code: &str,
    hashed_password: &HashedPassword,
) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        UPDATE users
        SET
            password = ?,
            code = NULL
        WHERE
            code = ?
        ",
        hashed_password.as_ref(),
        code
    )
    .execute(db_pool)
    .await
    .map(|result| result.rows_affected() == 1)
}

async fn fetch_user_email(db_pool: &MySqlPool, code: &str) -> sqlx::Result<Option<String>> {
    #[derive(Debug)]
    struct User {
        email: String,
    }

    sqlx::query_as!(
        User,
        "
        SELECT
            email
        FROM
            users
        WHERE
            code = ?
        ",
        code
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.map(|user| user.email))
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl From<SmtpError> for Error {
    fn from(_: SmtpError) -> Self {
        Self::FailedToSendEmail
    }
}

impl Error {
    pub fn as_location(&self) -> String {
        format!("/password-change?error={self}")
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
