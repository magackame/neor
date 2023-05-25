use crate::db::fetch_post_preview_from_raw;
use crate::session::auth;
use crate::types::id::Id;
use crate::types::page::{clamp_limit, Direction};
use crate::types::post::{Preview as PostPreview, RawPreview as RawPostPreview};
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

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub direction: Option<Direction>,
    pub start_id: Option<Id>,
    pub limit: Option<u64>,
}

#[get("/tag/{tag}")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<String>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let tag = path.into_inner();

    let current_user = auth(&state.db_pool, &req).await.ok();

    let direction = query.direction.unwrap_or_default();
    let start_id = query.start_id.unwrap_or(i64::MAX as Id);
    let limit = clamp_limit(query.limit);

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../templates/tag/tag.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let (min_id, max_id) = fetch_min_max_post_id(&state.db_pool, &tag).await?;

    let raw_post_previews = match direction {
        Direction::Backwards => {
            let mut raw_post_previews =
                fetch_raw_posts_backwards(&state.db_pool, &tag, start_id, limit).await?;

            raw_post_previews.reverse();

            raw_post_previews
        }
        Direction::Forwards => {
            fetch_raw_posts_forwards(&state.db_pool, &tag, start_id, limit).await?
        }
    };

    let posts = fetch_posts_from_raw(&state.db_pool, raw_post_previews).await?;

    let prev_start_id = posts
        .first()
        .map(|post| post.id)
        .unwrap_or(u64::MAX)
        .saturating_add(1);

    let prev_start_id = if prev_start_id > i64::MAX as u64 {
        i64::MAX as u64
    } else {
        prev_start_id
    };

    let next_start_id = posts
        .last()
        .map(|post| post.id)
        .unwrap_or(0)
        .saturating_sub(1);

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "tag": tag,
        "posts": posts,
        "min_id": min_id,
        "max_id": max_id,
        "prev_start_id": prev_start_id,
        "next_start_id": next_start_id,
        "limit": limit,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_posts_from_raw(
    db_pool: &MySqlPool,
    raw_post_previews: Vec<RawPostPreview>,
) -> sqlx::Result<Vec<PostPreview>> {
    let mut post_previews = Vec::new();

    for raw_post_preview in raw_post_previews {
        let post_preview = fetch_post_preview_from_raw(db_pool, raw_post_preview).await?;

        post_previews.push(post_preview);
    }

    Ok(post_previews)
}

async fn fetch_raw_posts_forwards(
    db_pool: &MySqlPool,
    tag: &str,
    start_id: Id,
    limit: u64,
) -> sqlx::Result<Vec<RawPostPreview>> {
    sqlx::query_as!(
        RawPostPreview,
        "
        SELECT DISTINCT
            posts.id,
            posts.title,
            posts.description,
            users.id AS posted_by_user_id,
            users.username AS posted_by_username,
            CONCAT(files.id, \".\", files.extension) AS posted_by_mini_pfp,
            posts.posted_at
        FROM posts
            JOIN post_tags ON posts.id = post_tags.post_id
            JOIN tags ON post_tags.tag_id = tags.id
            LEFT JOIN users ON posts.posted_by_user_id = users.id
            LEFT JOIN files ON users.mini_pfp_file_id = files.id
        WHERE
            posts.id <= ?
            AND tags.name = ?
        ORDER BY posts.id DESC
        LIMIT ?
        ",
        start_id,
        tag,
        limit
    )
    .fetch_all(db_pool)
    .await
}

async fn fetch_raw_posts_backwards(
    db_pool: &MySqlPool,
    tag: &str,
    start_id: Id,
    limit: u64,
) -> sqlx::Result<Vec<RawPostPreview>> {
    sqlx::query_as!(
        RawPostPreview,
        "
        SELECT DISTINCT
            posts.id,
            posts.title,
            posts.description,
            users.id AS posted_by_user_id,
            users.username AS posted_by_username,
            CONCAT(files.id, \".\", files.extension) AS posted_by_mini_pfp,
            posts.posted_at
        FROM posts
            JOIN post_tags ON posts.id = post_tags.post_id
            JOIN tags ON post_tags.tag_id = tags.id
            LEFT JOIN users ON posts.posted_by_user_id = users.id
            LEFT JOIN files ON users.mini_pfp_file_id = files.id
        WHERE
            posts.id >= ?
            AND tags.name = ?
        ORDER BY posts.id ASC
        LIMIT ?
        ",
        start_id,
        tag,
        limit
    )
    .fetch_all(db_pool)
    .await
}

async fn fetch_min_max_post_id(db_pool: &MySqlPool, tag: &str) -> sqlx::Result<(Id, Id)> {
    #[derive(Debug)]
    struct MinMax {
        min: Option<Id>,
        max: Option<Id>,
    }

    sqlx::query_as!(
        MinMax,
        "
        SELECT 
            MIN(posts.id) AS min,
            MAX(posts.id) AS max
        FROM posts
            JOIN post_tags ON posts.id = post_tags.post_id
            JOIN tags ON post_tags.tag_id = tags.id
        WHERE
            tags.name = ?
        ",
        tag
    )
    .fetch_one(db_pool)
    .await
    .map(|result| {
        (
            result.min.unwrap_or(0),
            result.max.unwrap_or(i64::MAX as Id),
        )
    })
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
