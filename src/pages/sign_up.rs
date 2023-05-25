use crate::session::auth;
use crate::types::user::{
    email::EMAIL_MAX_CHAR_COUNT, password::PASSWORD_MAX_CHAR_COUNT,
    username::USERNAME_MAX_CHAR_COUNT,
};
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    web::{Data, Query},
    HttpRequest, HttpResponse,
};
use lazy_static::lazy_static;
use liquid::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub error: Option<String>,
}

#[get("/sign-up")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    Query(query): Query<Request>,
) -> HttpResponse {
    let current_user = auth(&state.db_pool, &req).await.ok();

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../templates/sign-up.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "error": query.error,
        "username_max_char_count": USERNAME_MAX_CHAR_COUNT,
        "email_max_char_count": EMAIL_MAX_CHAR_COUNT,
        "password_max_char_count": PASSWORD_MAX_CHAR_COUNT,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    HttpResponse::Ok().body(s)
}
