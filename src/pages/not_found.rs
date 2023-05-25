use crate::LIQUID_PARSER;
use actix_web::HttpResponse;
use lazy_static::lazy_static;
use liquid::Template;

// TODO: Header, nav???
pub async fn service() -> HttpResponse {
    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../templates/not_found.html");

            LIQUID_PARSER.parse(template).unwrap()
        };
    }

    let globals = liquid::object!({});

    let s = TEMPLATE.render(&globals).unwrap();

    HttpResponse::NotFound().body(s)
}
