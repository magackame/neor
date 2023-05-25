use crate::db::fetch_comment_by_id;
use crate::session::auth;
use crate::session::User;
use crate::types::id::Id;
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpRequest, HttpResponse, ResponseError,
};
use serde::Deserialize;
use sqlx::mysql::{MySqlPool, MySqlQueryResult};
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
    pub post_id: Id,
}

impl Error {
    pub fn new(kind: ErrorKind, post_id: Id) -> Self {
        Self { kind, post_id }
    }
}

#[derive(Debug, Error, Copy, Clone)]
pub enum ErrorKind {
    #[error("You are not allowed to anonymise this comment")]
    UserCantAnonymiseComment,
    #[error("Commenter's username and confirmation string do not match")]
    InvalidConfirmation,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub comment_id: Id,
    pub confirm: String,
}

#[post("/api/comment/anonymise")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/comment/{}/anonymise", request.comment_id);

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let post_id = request.comment_id;

    service_inner(state, user, request)
        .await
        .map_err(|err| Error::new(err, post_id))
}

async fn service_inner(
    state: Data<State>,
    user: User,
    request: Request,
) -> Result<HttpResponse, ErrorKind> {
    let comment = fetch_comment_by_id(&state.db_pool, request.comment_id, &user)
        .await?
        .ok_or(ErrorKind::UserCantAnonymiseComment)?;

    if !comment.is_anonymisable {
        return Err(ErrorKind::UserCantAnonymiseComment);
    }

    if comment
        .posted_by
        .map(|posted_by| posted_by.username != request.confirm)
        .unwrap_or(true)
    {
        return Err(ErrorKind::InvalidConfirmation);
    }

    anonymise_comment(&state.db_pool, request.comment_id, &user).await?;

    let location = format!(
        "/post/{}?start_id={}#{}",
        comment.post_id, request.comment_id, request.comment_id
    );

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn anonymise_comment(
    db_pool: &MySqlPool,
    post_id: Id,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE comments
        SET
            posted_by_user_id = NULL
        WHERE
            id = ?
            AND posted_by_user_id = ?
        ",
        post_id,
        user.id
    )
    .execute(db_pool)
    .await
}

impl From<sqlx::Error> for ErrorKind {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn as_location(&self) -> String {
        format!("/comment/{}/anonymise?error={self}", self.post_id)
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
