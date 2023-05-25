use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize, sqlx::Type)]
pub enum Role {
    Admin,
    Mod,
    Member,
    Banned,
    Unverified,
}

impl Default for Role {
    fn default() -> Self {
        Self::Unverified
    }
}

impl Role {
    pub fn can_post(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_comment(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_edit_posts(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_edit_comments(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_edit_self(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_anonymise_posts(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_anonymise_comments(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_delete_posts(self) -> bool {
        match self {
            Self::Admin | Self::Mod => true,
            Self::Member | Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_delete_comments(self) -> bool {
        match self {
            Self::Admin | Self::Mod => true,
            Self::Member | Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_reply(self) -> bool {
        match self {
            Self::Admin | Self::Mod | Self::Member => true,
            Self::Banned | Self::Unverified => false,
        }
    }

    pub fn can_admin(self) -> bool {
        match self {
            Self::Admin => true,
            Self::Mod | Self::Member | Self::Banned | Self::Unverified => false,
        }
    }

    pub fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            ADMIN => Ok(Self::Admin),
            MOD => Ok(Self::Mod),
            MEMBER => Ok(Self::Member),
            BANNED => Ok(Self::Banned),
            UNVERIFIED => Ok(Self::Unverified),
            _ => Err(()),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => ADMIN,
            Self::Mod => MOD,
            Self::Member => MEMBER,
            Self::Banned => BANNED,
            Self::Unverified => UNVERIFIED,
        }
    }
}

const ADMIN: &str = "Admin";
const MOD: &str = "Mod";
const MEMBER: &str = "Member";
const BANNED: &str = "Banned";
const UNVERIFIED: &str = "Unverified";
