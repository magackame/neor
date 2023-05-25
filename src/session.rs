use crate::types::user::role::Role as UserRole;
use crate::types::{default_mini_pfp, id::Id};
use actix_web::{cookie::Cookie, HttpRequest, HttpResponseBuilder};
use serde::Serialize;
use sqlx::mysql::MySqlPool;

pub const SESSION_COOKIE_NAME: &str = "session";

pub trait GetSessionCookie {
    fn get_session(&self) -> Option<String>;
}

pub trait SetSessionCookie {
    fn set_session(&mut self, domain: &str, session: &str, remember_me: bool) -> &mut Self;
}

pub trait RemoveSessionCookie {
    fn remove_session(&mut self) -> &mut Self;
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Id,
    pub username: String,
    pub role: UserRole,
    pub mini_pfp: String,
}

pub async fn auth(db_pool: &MySqlPool, req: &HttpRequest) -> Result<User, ()> {
    let session = req.get_session().ok_or(())?;

    let raw_user = RawUser::fetch_by_session(db_pool, &session)
        .await
        .map_err(|_| ())?
        .ok_or(())?;

    let user = User::from_raw(raw_user)?;

    Ok(user)
}

impl GetSessionCookie for HttpRequest {
    fn get_session(&self) -> Option<String> {
        self.cookie(SESSION_COOKIE_NAME)
            .map(|cookie| cookie.value().to_owned())
    }
}

impl SetSessionCookie for HttpResponseBuilder {
    fn set_session<'c>(&mut self, domain: &str, session: &str, remember_me: bool) -> &mut Self {
        let cookie = Cookie::build(SESSION_COOKIE_NAME, session)
            // .secure(true)
            .path("/")
            .domain(domain);

        let cookie = if remember_me {
            cookie.permanent()
        } else {
            cookie
        };

        self.cookie(cookie.finish())
    }
}

impl RemoveSessionCookie for HttpResponseBuilder {
    fn remove_session(&mut self) -> &mut Self {
        let mut cookie = Cookie::named(SESSION_COOKIE_NAME);

        cookie.make_removal();

        self.cookie(cookie)
    }
}

impl User {
    fn from_raw(raw: RawUser) -> Result<Self, ()> {
        let role = UserRole::from_str(&raw.role)?;

        let user = Self {
            id: raw.id,
            username: raw.username,
            role,
            mini_pfp: raw.mini_pfp.unwrap_or(default_mini_pfp()),
        };

        Ok(user)
    }
}

#[derive(Debug, Clone)]
struct RawUser {
    pub id: Id,
    pub username: String,
    pub role: String,
    pub mini_pfp: Option<String>,
}

impl RawUser {
    pub async fn fetch_by_session(
        db_pool: &MySqlPool,
        session: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "
            SELECT
                users.id,
                users.username,
                users.role,
                CONCAT(files.id, \".\", files.extension) AS mini_pfp
            FROM users
                LEFT JOIN files ON users.mini_pfp_file_id = files.id
            WHERE
                session = ?
            ",
            session
        )
        .fetch_optional(db_pool)
        .await
    }
}
