pub const DESCRIPTION_MAX_CHAR_COUNT: usize = 512;

#[derive(Debug)]
pub struct Description(String);

impl AsRef<str> for Description {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Description {
    pub fn parse(description: impl Into<String>) -> Result<Self, ()> {
        let description = description.into();

        if description.is_empty() {
            return Err(());
        }

        if description.chars().count() > DESCRIPTION_MAX_CHAR_COUNT {
            return Err(());
        }

        Ok(Self(description))
    }
}
