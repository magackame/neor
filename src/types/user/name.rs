pub const NAME_MAX_CHAR_COUNT: usize = 256;

#[derive(Debug)]
pub struct Name(String);

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Name {
    pub fn parse(name: impl Into<String>) -> Result<Self, ()> {
        let name = name.into();

        if name.is_empty() {
            return Err(());
        }

        if name.chars().count() > NAME_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(name))
    }
}
