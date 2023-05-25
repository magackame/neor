use crate::db::insert_post_tags;
use crate::session::auth;
use crate::session::User;
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
pub enum Error {
    #[error("You are not allowed to post")]
    UserCantPost,
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
    pub title: String,
    pub description: String,
    pub tags: String,
    pub content: String,
}

#[post("/api/post/create")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let user = match auth(&state.db_pool, &req).await {
        Ok(user) => user,
        Err(_) => {
            let response = HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/sign-in?back=/post/create"))
                .finish();

            return Ok(response);
        }
    };

    if !user.role.can_post() {
        return Err(Error::UserCantPost);
    }

    let title = Title::parse(request.title).map_err(|_| Error::InvalidTitle)?;
    let description =
        Description::parse(request.description).map_err(|_| Error::InvalidDescription)?;
    let tags = Tags::parse(request.tags).map_err(|_| Error::InvalidTags)?;
    let content = Content::parse(request.content).map_err(|_| Error::InvalidContent)?;

    let post_insert_result =
        insert_post(&state.db_pool, &title, &description, &content, &user).await?;

    insert_post_tags(
        &state.db_pool,
        &tags,
        post_insert_result.last_insert_id(),
        user.id,
    )
    .await?;

    let location = format!("/post/{}", post_insert_result.last_insert_id());

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn insert_post(
    db_pool: &MySqlPool,
    title: &Title,
    description: &Description,
    content: &Content,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    let markdown_content = markdown::to_html(content.as_ref());

    sqlx::query!(
        "
        INSERT INTO posts
        (
            title,
            description,
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
        title.as_ref(),
        description.as_ref(),
        content.as_ref(),
        markdown_content,
        user.id
    )
    .execute(db_pool)
    .await
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn as_location(&self) -> String {
        format!("/post/create?error={self}")
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
