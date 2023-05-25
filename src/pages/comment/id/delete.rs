use crate::db::fetch_comment_by_id;
use crate::session::auth;
use crate::types::id::Id;
use crate::types::user::username::USERNAME_MAX_CHAR_COUNT;
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

#[get("/comment/{id}/delete")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<Id>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let comment_id = path.into_inner();

    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/comment/{comment_id}/delete");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let Some(comment) = fetch_comment_by_id(&state.db_pool, comment_id, &current_user).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    if !comment.is_deletable && query.error.is_none() {
        let location = format!(
            "/comment/{comment_id}/delete?error=You are not allowed to delete this comment"
        );

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    }

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/comment/id/delete.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "comment": comment,
        "username_max_char_count": USERNAME_MAX_CHAR_COUNT,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    Ok(HttpResponse::Ok().body(s))
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
