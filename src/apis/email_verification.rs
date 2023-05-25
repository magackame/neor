use crate::types::user::role::Role as UserRole;
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpResponse, ResponseError,
};
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Invalid code")]
    InvalidCode,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub code: String,
}

#[post("/api/email-verification")]
pub async fn service(
    state: Data<State>,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    if !verify_email(&state.db_pool, &request.code).await? {
        return Err(Error::InvalidCode);
    }

    let response = HttpResponse::SeeOther()
        .append_header((
            header::LOCATION,
            "/sign-in?message=Successfully verified email",
        ))
        .finish();

    Ok(response)
}

async fn verify_email(db_pool: &MySqlPool, code: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        UPDATE users
        SET
            code = NULL,
            role = ?
        WHERE
            code = ?
            AND role = ?
        ",
        UserRole::Member.as_str(),
        code,
        UserRole::default().as_str(),
    )
    .execute(db_pool)
    .await
    .map(|result| result.rows_affected() == 1)
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn as_location(&self) -> String {
        format!("/email-verification?error={self}")
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
