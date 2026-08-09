pub mod font;
pub mod library;
pub mod reader;
pub mod settings;
pub mod theme;

pub(crate) trait OrLog<T> {
    fn or_log(self, action: &str) -> Option<T>;
}

impl<T> OrLog<T> for Result<T, crate::library::Error> {
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
