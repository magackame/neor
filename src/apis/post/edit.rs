use crate::db::insert_post_tags;
use crate::session::auth;
use crate::session::User;
use crate::types::id::Id;
use crate::types::post::{content::Content, description::Description, tags::Tags, title::Title};
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
    #[error("You are not allowed to edit this post")]
    UserCantEditPost,
    #[error("Invalid title")]
    InvalidTitle,
    #[error("Invalid description")]
    InvalidDescription,
    #[error("Invalid tags")]
    InvalidTags,
    #[error("Invalid content")]
    InvalidContent,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub tags: String,
    pub content: String,
}

#[post("/api/post/edit")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/post/{}/edit", request.id);

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let post_id = request.id;

    service_inner(state, user, request)
        .await
        .map_err(|err| Error::new(err, post_id))
}

async fn service_inner(
    state: Data<State>,
    user: User,
    request: Request,
) -> Result<HttpResponse, ErrorKind> {
    if !user.role.can_edit_posts() {
        return Err(ErrorKind::UserCantEditPost);
    }

    let title = Title::parse(request.title).map_err(|_| ErrorKind::InvalidTitle)?;
    let description =
        Description::parse(request.description).map_err(|_| ErrorKind::InvalidDescription)?;
    let tags = Tags::parse(request.tags).map_err(|_| ErrorKind::InvalidTags)?;
    let content = Content::parse(request.content).map_err(|_| ErrorKind::InvalidContent)?;

    let update_post_result = update_post(
        &state.db_pool,
        request.id,
        &title,
        &description,
        &content,
        &user,
    )
    .await?;

    if update_post_result.rows_affected() == 0 {
        return Err(ErrorKind::UserCantEditPost);
    }

    update_post_tags(&state.db_pool, &tags, request.id, user.id).await?;

    let location = format!("/post/{}", request.id);

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn update_post(
    db_pool: &MySqlPool,
    id: Id,
    title: &Title,
    description: &Description,
    content: &Content,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    let markdown_content = markdown::to_html(content.as_ref());

    sqlx::query!(
        "
        UPDATE posts
        SET
            title = ?,
            description = ?,
            content = ?,
            markdown_content = ?,
            modified_at = NOW()
        WHERE
            id = ?
            AND posted_by_user_id = ?
        ",
        title.as_ref(),
        description.as_ref(),
        content.as_ref(),
        markdown_content,
        id,
        user.id
    )
    .execute(db_pool)
    .await
}

async fn update_post_tags(
    db_pool: &MySqlPool,
    tags: &Tags,
    post_id: Id,
    user_id: Id,
) -> sqlx::Result<()> {
    delete_post_tags(db_pool, post_id).await?;
    insert_post_tags(db_pool, tags, post_id, user_id).await?;

    Ok(())
}

async fn delete_post_tags(db_pool: &MySqlPool, id: Id) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        DELETE FROM post_tags
        WHERE
            post_id = ?
        ",
        id
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
        format!("/post/{}/edit?error={self}", self.post_id)
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
