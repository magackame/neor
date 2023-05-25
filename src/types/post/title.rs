pub const TITLE_MAX_CHAR_COUNT: usize = 256;

#[derive(Debug)]
pub struct Title(String);

impl AsRef<str> for Title {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Title {
    pub fn parse(title: impl Into<String>) -> Result<Self, ()> {
        let title = title.into();

        if title.is_empty() {
            return Err(());
        }

        if title.chars().count() > TITLE_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(title))
    }
}
