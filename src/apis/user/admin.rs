use crate::apis::is_checked;
use crate::session::auth;
use crate::session::User;
use crate::types::user::role::Role as UserRole;
use crate::State;
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Form},
    HttpRequest, HttpResponse, ResponseError,
};
use serde::Deserialize;
use sqlx::mysql::{MySqlPool, MySqlQueryResult};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
    pub username: String,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum ErrorKind {
    #[error("You are not allowed to admin this user")]
    UserCantAdmin,
    #[error("Invalid role")]
    InvalidRole,
    #[error("Server error")]
    Server,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub username: String,

    pub reset_name: Option<String>,
    pub reset_description: Option<String>,
    pub reset_pfp: Option<String>,

    pub role: UserRole,
}

#[post("/api/user/admin")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    Form(request): Form<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/user/{}/admin", request.username.as_str());

        let response = HttpResponse::SeeOther()
            .append_header((header::LOCATION, location))
            .finish();

        return Ok(response);
    };

    let username = request.username.clone();

    service_inner(state, request, user)
        .await
        .map_err(|err| Error::new(err, username))
}

async fn service_inner(
    state: Data<State>,
    request: Request,
    user: User,
) -> Result<HttpResponse, ErrorKind> {
    if !user.role.can_admin() {
        return Err(ErrorKind::UserCantAdmin);
    }

    if let UserRole::Admin | UserRole::Unverified = request.role {
        return Err(ErrorKind::InvalidRole);
    }

    update_user_role(&state.db_pool, request.role, &request.username).await?;

    if is_checked(request.reset_name) {
        reset_user_name(&state.db_pool, &request.username).await?;
    }

    if is_checked(request.reset_description) {
        reset_user_description(&state.db_pool, &request.username).await?;
    }

    if is_checked(request.reset_pfp) {
        reset_user_pfp(&state.db_pool, &request.username).await?;
    }

    let location = format!("/user/{}", request.username);

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn reset_user_name(db_pool: &MySqlPool, username: &str) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            name = ''
        WHERE
            username = ?
        ",
        username
    )
    .execute(db_pool)
    .await
}

async fn reset_user_description(
    db_pool: &MySqlPool,
    username: &str,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            description = ''
        WHERE
            username = ?
        ",
        username
    )
    .execute(db_pool)
    .await
}

async fn reset_user_pfp(db_pool: &MySqlPool, username: &str) -> sqlx::Result<MySqlQueryResult> {
    // TODO: Delete old pfps
    sqlx::query!(
        "
        UPDATE users
        SET
            pfp_file_id = NULL,
            mini_pfp_file_id = NULL
        WHERE
            username = ?
        ",
        username
    )
    .execute(db_pool)
    .await
}

async fn update_user_role(
    db_pool: &MySqlPool,
    role: UserRole,
    username: &str,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            role = ?
        WHERE
            username = ?
        ",
        role.as_str(),
        username
    )
    .execute(db_pool)
    .await
}

impl From<sqlx::Error> for ErrorKind {
    fn from(_: sqlx::Error) -> Self {
        Self::Server
    }
}

impl Error {
    pub fn new(kind: ErrorKind, username: String) -> Self {
        Self { kind, username }
    }

    pub fn as_location(&self) -> String {
        format!("/user/{}/admin?error={self}", self.username)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .append_header((header::LOCATION, self.as_location()))
            .finish()
    }
}
