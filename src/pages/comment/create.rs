use crate::db::{fetch_comment_by_id, fetch_post_preview_from_raw};
use crate::session::auth;
use crate::types::comment::content::CONTENT_MAX_CHAR_COUNT;
use crate::types::id::Id;
use crate::types::post::{Preview as PostPreview, RawPreview as RawPostPreview};
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    http::header,
    web::{Data, Query},
    HttpRequest, HttpResponse, ResponseError,
};
use lazy_static::lazy_static;
use liquid::Template;
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub post_id: u64,
    pub reply_to_comment_id: Option<u64>,
    pub error: Option<String>,
}

#[get("/comment/create")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let reply_to_comment_id = match query.reply_to_comment_id {
            Some(reply_to_comment_id) => format!("&reply_to_comment_id={reply_to_comment_id}"),
            None => String::new(),
        };

        let location = format!("/sign-in?back=/comment/create?post_id={}{}", query.post_id, reply_to_comment_id);

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../templates/comment/create.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let post = match fetch_post(&state.db_pool, query.post_id).await? {
        Some(post) => post,
        None => {
            let response = crate::pages::not_found::service().await;

            return Ok(response);
        }
    };
    let comment = match query.reply_to_comment_id {
        Some(reply_to_comment_id) => {
            Some(fetch_comment_by_id(&state.db_pool, reply_to_comment_id, &current_user).await?)
        }
        None => None,
    };

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "post": post,
        "comment": comment,
        "content_max_char_count": CONTENT_MAX_CHAR_COUNT,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_post(db_pool: &MySqlPool, post_id: Id) -> sqlx::Result<Option<PostPreview>> {
    let raw_post_preview = sqlx::query_as!(
        RawPostPreview,
        "
        SELECT
            posts.id,
            posts.title,
            posts.description,
            users.id AS posted_by_user_id,
            users.username AS posted_by_username,
            CONCAT(files.id, \".\", files.extension) AS posted_by_mini_pfp,
            posts.posted_at
        FROM posts
            LEFT JOIN users ON posts.posted_by_user_id = users.id
            LEFT JOIN files ON users.mini_pfp_file_id = files.id
        WHERE
            posts.id = ?
        ",
        post_id
    )
    .fetch_optional(db_pool)
    .await?;

    let Some(raw_post_preview) = raw_post_preview else {
        return Ok(None);
    };

    let post_preview = fetch_post_preview_from_raw(db_pool, raw_post_preview).await?;

    Ok(Some(post_preview))
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
