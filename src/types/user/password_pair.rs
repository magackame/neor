use super::password::Password;

#[derive(Debug)]
pub struct PasswordPair(String);

impl AsRef<str> for PasswordPair {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PasswordPair {
    pub fn parse(password: Password, password_repeat: Password) -> Result<Self, ()> {
        if password != password_repeat {
            return Err(());
        }

        Ok(Self(password.into_string()))
    }
}
