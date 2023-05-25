use crate::db::fetch_post_from_raw;
use crate::session::{auth, User};
use crate::types::id::Id;
use crate::types::post::{
    content::CONTENT_MAX_CHAR_COUNT, description::DESCRIPTION_MAX_CHAR_COUNT,
    tags::TAGS_MAX_CHAR_COUNT, title::TITLE_MAX_CHAR_COUNT,
};
use crate::types::post::{Post, RawPost};
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    http::header,
    web::{Data, Path, Query},
    HttpRequest, HttpResponse, ResponseError,
};
use lazy_static::lazy_static;
use liquid::Template;
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub error: Option<String>,
}

#[get("/post/{id}/edit")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<Id>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let post_id = path.into_inner();

    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/post/{post_id}/edit");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let Some(post) = fetch_post_by_id(&state.db_pool, post_id, &current_user).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    if !post.is_editable && query.error.is_none() {
        let location = format!("/post/{post_id}/edit?error=You are not allowed to edit this post");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    }

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/post/id/edit.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "post": post,
        "title_max_char_count": TITLE_MAX_CHAR_COUNT,
        "description_max_char_count": DESCRIPTION_MAX_CHAR_COUNT,
        "content_max_char_count": CONTENT_MAX_CHAR_COUNT,
        "tags_max_char_count": TAGS_MAX_CHAR_COUNT,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_post_by_id(
    db_pool: &MySqlPool,
    post_id: Id,
    user: &User,
) -> sqlx::Result<Option<Post>> {
    let Some(raw_post) = fetch_raw_post_by_id(db_pool, post_id).await? else {
        return Ok(None);
    };

    let post = fetch_post_from_raw(db_pool, raw_post, user).await?;

    Ok(Some(post))
}

async fn fetch_raw_post_by_id(db_pool: &MySqlPool, post_id: Id) -> sqlx::Result<Option<RawPost>> {
    sqlx::query_as!(
        RawPost,
        "
        SELECT
            posts.id,
            posts.title,
            posts.description,
            posts.content,
            users.id AS posted_by_user_id,
            users.username AS posted_by_username,
            CONCAT(files.id, \".\", files.extension) AS posted_by_mini_pfp,
            posts.posted_at,
            posts.modified_at
        FROM posts
            LEFT JOIN users ON posts.posted_by_user_id = users.id
            LEFT JOIN files ON users.mini_pfp_file_id = files.id
        WHERE
            posts.id = ?
        ",
        post_id
    )
    .fetch_optional(db_pool)
    .await
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
