use crate::types::id::Id;
use rand::Rng;
use sqlx::mysql::MySqlPool;

pub const CODE_CHAR_COUNT: usize = 6;
pub const CODE_UPPER_NUM_BOUND: usize = 10usize.pow(CODE_CHAR_COUNT as u32);

// TODO: Code expiration
#[derive(Debug)]
pub struct Code(String);

impl AsRef<str> for Code {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Code {
    pub async fn new(db_pool: &MySqlPool) -> sqlx::Result<Self> {
        let mut rng = rand::thread_rng();

        let code = loop {
            let code = rng.gen_range(100_000..CODE_UPPER_NUM_BOUND).to_string();

            if Self::fetch_user_id(db_pool, &code).await?.is_none() {
                break code;
            }
        };

        Ok(Self(code))
    }

    pub async fn fetch_user_id(db_pool: &MySqlPool, code: &str) -> sqlx::Result<Option<Id>> {
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
                code = ?
            ",
            code
        )
        .fetch_optional(db_pool)
        .await
        .map(|result| result.map(|user| user.id))
    }
}
