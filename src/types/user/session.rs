use crate::types::id::Id;
use sqlx::mysql::MySqlPool;
use sqlx::Result;
use uuid::Uuid;

#[derive(Debug)]
pub struct Session(String);

impl AsRef<str> for Session {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Session {
    pub async fn new(db_pool: &MySqlPool) -> Result<Self> {
        let session = loop {
            let session = Uuid::new_v4().to_string();

            if Self::fetch_user_id(db_pool, &session).await?.is_none() {
                break session;
            }
        };

        Ok(Self(session))
    }

    pub async fn fetch_user_id(db_pool: &MySqlPool, session: &str) -> sqlx::Result<Option<Id>> {
        #[derive(Debug)]
        struct User {
            pub id: Id,
        }

        sqlx::query_as!(
            User,
            "
            SELECT
                id
            FROM users
            WHERE
                session = ?
            ",
            session
        )
        .fetch_optional(db_pool)
        .await
        .map(|result| result.map(|user| user.id))
    }
}
