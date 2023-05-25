pub mod comment;
pub mod email_verification;
pub mod password_change;
pub mod password_reset;
pub mod post;
pub mod sign_in;
pub mod sign_up;
pub mod user;

pub fn is_checked(checkbox: Option<String>) -> bool {
    match checkbox {
        Some(checkbox) => matches!(checkbox.as_str(), "on"),
        None => false,
    }
}
