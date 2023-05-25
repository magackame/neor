use crate::types::user::email::EMAIL_MAX_CHAR_COUNT;
use crate::LIQUID_PARSER;
use actix_web::{get, web::Query, HttpRequest, HttpResponse};
use lazy_static::lazy_static;
use liquid::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub error: Option<String>,
}

#[get("/password-reset")]
pub async fn service(req: HttpRequest, Query(query): Query<Request>) -> HttpResponse {
    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../templates/password-reset.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "email_max_char_count": EMAIL_MAX_CHAR_COUNT,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    HttpResponse::Ok().body(s)
}
