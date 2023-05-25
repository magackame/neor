use super::{default_pfp, id::Id, post::format_posted_at};
use crate::session::User as AuthUser;
use chrono::NaiveDateTime;
use serde::Serialize;

pub mod code;
pub mod description;
pub mod email;
pub mod hashed_password;
pub mod name;
pub mod password;
pub mod password_pair;
pub mod session;
pub mod username;

pub mod role;
use role::Role;

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub id: Id,
    pub username: String,
    pub mini_pfp: String,
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: Id,
    pub username: String,
    pub role: Role,
    pub pfp: String,
    pub name: String,
    pub description: String,
    pub joined_at: String,

    pub is_editable: bool,
    pub is_sign_outable: bool,
    pub is_adminable: bool,
}

impl User {
    pub fn from_raw<'a>(raw: RawUser, user: impl Into<Option<&'a AuthUser>>) -> Self {
        let user = user.into();

        // TODO: Error handling?
        let role = Role::from_str(&raw.role).unwrap();

        Self {
            id: raw.id,
            username: raw.username,
            role,
            pfp: raw.pfp.unwrap_or(default_pfp()),
            name: raw.name,
            description: raw.description,
            joined_at: format_posted_at(raw.joined_at),

            is_editable: user
                .map(|user| user.role.can_edit_self() && user.id == raw.id)
                .unwrap_or(false),
            is_sign_outable: user.map(|user| user.id == raw.id).unwrap_or(false),
            is_adminable: user
                .map(|user| user.role.can_admin() && !role.can_admin())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug)]
pub struct RawUser {
    pub id: Id,
    pub username: String,
    pub role: String,
    pub pfp: Option<String>,
    pub name: String,
    pub description: String,
    pub joined_at: NaiveDateTime,
}
