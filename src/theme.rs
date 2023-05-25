use actix_web::{cookie::Cookie, HttpRequest, HttpResponseBuilder};

#[derive(Debug, Copy, Clone)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn from_str(theme: &str) -> Result<Self, ()> {
        match theme {
            LIGHT => Ok(Self::Light),
            DARK => Ok(Self::Dark),
            _ => Err(()),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => LIGHT,
            Self::Dark => DARK,
        }
    }

    pub fn switch(&self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn body_background_color(&self) -> &'static str {
        match self {
            Self::Light => "white",
            Self::Dark => "rgb(38, 38, 38)",
        }
    }

    pub fn main_font_color(&self) -> &'static str {
        match self {
            Self::Light => "black",
            Self::Dark => "white",
        }
    }

    pub fn link_color(&self) -> &'static str {
        match self {
            Self::Light => "blue",
            Self::Dark => "pink",
        }
    }

    pub fn author_font_color(&self) -> &'static str {
        match self {
            Self::Light => "rgb(92, 92, 92)",
            Self::Dark => "#d3d3d3",
        }
    }

    pub fn tag_background_color(&self) -> &'static str {
        match self {
            Self::Light => "rgb(38, 38, 38)",
            Self::Dark => "rgba(255, 192, 203, 0.7)",
        }
    }

    pub fn link_hover_color(&self) -> &'static str {
        match self {
            Self::Light => "#48c778",
            Self::Dark => "white",
        }
    }

    pub fn tag_color(&self) -> &'static str {
        match self {
            Self::Light => "white",
            Self::Dark => "white",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
}

pub const THEME_COOKIE_NAME: &str = "theme";

pub trait GetThemeCookie {
    fn get_theme(&self) -> Theme;
}

impl GetThemeCookie for HttpRequest {
    fn get_theme(&self) -> Theme {
        self.cookie(THEME_COOKIE_NAME)
            .and_then(|cookie| Theme::from_str(cookie.value()).ok())
            .unwrap_or_default()
    }
}

pub trait SetThemeCookie {
    fn set_theme(&mut self, theme: Theme) -> &mut Self;
}

impl SetThemeCookie for HttpResponseBuilder {
    fn set_theme(&mut self, theme: Theme) -> &mut Self {
        let cookie = Cookie::new(THEME_COOKIE_NAME, theme.as_str());

        self.cookie(cookie)
    }
}

const LIGHT: &str = "light";
const DARK: &str = "dark";
