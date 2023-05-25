use crate::theme::GetThemeCookie;
use actix_web::{get, HttpRequest, HttpResponse};
use lazy_static::lazy_static;
use liquid::Template;

#[get("/style.css")]
pub async fn service(request: HttpRequest) -> HttpResponse {
    let theme = request.get_theme();

    lazy_static! {
        static ref TEMPLATE: Template = {
            let template = include_str!("../../style.css");

            liquid::ParserBuilder::with_stdlib()
                .build()
                .unwrap()
                .parse(template)
                .unwrap()
        };
    }

    let globals = liquid::object!({
        "body-background-color": theme.body_background_color(),
        "main-font-color": theme.main_font_color(),
        "link-color": theme.link_color(),
        "author-font-color": theme.author_font_color(),
        "tag-background-color": theme.tag_background_color(),
        "tag-color": theme.tag_color(),
        "link-hover-color": theme.link_hover_color(),
    });

    let s = TEMPLATE.render(&globals).unwrap();

    HttpResponse::Ok().body(s)
}
