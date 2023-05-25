pub const EMAIL_MAX_CHAR_COUNT: usize = 320;

#[derive(Debug)]
pub struct Email(String);

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Email {
    pub fn parse(email: impl Into<String>) -> Result<Self, ()> {
        let email = email.into();

        if email.is_empty() {
            return Err(());
        }

        if email.chars().count() > EMAIL_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(email))
    }
}
