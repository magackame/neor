use super::default_mini_pfp;
use super::id::Id;
use super::post::format_posted_at;
use super::user::Preview as UserPreview;
use crate::session::User;
use chrono::NaiveDateTime;
use serde::Serialize;

pub mod content;

pub mod reply;
use reply::Reply;

#[derive(Debug, Serialize)]
pub struct Comment {
    pub id: Id,
    pub post_id: Id,
    pub reply: Option<Reply>,
    pub content: String,
    pub posted_by: Option<UserPreview>,
    pub posted_at: String,
    pub modified_at: Option<String>,

    pub is_repliable: bool,
    pub is_editable: bool,
    pub is_anonymisable: bool,
    pub is_deletable: bool,
}

impl Comment {
    pub fn from_raw<'a>(raw: RawComment, user: impl Into<Option<&'a User>>) -> Self {
        let user = user.into();

        Self {
            id: raw.id,
            post_id: raw.post_id,
            reply: raw.reply_to_comment_id.map(|comment_id| Reply {
                comment_id,
                posted_by: raw
                    .reply_to_user_id
                    .zip(raw.reply_to_username)
                    .map(|(id, username)| UserPreview {
                        id,
                        username,
                        mini_pfp: raw.reply_to_mini_pfp.unwrap_or(default_mini_pfp()),
                    }),
            }),
            content: raw.content,
            posted_by: raw
                .posted_by_user_id
                .zip(raw.posted_by_username)
                .map(|(id, username)| UserPreview {
                    id,
                    username,
                    mini_pfp: raw.posted_by_mini_pfp.unwrap_or(default_mini_pfp()),
                }),
            posted_at: format_posted_at(raw.posted_at),
            modified_at: raw.modified_at.map(format_posted_at),

            // TODO: move out the functions and reuse them in API
            is_repliable: user.map(|user| user.role.can_reply()).unwrap_or(false),
            is_editable: user
                .map(|user| {
                    let hours_since_posted_at =
                        (chrono::Utc::now().naive_utc() - raw.posted_at).num_hours();

                    // TODO: Make edit time window configurable
                    raw.posted_by_user_id
                        .map(|id| user.id == id && user.role.can_edit_comments())
                        .unwrap_or(false)
                        && hours_since_posted_at < 2
                })
                .unwrap_or(false),
            is_anonymisable: user
                .map(|user| {
                    raw.posted_by_user_id
                        .map(|id| user.id == id && user.role.can_anonymise_comments())
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            is_deletable: user
                .map(|user| user.role.can_delete_comments())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug)]
pub struct RawComment {
    pub id: Id,
    pub post_id: Id,
    pub reply_to_comment_id: Option<Id>,
    pub reply_to_user_id: Option<Id>,
    pub reply_to_username: Option<String>,
    pub reply_to_mini_pfp: Option<String>,
    pub content: String,
    pub posted_by_user_id: Option<Id>,
    pub posted_by_username: Option<String>,
    pub posted_by_mini_pfp: Option<String>,
    pub posted_at: NaiveDateTime,
    pub modified_at: Option<NaiveDateTime>,
}
