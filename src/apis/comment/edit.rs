use crate::session::auth;
use crate::session::User;
use crate::types::comment::content::Content;
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
    pub comment_id: Id,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum ErrorKind {
    #[error("You are not allowed to edit this comment")]
    UserCantEditComments,
    #[error("Invalid content")]
    InvalidContent,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Id,
    pub content: String,
}

#[post("/api/comment/edit")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/comment/{}/edit", request.id);

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let comment_id = request.id;

    service_inner(state, request, user)
        .await
        .map_err(|err| Error::new(err, comment_id))
}

async fn service_inner(
    state: Data<State>,
    request: Request,
    user: User,
) -> Result<HttpResponse, ErrorKind> {
    if !user.role.can_post() {
        return Err(ErrorKind::UserCantEditComments);
    }

    let content = Content::parse(request.content).map_err(|_| ErrorKind::InvalidContent)?;

    let comment_update_result = update_comment(&state.db_pool, request.id, &content, &user).await?;

    if comment_update_result.rows_affected() == 0 {
        return Err(ErrorKind::UserCantEditComments);
    }

    let comment = fetch_comment_by_id(&state.db_pool, request.id).await?;

    let location = format!(
        "/post/{}?start_id={}#{}",
        comment.post_id, comment.id, comment.id
    );

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn update_comment(
    db_pool: &MySqlPool,
    comment_id: Id,
    content: &Content,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    let markdown_content = markdown::to_html(content.as_ref());

    sqlx::query!(
        "
        UPDATE comments
        SET
            content = ?,
            markdown_content = ?,
            modified_at = NOW()
        WHERE
            id = ?
            AND posted_by_user_id = ?
        ",
        content.as_ref(),
        markdown_content,
        comment_id,
        user.id
    )
    .execute(db_pool)
    .await
}

#[derive(Debug)]
struct Comment {
    pub id: Id,
    pub post_id: Id,
}

async fn fetch_comment_by_id(db_pool: &MySqlPool, comment_id: Id) -> sqlx::Result<Comment> {
    sqlx::query_as!(
        Comment,
        "
        SELECT
            id,
            post_id
        FROM comments
        WHERE
            id = ?
        ",
        comment_id
    )
    .fetch_one(db_pool)
    .await
}

impl From<sqlx::Error> for ErrorKind {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn new(kind: ErrorKind, comment_id: Id) -> Self {
        Self { kind, comment_id }
    }

    pub fn as_location(&self) -> String {
        format!("/comment/{}/edit?error={self}", self.comment_id)
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
