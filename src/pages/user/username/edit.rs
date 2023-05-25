use crate::db::fetch_user_by_username;
use crate::session::auth;
use crate::types::user::{description::DESCRIPTION_MAX_CHAR_COUNT, name::NAME_MAX_CHAR_COUNT};
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

#[get("/user/{username}/edit")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    path: Path<String>,
    Query(query): Query<Request>,
) -> Result<HttpResponse, Error> {
    let username = path.into_inner();

    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/user/{username}/edit");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let Some(user) = fetch_user_by_username(&state.db_pool, &username, &current_user).await? else {
        return Ok(crate::pages::not_found::service().await);
    };

    if !user.is_editable && query.error.is_none() {
        let location =
            format!("/user/{username}/edit?error=You are not allowed to edit this profile");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    }

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../../templates/user/username/edit.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "user": user,
        "name_max_char_count": NAME_MAX_CHAR_COUNT,
        "description_max_char_count": DESCRIPTION_MAX_CHAR_COUNT,
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
