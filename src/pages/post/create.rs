use crate::session::auth;
use crate::types::post::{
    content::CONTENT_MAX_CHAR_COUNT, description::DESCRIPTION_MAX_CHAR_COUNT,
    tags::TAGS_MAX_CHAR_COUNT, title::TITLE_MAX_CHAR_COUNT,
};
use crate::State;
use crate::LIQUID_PARSER;
use actix_web::{
    get,
    http::header,
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

#[get("/post/create")]
pub async fn service(
    req: HttpRequest,
    state: Data<State>,
    Query(query): Query<Request>,
) -> HttpResponse {
    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/sign-in?back=/post/create"))
            .finish();

        return response;
    };

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../../templates/post/create.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let current_url = urlencoding::encode(&req.uri().to_string()).into_owned();

    let globals = liquid::object!({
        "current_url": current_url,
        "current_user": current_user,
        "title_max_char_count": TITLE_MAX_CHAR_COUNT,
        "description_max_char_count": DESCRIPTION_MAX_CHAR_COUNT,
        "content_max_char_count": CONTENT_MAX_CHAR_COUNT,
        "tags_max_char_count": TAGS_MAX_CHAR_COUNT,
        "error": query.error,
    });

    let s = TEMPLATE.render(&globals).unwrap();

    HttpResponse::Ok().body(s)
}
