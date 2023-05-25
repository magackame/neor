use crate::types::id::Id;
use crate::types::user::Preview as UserPreview;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Reply {
    pub comment_id: Id,
    pub posted_by: Option<UserPreview>,
}
