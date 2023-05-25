use crate::types::user::{
    code::Code, email::Email, hashed_password::HashedPassword, password::Password,
    password_pair::PasswordPair, role::Role as UserRole, session::Session, username::Username,
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
use sqlx::mysql::{MySqlPool, MySqlQueryResult};
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Invalid username")]
    InvalidUsername,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Passwords do not match")]
    PasswordsDoNotMatch,
    #[error("Username already taken")]
    UsernameTaken,
    #[error("Email already taken")]
    EmailTaken,
    #[error("Failed to hash password")]
    FailedToHashPassword,
    #[error("Failed to send email to the given address")]
    FailedToSendEmail,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_repeat: String,
}

#[post("/api/sign-up")]
pub async fn service(
    state: Data<State>,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let username = Username::parse(request.username).map_err(|_| Error::InvalidUsername)?;
    let email = Email::parse(request.email).map_err(|_| Error::InvalidEmail)?;

    let password = Password::parse(request.password).map_err(|_| Error::InvalidPassword)?;
    let password_repeat =
        Password::parse(request.password_repeat).map_err(|_| Error::InvalidPassword)?;

    let password_pair =
        PasswordPair::parse(password, password_repeat).map_err(|_| Error::PasswordsDoNotMatch)?;

    if is_username_taken(&state.db_pool, &username).await? {
        return Err(Error::UsernameTaken);
    }

    if is_email_taken(&state.db_pool, &email).await? {
        return Err(Error::EmailTaken);
    }

    let hashed_password =
        HashedPassword::hash(&password_pair).map_err(|_| Error::FailedToHashPassword)?;

    let session = Session::new(&state.db_pool).await?;

    let code = Code::new(&state.db_pool).await?;

    // TODO: Check for bounce
    send_email(
        &state.domain,
        state.email_from.clone(),
        &state.mailer,
        &email,
        &code,
    )
    .await?;

    insert_user(
        &state.db_pool,
        &username,
        &email,
        &hashed_password,
        &session,
        &code,
    )
    .await?;

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/email-verification"))
        .finish();

    Ok(response)
}

async fn insert_user(
    db_pool: &MySqlPool,
    username: &Username,
    email: &Email,
    hashed_password: &HashedPassword,
    session: &Session,
    code: &Code,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        INSERT INTO users
        (
            username,
            email,
            password,
            role,
            session,
            code,
            mini_pfp_file_id,
            pfp_file_id,
            name,
            description,
            joined_at
        )
        VALUES
        (
            ?,
            ?,
            ?,
            ?,
            ?,
            ?,
            NULL,
            NULL,
            '',
            '',
            NOW()
        )
        ",
        username.as_ref(),
        email.as_ref(),
        hashed_password.as_ref(),
        UserRole::default().as_str(),
        session.as_ref(),
        code.as_ref(),
    )
    .execute(db_pool)
    .await
}

async fn send_email(
    domain: &str,
    from: Mailbox,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
    to: &Email,
    code: &Code,
) -> Result<(), Error> {
    let body = format!("Your registration verification code is {}.\n\nTo proceed go to https://{domain}/email-verification\n\nIf you didn't sign up at https://{domain} ignore this message.", code.as_ref());

    let to = format!("<{}>", to.as_ref())
        .parse()
        .map_err(|_| Error::InvalidEmail)?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("neor registration")
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .unwrap();

    mailer.send(email).await?;

    Ok(())
}

async fn is_username_taken(db_pool: &MySqlPool, username: &Username) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        SELECT
            id
        FROM users
        WHERE
            username = ?
        ",
        username.as_ref()
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.is_some())
}
async fn is_email_taken(db_pool: &MySqlPool, email: &Email) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        SELECT
            id
        FROM users
        WHERE
            email = ?
        ",
        email.as_ref()
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.is_some())
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
        format!("/sign-up?error={self}")
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
