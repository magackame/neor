use super::password_pair::PasswordPair;
use argon2::{
    password_hash::{errors::Error, rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

#[derive(Debug)]
pub struct HashedPassword(String);

impl AsRef<str> for HashedPassword {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl HashedPassword {
    pub fn hash(password_pair: &PasswordPair) -> Result<Self, Error> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password_pair.as_ref().as_bytes(), &salt)?
            .to_string();

        Ok(Self(password_hash))
    }
}
