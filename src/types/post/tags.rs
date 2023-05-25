use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

pub const TAGS_MAX_CHAR_COUNT: usize = TAG_MAX_CHAR_COUNT * TAG_MAX_COUNT + TAG_MAX_COUNT - 1;
pub const TAG_MAX_CHAR_COUNT: usize = 64;

pub const TAG_MIN_COUNT: usize = 1;
pub const TAG_MAX_COUNT: usize = 10;

#[derive(Debug, Copy, Clone)]
pub enum Error {
    NotEnoughTags,
    TooManyTags,
    InvalidTag,
}

#[derive(Debug)]
pub struct Tags(Vec<String>);

impl AsRef<Vec<String>> for Tags {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl Tags {
    pub fn parse(tags: impl Into<String>) -> Result<Self, Error> {
        let tags = tags.into();

        let tags = tags.split_whitespace();

        let tag_count = tags.clone().count();

        if tag_count < TAG_MIN_COUNT {
            return Err(Error::NotEnoughTags);
        }

        if tag_count > TAG_MAX_COUNT {
            return Err(Error::TooManyTags);
        }

        lazy_static! {
            static ref REGEX: Regex = {
                let regex = format!("^[A-z0-9\\-_]{{1,{}}}$", TAG_MAX_CHAR_COUNT);

                Regex::new(&regex).expect("Failed to compile regex")
            };
        }

        if tags
            .clone()
            .map(|tag| REGEX.is_match(tag))
            .any(|is_valid| !is_valid)
        {
            return Err(Error::InvalidTag);
        }

        let tags = tags
            .map(|tag| tag.to_owned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Ok(Self(tags))
    }
}

impl IntoIterator for Tags {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
