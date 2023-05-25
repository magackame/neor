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
    pub post_id: Id,
    pub reply_to_comment_id: Option<Id>,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum ErrorKind {
    #[error("You are not allowed to comment")]
    UserCantComment,
    #[error("Reply and original comment must be on the same post")]
    InvalidReply,
    #[error("Post not found")]
    PostNotFound,
    #[error("Invalid content")]
    InvalidContent,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub post_id: Id,
    pub reply_to_comment_id: Option<Id>,
    pub content: String,
}

#[post("/api/comment/create")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/sign-in?back=/comment/create"))
            .finish();

        return Ok(response);
    };

    let post_id = request.post_id;
    let reply_to_comment_id = request.reply_to_comment_id;

    service_inner(state, request, user)
        .await
        .map_err(|err| Error::new(err, post_id, reply_to_comment_id))
}

async fn service_inner(
    state: Data<State>,
    request: Request,
    user: User,
) -> Result<HttpResponse, ErrorKind> {
    if !user.role.can_comment() {
        return Err(ErrorKind::UserCantComment);
    }

    // Checks that post with id `request.post_id` exists
    // and that reply and original comment share the same `post_id`
    match request.reply_to_comment_id {
        Some(comment_id) => match fetch_comment_post_id(&state.db_pool, comment_id).await? {
            Some(post_id) => {
                if post_id != request.post_id {
                    return Err(ErrorKind::InvalidReply);
                }
            }
            None => {
                return Err(ErrorKind::InvalidContent);
            }
        },
        None => {
            if !fetch_post_exists(&state.db_pool, request.post_id).await? {
                return Err(ErrorKind::PostNotFound);
            }
        }
    }

    let content = Content::parse(request.content).map_err(|_| ErrorKind::InvalidContent)?;

    let comment_insert_result = insert_comment(
        &state.db_pool,
        request.post_id,
        request.reply_to_comment_id,
        &content,
        &user,
    )
    .await?;

    let comment_id = comment_insert_result.last_insert_id();

    let location = format!(
        "/post/{}?start_id={comment_id}#{comment_id}",
        request.post_id
    );

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn insert_comment(
    db_pool: &MySqlPool,
    post_id: Id,
    reply_to_comment_id: Option<Id>,
    content: &Content,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    let markdown_content = markdown::to_html(content.as_ref());

    sqlx::query!(
        "
        INSERT INTO comments
        (
            post_id,
            reply_to_comment_id,
            content,
            markdown_content,
            posted_by_user_id,
            posted_at,
            modified_at
        )
        VALUES
        (
            ?,
            ?,
            ?,
            ?,
            ?,
            NOW(),
            NULL
        )
        ",
        post_id,
        reply_to_comment_id,
        content.as_ref(),
        markdown_content,
        user.id
    )
    .execute(db_pool)
    .await
}

async fn fetch_comment_post_id(db_pool: &MySqlPool, comment_id: Id) -> sqlx::Result<Option<Id>> {
    #[derive(Debug)]
    struct Comment {
        pub post_id: Id,
    }

    sqlx::query_as!(
        Comment,
        "
        SELECT
            post_id
        FROM comments
        WHERE
            id = ?
        ",
        comment_id,
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.map(|comment| comment.post_id))
}

async fn fetch_post_exists(db_pool: &MySqlPool, post_id: Id) -> sqlx::Result<bool> {
    sqlx::query!(
        "
        SELECT
            id
        FROM posts
        WHERE
            id = ?
        ",
        post_id
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.is_some())
}

impl From<sqlx::Error> for ErrorKind {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn new(kind: ErrorKind, post_id: Id, reply_to_comment_id: Option<Id>) -> Self {
        Self {
            kind,
            post_id,
            reply_to_comment_id,
        }
    }

    pub fn as_location(&self) -> String {
        let reply_to_comment_id = match self.reply_to_comment_id {
            Some(reply_to_comment_id) => format!("&reply_to_comment_id={reply_to_comment_id}"),
            None => String::new(),
        };

        format!(
            "/comment/create?error={self}&post_id={}{}",
            self.post_id, reply_to_comment_id
        )
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
