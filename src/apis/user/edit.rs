use crate::session::auth;
use crate::session::User;
use crate::types::id::Id;
use crate::types::user::{description::Description, name::Name};
use crate::State;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{
    http::{header, StatusCode},
    post,
    web::Data,
    HttpRequest, HttpResponse, ResponseError,
};
use sqlx::mysql::{MySqlPool, MySqlQueryResult};
use std::path::PathBuf;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Clone, Error)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
    pub username: String,
}

#[derive(Debug, Error, Copy, Clone)]
pub enum ErrorKind {
    #[error("You are not allowed to edit this user")]
    UserCantEditUser,
    #[error("Invalid name")]
    InvalidName,
    #[error("Invalid description")]
    InvalidDescription,
    #[error("Invalid profile picture")]
    InvalidPfp,
    #[error("Server error")]
    Server,
}

#[derive(Debug, MultipartForm)]
pub struct Request {
    pub username: Text<String>,

    pub name: Text<String>,
    pub description: Text<String>,
    pub pfp: TempFile,
}

#[post("/api/user/edit")]
pub async fn service(
    state: Data<State>,
    req: HttpRequest,
    MultipartForm(request): MultipartForm<Request>,
) -> Result<HttpResponse, Error> {
    let Ok(user) = auth(&state.db_pool, &req).await else {
        let location = format!("/sign-in?back=/user/{}/edit", request.username.as_str());

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
    let Text(username) = request.username;
    let Text(name) = request.name;
    let Text(description) = request.description;

    if !user.role.can_edit_self() {
        return Err(ErrorKind::UserCantEditUser);
    }

    if user.username != username {
        return Err(ErrorKind::UserCantEditUser);
    }

    let name = Name::parse(name).map_err(|_| ErrorKind::InvalidName)?;
    let description = Description::parse(description).map_err(|_| ErrorKind::InvalidDescription)?;

    if request.pfp.size != 0 {
        // TODO: replace hardcoded dimensions
        let mini_pfp_file_id = resize_and_persist_image(&state.db_pool, 32, &request.pfp, &user)
            .await
            .map_err(|_| ErrorKind::InvalidPfp)?;

        let pfp_file_id = resize_and_persist_image(&state.db_pool, 128, &request.pfp, &user)
            .await
            .map_err(|_| ErrorKind::InvalidPfp)?;

        update_user_pfps(
            &state.db_pool,
            Some(mini_pfp_file_id),
            Some(pfp_file_id),
            &username,
        )
        .await?;
    }

    update_user(&state.db_pool, &name, &description, &username).await?;

    let location = format!("/user/{}", username);

    let response = HttpResponse::SeeOther()
        .append_header((header::LOCATION, location))
        .finish();

    Ok(response)
}

async fn resize_and_persist_image(
    db_pool: &MySqlPool,
    width: u64,
    file: &TempFile,
    user: &User,
) -> Result<Id, ()> {
    let file_id = insert_file(db_pool, file, user)
        .await
        .map_err(|_| ())?
        .last_insert_id();

    let path = file.file.path();

    let mut command = Command::new("convert");

    let mut save_path = PathBuf::new();

    save_path.push("public");
    save_path.push("files");
    // TODO: replace hardcoded `png`
    // TODO: Cleanup previous pfp
    save_path.push(format!("{file_id}.png"));

    command
        .arg(format!("-resize"))
        .arg(format!("{width}x"))
        .arg(&path)
        .arg(&save_path);

    if !command.status().await.map_err(|_| ())?.success() {
        return Err(());
    }

    Ok(file_id)
}

async fn insert_file(
    db_pool: &MySqlPool,
    file: &TempFile,
    user: &User,
) -> sqlx::Result<MySqlQueryResult> {
    // TODO: replace hardcoded `png` extensinon
    sqlx::query!(
        "
        INSERT INTO files
        (
            filename,
            extension,
            size,
            uploaded_by_user_id,
            uploaded_at
        )
        VALUES
        (
            ?,
            'png',
            ?,
            ?,
            NOW()
        )
        ",
        file.file_name,
        file.size as u64,
        user.id,
    )
    .execute(db_pool)
    .await
}

async fn update_user(
    db_pool: &MySqlPool,
    name: &Name,
    description: &Description,
    username: &str,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            name = ?,
            description = ?
        WHERE
            username = ?
        ",
        name.as_ref(),
        description.as_ref(),
        username
    )
    .execute(db_pool)
    .await
}

async fn update_user_pfps(
    db_pool: &MySqlPool,
    mini_pfp_file_id: Option<Id>,
    pfp_file_id: Option<Id>,
    username: &str,
) -> sqlx::Result<MySqlQueryResult> {
    sqlx::query!(
        "
        UPDATE users
        SET
            mini_pfp_file_id = ?,
            pfp_file_id = ?
        WHERE
            username = ?
        ",
        mini_pfp_file_id,
        pfp_file_id,
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
        format!("/user/{}/edit?error={self}", self.username)
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
