use crate::session::{auth, User};
use crate::types::comment::content::CONTENT_MAX_CHAR_COUNT;
use crate::types::comment::{Comment, RawComment};
use crate::types::id::Id;
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

#[get("/comment/{id}/edit")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<Id>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let current_user = auth(&state.db_pool, &req).await.ok();

    let comment_id = path.into_inner();

    let Some(current_user) = current_user else {
        let location = format!("/sign-in?back=/comment/{comment_id}/edit");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let Some(comment) = fetch_comment_by_id(&state.db_pool, comment_id, &current_user).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    if !comment.is_editable && query.error.is_none() {
        let location =
            format!("/comment/{comment_id}/edit?error=You are not allowed to edit this comment");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    }

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/comment/id/edit.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "content_max_char_count": CONTENT_MAX_CHAR_COUNT,
        "comment": comment,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

async fn fetch_comment_by_id(
    db_pool: &MySqlPool,
    comment_id: Id,
    user: &User,
) -> sqlx::Result<Option<Comment>> {
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
            comments.content AS content,
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
            comments.id = ?
        ",
        comment_id
    )
    .fetch_optional(db_pool)
    .await
    .map(|result| result.map(|raw_comment| Comment::from_raw(raw_comment, user)))
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
