use actix_web::{
    web::{route, Data},
    App, HttpServer,
};
use dotenvy::dotenv;
use lazy_static::lazy_static;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::AsyncSmtpTransport;
use lettre::Tokio1Executor;
use sqlx::mysql::MySqlPool;

mod apis;
mod db;
mod pages;
mod session;
mod theme;
mod types;

lazy_static! {
    static ref LIQUID_PARSER: liquid::Parser = {
        let mut sources = liquid::partials::InMemorySource::new();

        sources.add("nav", include_str!("../partials/nav.html"));
        sources.add(
            "post_preview",
            include_str!("../partials/post_preview.html"),
        );
        sources.add("comment", include_str!("../partials/comment.html"));

        let partials = liquid::partials::EagerCompiler::new(sources);

        liquid::ParserBuilder::with_stdlib()
            .partials(partials)
            .build()
            .unwrap()
    };
}

#[derive(Debug)]
pub struct State {
    domain: String,
    email_from: Mailbox,
    mailer: AsyncSmtpTransport<Tokio1Executor>,

    db_pool: MySqlPool,
}

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load `.env` file");

    let port = std::env::var("PORT")
        .expect("Evnironment variable $PORT is not set")
        .parse::<u16>()
        .expect("Environment variable $PORT must be a `u16`");

    let domain = std::env::var("DOMAIN").expect("Evnironment variable $DOMAIN is not set");
    let email_from =
        std::env::var("EMAIL_FROM").expect("Evnironment variable $EMAIL_FROM is not set");
    let from = email_from
        .parse::<Mailbox>()
        .expect("$EMAIL_FROM contains an invalid email address");

    // TODO: Check that email works
    let email_password =
        std::env::var("EMAIL_PASSWORD").expect("Evnironment variable $EMAIL_PASSWORD is not set");
    let email_relay =
        std::env::var("EMAIL_RELAY").expect("Evnironment variable $EMAIL_RELAY is not set");

    let email_credentials = Credentials::new(email_from, email_password);

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&email_relay)
        .expect("Failed to create email transport")
        .credentials(email_credentials)
        .build::<Tokio1Executor>();

    let db_connection_string =
        std::env::var("DATABASE_URL").expect("Environment variable $DATABASE_URL is not set");

    let db_pool = MySqlPool::connect(&db_connection_string)
        .await
        .expect("Failed to connect to DB");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(State {
                domain: domain.clone(),
                email_from: from.clone(),
                mailer: mailer.clone(),

                db_pool: db_pool.clone(),
            }))
            .service(apis::user::admin::service)
            .service(apis::password_reset::service)
            .service(apis::password_change::service)
            .service(apis::sign_up::service)
            .service(apis::email_verification::service)
            .service(apis::post::anonymise::service)
            .service(apis::comment::anonymise::service)
            .service(apis::post::delete::service)
            .service(apis::comment::delete::service)
            .service(apis::sign_in::service)
            .service(apis::post::create::service)
            .service(apis::comment::create::service)
            .service(apis::post::edit::service)
            .service(apis::user::edit::service)
            .service(apis::comment::edit::service)
            .service(pages::files::service)
            .service(pages::style::service)
            .service(pages::switch_theme::service)
            .service(pages::index::service)
            .service(pages::post::create::service)
            .service(pages::tag::tag::service)
            .service(pages::comment::id::edit::service)
            .service(pages::comment::id::delete::service)
            .service(pages::post::id::anonymise::service)
            .service(pages::post::id::delete::service)
            .service(pages::post::id::edit::service)
            .service(pages::user::username::edit::service)
            .service(pages::user::username::admin::service)
            .service(pages::post::id::service)
            .service(pages::user::username::service)
            .service(pages::comment::create::service)
            .service(pages::comment::id::anonymise::service)
            .service(pages::sign_up::service)
            .service(pages::email_verification::service)
            .service(pages::sign_in::service)
            .service(pages::sign_out::service)
            .service(pages::password_reset::service)
            .service(pages::password_change::service)
            .default_service(route().to(pages::not_found::service))
    })
    .bind(("0.0.0.0", port))
    .expect("Failed to bind to socket")
    .run()
    .await
    .expect("Failed to run the server");
}
