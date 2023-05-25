use crate::db::fetch_post_from_raw;
use crate::session::auth;
use crate::session::User;
use crate::types::comment::{Comment, RawComment};
use crate::types::id::Id;
use crate::types::page::{clamp_limit, Direction};
use crate::types::post::{Post, RawPost};
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    web::{Data, Path, Query},
    HttpRequest, HttpResponse, ResponseError,
};
use lazy_static::lazy_static;
use liquid::Template;
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use thiserror::Error;

pub mod anonymise;
pub mod delete;
pub mod edit;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub direction: Option<Direction>,
    pub start_id: Option<Id>,
    pub limit: Option<u64>,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[get("/post/{id}")]
pub async fn service(
    req: HttpRequest,
    path: Path<Id>,
    Query(query): Query<Request>,
    state: Data<State>,
) -> Result<HttpResponse, Error> {
    let current_user = auth(&state.db_pool, &req).await.ok();

    let post_id = path.into_inner();

    let direction = query.direction.unwrap_or_default();
    let start_id = query.start_id.unwrap_or(0);
    let limit = clamp_limit(query.limit);

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/post/id.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    // TODO: Make all requests concurrent
    let (min_id, max_id) = fetch_min_max_comment_id(&state.db_pool, post_id).await?;

    let Some(post) = fetch_post_by_id(&state.db_pool, post_id, current_user.as_ref()).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    let comments = match direction {
        Direction::Backwards => {
            fetch_comments_backwards(
                &state.db_pool,
                post_id,
                start_id,
                limit,
                current_user.as_ref(),
            )
            .await?
        }
        Direction::Forwards => {
            fetch_comments_forwards(
                &state.db_pool,
                post.id,
                start_id,
                limit,
                current_user.as_ref(),
            )
            .await?
        }
    };

    let prev_start_id = comments
        .first()
        .map(|comment| comment.id)
        .unwrap_or(0)
        .saturating_sub(1);

    let next_start_id = comments
        .last()
        .map(|comment| comment.id)
        .unwrap_or(u64::MAX)
        .saturating_add(1);

    let next_start_id = if next_start_id > i64::MAX as u64 {
        i64::MAX as u64
    } else {
        next_start_id
    };

    let current_url =
        format!("/post/{post_id}?direction={direction}%26start_id={start_id}%26limit={limit}");

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "post": post,
        "comments": comments,
        "min_id": min_id,
        "prev_start_id": prev_start_id,
        "next_start_id": next_start_id,
        "max_id": max_id,
        "limit": limit,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_post_by_id(
    db_pool: &MySqlPool,
    post_id: Id,
    user: Option<&User>,
) -> sqlx::Result<Option<Post>> {
    let Some(raw_post) = fetch_raw_post_by_id(db_pool, post_id).await? else {
        return Ok(None);
    };

    let post = fetch_post_from_raw(db_pool, raw_post, user).await?;

    Ok(Some(post))
}

async fn fetch_raw_post_by_id(db_pool: &MySqlPool, id: Id) -> sqlx::Result<Option<RawPost>> {
    sqlx::query_as!(
        RawPost,
        "
        SELECT
            posts.id,
            posts.title,
            posts.description,
            posts.markdown_content AS content,
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
        id
    )
    .fetch_optional(db_pool)
    .await
}

async fn fetch_comments_forwards(
    db_pool: &MySqlPool,
    post_id: Id,
    start_id: Id,
    limit: u64,
    user: Option<&User>,
) -> sqlx::Result<Vec<Comment>> {
    sqlx::query_as!(
        RawComment,
        "
        SELECT
            comments.id,
            comments.post_id,
            comments.reply_to_comment_id,
            users_reply_to.id AS reply_to_user_id,
            users_reply_to.username AS reply_to_username,
            CONCAT(files_reply_to.id, \".\", files_reply_to.extension) AS reply_to_mini_pfp,
            comments.markdown_content AS content,
            users_posted_by.id AS posted_by_user_id,
            users_posted_by.username AS posted_by_username,
            CONCAT(files_posted_by.id, \".\", files_posted_by.extension) AS posted_by_mini_pfp,
            comments.posted_at,
            comments.modified_at
        FROM comments
            LEFT JOIN comments AS comments_reply_to ON comments.reply_to_comment_id = comments_reply_to.id
            LEFT JOIN users AS users_reply_to ON comments_reply_to.posted_by_user_id = users_reply_to.id
            LEFT JOIN files AS files_reply_to ON users_reply_to.mini_pfp_file_id = files_reply_to.id
            LEFT JOIN users AS users_posted_by ON comments.posted_by_user_id = users_posted_by.id
            LEFT JOIN files AS files_posted_by ON users_posted_by.mini_pfp_file_id = files_posted_by.id
        WHERE
            comments.post_id = ?
            AND comments.id >= ?
        ORDER BY comments.posted_at ASC
        LIMIT ?
        ",
        post_id,
        start_id,
        limit
    )
    .fetch_all(db_pool).await.map(|result| result.into_iter().map(|raw_comment| Comment::from_raw(raw_comment, user)).collect())
}

async fn fetch_comments_backwards(
    db_pool: &MySqlPool,
    post_id: Id,
    start_id: Id,
    limit: u64,
    user: Option<&User>,
) -> sqlx::Result<Vec<Comment>> {
    sqlx::query_as!(
        RawComment,
        "
        SELECT
            comments.id,
            comments.post_id,
            comments.reply_to_comment_id,
            users_reply_to.id AS reply_to_user_id,
            users_reply_to.username AS reply_to_username,
            CONCAT(files_reply_to.id, \".\", files_reply_to.extension) AS reply_to_mini_pfp,
            comments.markdown_content AS content,
            users_posted_by.id AS posted_by_user_id,
            users_posted_by.username AS posted_by_username,
            CONCAT(files_posted_by.id, \".\", files_posted_by.extension) AS posted_by_mini_pfp,
            comments.posted_at,
            comments.modified_at
        FROM comments
            LEFT JOIN comments AS comments_reply_to ON comments.reply_to_comment_id = comments_reply_to.id
            LEFT JOIN users AS users_reply_to ON comments_reply_to.posted_by_user_id = users_reply_to.id
            LEFT JOIN files AS files_reply_to ON users_reply_to.mini_pfp_file_id = files_reply_to.id
            LEFT JOIN users AS users_posted_by ON comments.posted_by_user_id = users_posted_by.id
            LEFT JOIN files AS files_posted_by ON users_posted_by.mini_pfp_file_id = files_posted_by.id
        WHERE
            comments.post_id = ?
            AND comments.id <= ?
        ORDER BY comments.posted_at DESC
        LIMIT ?
        ",
        post_id,
        start_id,
        limit
    )
    .fetch_all(db_pool).await.map(|result| result.into_iter().rev().map(|raw_comment| Comment::from_raw(raw_comment, user)).collect())
}

async fn fetch_min_max_comment_id(
    db_pool: &MySqlPool,
    post_id: Id,
) -> sqlx::Result<(Option<Id>, Option<Id>)> {
    #[derive(Debug)]
    struct MinMax {
        min: Option<Id>,
        max: Option<Id>,
    }

    sqlx::query_as!(
        MinMax,
        "
        SELECT
            MIN(id) AS min,
            MAX(id) AS max
        FROM comments
        WHERE
            post_id = ?
        ",
        post_id
    )
    .fetch_one(db_pool)
    .await
    .map(|result| (result.min, result.max))
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
