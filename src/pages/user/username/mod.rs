use crate::db::{fetch_post_preview_from_raw, fetch_user_by_username};
use crate::session::{auth, User as AuthUser};
use crate::types::comment::{Comment, RawComment};
use crate::types::id::Id;
use crate::types::page::{clamp_limit, Direction};
use crate::types::post::{Preview as PostPreview, RawPreview as RawPostPreview};
use crate::types::user::User;
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    web::{Data, Path, Query},
    HttpRequest, HttpResponse, ResponseError,
};
use lazy_static::lazy_static;
use liquid::Template;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use thiserror::Error;

pub mod admin;
pub mod edit;

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    Posts,
    Comments,
}

impl Default for Display {
    fn default() -> Self {
        Self::Posts
    }
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub display: Option<Display>,
    pub direction: Option<Direction>,
    pub start_id: Option<Id>,
    pub limit: Option<u64>,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[get("/user/{username}")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<String>,
    query: Query<Request>,
) -> Result<HttpResponse, Error> {
    let current_user = auth(&state.db_pool, &req).await.ok();

    let username = path.into_inner();

    let direction = query.direction.unwrap_or_default();
    let start_id = query.start_id.unwrap_or(i64::MAX as Id);
    let limit = clamp_limit(query.limit);

    let display = query.display.unwrap_or_default();

    let mut posts = Vec::new();
    let mut comments = Vec::new();

    let Some(user) = fetch_user_by_username(&state.db_pool, &username, current_user.as_ref()).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    let (min_id, max_id) = match display {
        Display::Posts => fetch_min_max_post_id(&state.db_pool, user.id).await?,
        Display::Comments => fetch_min_max_comment_id(&state.db_pool, user.id).await?,
    };

    match display {
        Display::Posts => {
            posts = fetch_posts(&state.db_pool, direction, start_id, limit, &user).await?;
        }
        Display::Comments => {
            comments = fetch_comments(
                &state.db_pool,
                direction,
                start_id,
                limit,
                &user,
                current_user.as_ref(),
            )
            .await?;
        }
    }

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/user/username.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let next_start_id = match display {
        Display::Posts => posts
            .last()
            .map(|post| post.id)
            .unwrap_or(0)
            .saturating_sub(1),
        Display::Comments => comments
            .last()
            .map(|comment| comment.id)
            .unwrap_or(0)
            .saturating_sub(1),
    };

    let prev_start_id = match display {
        Display::Posts => posts
            .first()
            .map(|post| post.id)
            .unwrap_or(u64::MAX)
            .saturating_add(1),
        Display::Comments => comments
            .first()
            .map(|comment| comment.id)
            .unwrap_or(u64::MAX)
            .saturating_add(1),
    };

    let prev_start_id = if prev_start_id > i64::MAX as u64 {
        i64::MAX as u64
    } else {
        prev_start_id
    };

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "user": user,
        "display": display,
        "posts": posts,
        "comments": comments,
        "start_id": start_id,
        "min_id": min_id,
        "prev_start_id": prev_start_id,
        "next_start_id": next_start_id,
        "max_id": max_id,
        "limit": limit,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_posts(
    db_pool: &MySqlPool,
    direction: Direction,
    start_id: Id,
    limit: u64,
    user: &User,
) -> sqlx::Result<Vec<PostPreview>> {
    let raw_post_previews = fetch_raw_posts(db_pool, direction, start_id, limit, user).await?;

    let mut post_previews = Vec::new();

    for raw_post in raw_post_previews {
        let post = fetch_post_preview_from_raw(db_pool, raw_post).await?;

        post_previews.push(post);
    }

    Ok(post_previews)
}

async fn fetch_raw_posts(
    db_pool: &MySqlPool,
    direction: Direction,
    start_id: Id,
    limit: u64,
    user: &User,
) -> sqlx::Result<Vec<RawPostPreview>> {
    match direction {
        Direction::Forwards => fetch_raw_posts_forwards(db_pool, start_id, limit, user).await,
        Direction::Backwards => fetch_raw_posts_backwards(db_pool, start_id, limit, user).await,
    }
}

async fn fetch_raw_posts_forwards(
    db_pool: &MySqlPool,
    start_id: Id,
    limit: u64,
    user: &User,
) -> sqlx::Result<Vec<RawPostPreview>> {
    sqlx::query_as!(
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
            posts.id <= ?
            AND posts.posted_by_user_id = ?
        ORDER BY posts.id DESC
        LIMIT ?
        ",
        start_id,
        user.id,
        limit
    )
    .fetch_all(db_pool)
    .await
}

async fn fetch_raw_posts_backwards(
    db_pool: &MySqlPool,
    start_id: Id,
    limit: u64,
    user: &User,
) -> sqlx::Result<Vec<RawPostPreview>> {
    sqlx::query_as!(
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
            posts.id >= ?
            AND posts.posted_by_user_id = ?
        ORDER BY posts.id ASC
        LIMIT ?
        ",
        start_id,
        user.id,
        limit
    )
    .fetch_all(db_pool)
    .await
    .map(|mut result| {
        result.reverse();

        result
    })
}

async fn fetch_comments(
    db_pool: &MySqlPool,
    direction: Direction,
    start_id: Id,
    limit: u64,
    user: &User,
    current_user: Option<&AuthUser>,
) -> sqlx::Result<Vec<Comment>> {
    match direction {
        Direction::Backwards => {
            fetch_comments_backwards(db_pool, start_id, limit, user, current_user).await
        }
        Direction::Forwards => {
            fetch_comments_forwards(db_pool, start_id, limit, user, current_user).await
        }
    }
}

async fn fetch_comments_forwards(
    db_pool: &MySqlPool,
    start_id: Id,
    limit: u64,
    user: &User,
    current_user: Option<&AuthUser>,
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
            comments.id <= ?
            AND comments.posted_by_user_id = ?
        ORDER BY comments.posted_at DESC
        LIMIT ?
        ",
        start_id,
        user.id,
        limit
        )
    .fetch_all(db_pool).await.map(|result| result.into_iter().map(|raw_comment| Comment::from_raw(raw_comment, current_user)).collect())
}

async fn fetch_comments_backwards(
    db_pool: &MySqlPool,
    start_id: Id,
    limit: u64,
    user: &User,
    current_user: Option<&AuthUser>,
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
            comments.id >= ?
            AND comments.posted_by_user_id = ?
        ORDER BY comments.posted_at ASC
        LIMIT ?
        ",
        start_id,
        user.id,
        limit
        )
    .fetch_all(db_pool).await.map(|result| result.into_iter().map(|raw_comment| Comment::from_raw(raw_comment, current_user)).rev().collect())
}

async fn fetch_min_max_post_id(
    db_pool: &MySqlPool,
    user_id: Id,
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
        FROM posts
        WHERE
            posted_by_user_id = ?
        ",
        user_id,
    )
    .fetch_one(db_pool)
    .await
    .map(|result| (result.min, result.max))
}

async fn fetch_min_max_comment_id(
    db_pool: &MySqlPool,
    user_id: Id,
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
            posted_by_user_id = ?
        ",
        user_id,
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
