pub const CONTENT_MAX_CHAR_COUNT: usize = 8_129;

#[derive(Debug)]
pub struct Content(String);

impl AsRef<str> for Content {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Content {
    pub fn parse(content: impl Into<String>) -> Result<Self, ()> {
        let content = content.into();

        if content.is_empty() {
            return Err(());
        }

        if content.chars().count() > CONTENT_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(content))
    }
}
