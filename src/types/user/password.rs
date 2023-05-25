pub const PASSWORD_MAX_CHAR_COUNT: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub struct Password(String);

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Password {
    pub fn parse(password: impl Into<String>) -> Result<Self, ()> {
        let password = password.into();

        if password.is_empty() {
            return Err(());
        }

        if password.chars().count() > PASSWORD_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(password))
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
