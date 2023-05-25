use lazy_static::lazy_static;
use regex::Regex;

pub const USERNAME_MAX_CHAR_COUNT: usize = 64;

#[derive(Debug)]
pub struct Username(String);

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Username {
    pub fn parse(username: impl Into<String>) -> Result<Self, ()> {
        lazy_static! {
            static ref REGEX: Regex = {
                let regex = format!("^[A-z0-9\\-_]{{1,{}}}$", USERNAME_MAX_CHAR_COUNT);

                Regex::new(&regex).expect("Failed to compile regex")
            };
        }

        let username = username.into();

        if !REGEX.is_match(&username) {
            return Err(());
        }

        Ok(Self(username))
    }
}
