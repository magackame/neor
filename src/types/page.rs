use serde::Deserialize;

pub const MIN_LIMIT: u64 = 1;
pub const MAX_LIMIT: u64 = 100;

pub const DEFAULT_LIMIT: u64 = 40;

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Backwards,
    Forwards,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Forwards
    }
}

// TODO: Unify with serde, so both produce the same result
impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Backwards => "backwards",
            Self::Forwards => "forwards",
        };

        write!(f, "{s}")
    }
}

pub fn clamp(n: u64, min: u64, max: u64) -> u64 {
    std::cmp::max(min, std::cmp::min(n, max))
}

pub fn clamp_limit(limit: impl Into<Option<u64>>) -> u64 {
    let limit = limit.into().unwrap_or(DEFAULT_LIMIT);

    clamp(limit, MIN_LIMIT, MAX_LIMIT)
}
