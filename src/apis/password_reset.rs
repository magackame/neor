use crate::types::user::code::Code;
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpResponse, ResponseError,
};
use lettre::message::Mailbox;
use lettre::transport::smtp::Error as SmtpError;
use lettre::AsyncTransport;
use lettre::Message;
use lettre::{message::header::ContentType, AsyncSmtpTransport, Tokio1Executor};
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Account with that email address was not found")]
    EmailNotFound,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Failed to send email")]
    FailedToSendEmail,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub email: String,
}

#[post("/api/password-reset")]
pub async fn service(
    state: Data<State>,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let code = Code::new(&state.db_pool).await?;

    if !update_code(&state.db_pool, &code, &request.email).await? {
        return Err(Error::EmailNotFound);
    }

    send_email(
        &state.domain,
        state.email_from.clone(),
        &state.mailer,
        &request.email,
        &code,
    )
    .await?;

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/password-change"))
        .finish();

    Ok(response)
}

async fn update_code(db_pool: &MySqlPool, code: &Code, email: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        UPDATE users
        SET
            code = ?
        WHERE
            email = ?
        ",
        code.as_ref(),
        email
    )
    .execute(db_pool)
    .await
    .map(|result| result.rows_affected() == 1)
}

async fn send_email(
    domain: &str,
    from: Mailbox,
    mailer: &AsyncSmtpTransport<Tokio1Executor>,
    to: &str,
    code: &Code,
) -> Result<(), Error> {
    let body = format!("Your password change verification code is {}.\n\nTo proceed go to https://{domain}/password-change\n\nIf you didn't change your password ignore this message.", code.as_ref());

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
        format!("/password-reset?error={self}")
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
