use super::default_mini_pfp;
use super::id::Id;
use super::user::Preview as UserPreview;
use crate::session::User;
use chrono::NaiveDateTime;
use serde::Serialize;

pub mod content;
pub mod description;
pub mod tags;
pub mod title;

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub posted_by: Option<UserPreview>,
    pub posted_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub content: String,
    pub posted_by: Option<UserPreview>,
    pub posted_at: String,
    pub modified_at: Option<String>,

    pub is_commentable: bool,
    pub is_editable: bool,
    pub is_anonymisable: bool,
    pub is_deletable: bool,
}

impl Post {
    pub fn from_raw<'a>(
        raw: RawPost,
        tags: Vec<String>,
        user: impl Into<Option<&'a User>>,
    ) -> Self {
        let user = user.into();

        Self {
            id: raw.id,
            title: raw.title,
            description: raw.description,
            tags,
            content: raw.content,
            posted_at: format_posted_at(raw.posted_at),
            modified_at: raw.modified_at.map(format_posted_at),
            posted_by: raw
                .posted_by_user_id
                .zip(raw.posted_by_username)
                .map(|(id, username)| UserPreview {
                    id,
                    username,
                    mini_pfp: raw.posted_by_mini_pfp.unwrap_or(default_mini_pfp()),
                }),

            is_commentable: user.map(|user| user.role.can_comment()).unwrap_or(false),
            // TODO: move out the functions and reuse them in API
            is_editable: user
                .map(|user| {
                    let hours_since_posted_at =
                        (chrono::Utc::now().naive_utc() - raw.posted_at).num_hours();

                    // TODO: Make edit time window configurable
                    raw.posted_by_user_id
                        .map(|id| user.id == id && user.role.can_edit_posts())
                        .unwrap_or(false)
                        && hours_since_posted_at < 2
                })
                .unwrap_or(false),
            is_anonymisable: user
                .map(|user| {
                    raw.posted_by_user_id
                        .map(|id| user.id == id && user.role.can_anonymise_posts())
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            is_deletable: user
                .map(|user| user.role.can_delete_posts())
                .unwrap_or(false),
        }
    }
}

impl Preview {
    pub fn from_raw(raw: RawPreview, tags: Vec<String>) -> Self {
        Self {
            id: raw.id,
            title: raw.title,
            description: raw.description,
            tags,
            posted_by: raw
                .posted_by_user_id
                .zip(raw.posted_by_username)
                .map(|(id, username)| UserPreview {
                    id,
                    username,
                    mini_pfp: raw.posted_by_mini_pfp.unwrap_or(default_mini_pfp()),
                }),
            posted_at: format_posted_at(raw.posted_at),
        }
    }
}

#[derive(Debug)]
pub struct RawPost {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub content: String,
    pub posted_by_user_id: Option<Id>,
    pub posted_by_username: Option<String>,
    pub posted_by_mini_pfp: Option<String>,
    pub posted_at: NaiveDateTime,
    pub modified_at: Option<NaiveDateTime>,
}

#[derive(Debug)]
pub struct RawPreview {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub posted_by_user_id: Option<Id>,
    pub posted_by_username: Option<String>,
    pub posted_by_mini_pfp: Option<String>,
    pub posted_at: NaiveDateTime,
}

pub fn format_posted_at(posted_at: NaiveDateTime) -> String {
    format!("{}", posted_at.format("%B %d · %Y"))
}
