use crate::theme::{GetThemeCookie, SetThemeCookie};
use actix_web::{get, http::header, web::Query, HttpRequest, HttpResponse};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub back: Option<String>,
}

#[get("/switch-theme")]
pub async fn service(req: HttpRequest, Query(query): Query<Request>) -> HttpResponse {
    let switched_theme = req.get_theme().switch();

    let location = query.back.unwrap_or("/".to_owned());

    HttpResponse::SeeOther()
        .set_theme(switched_theme)
        .append_header((header::LOCATION, location))
        .finish()
}
