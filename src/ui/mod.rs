pub mod components;
pub mod font;
pub mod library;
pub mod reader;
pub mod settings;
pub mod theme;

pub(crate) trait OrLog<T> {
    fn or_log(self, action: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> OrLog<T> for Result<T, E> {
    fn or_log(self, action: &str) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                eprintln!("could not {action}: {error}");
                None
            }
        }
    }
}
