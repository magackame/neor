use crate::session::RemoveSessionCookie;
use crate::session::{auth, User};
use crate::types::user::session::Session;
use crate::State;
use actix_web::{get, http::header, web::Data, HttpRequest, HttpResponse, ResponseError};
use sqlx::mysql::{MySqlPool, MySqlQueryResult};
use thiserror::Error;

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("Server error")]
    Server,
}

#[get("/sign-out")]
pub async fn service(req: HttpRequest, state: Data<State>) -> Result<HttpResponse, Error> {
    let Ok(current_user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in");

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let session = Session::new(&state.db_pool).await?;

    set_user_session(&state.db_pool, &session, &current_user).await?;

    let response = HttpResponse::SeeOther()
        .remove_session()
        .append_header((header::LOCATION, "/sign-in"))
        .finish();

    Ok(response)
}

async fn set_user_session(
    db_pool: &MySqlPool,
    session: &Session,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            session = ?
        WHERE
            id = ?
        ",
        session.as_ref(),
        user.id
    )
    .execute(db_pool)
    .await
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl ResponseError for Error {}
