use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) app_dir: PathBuf,
    pub(crate) books_dir: PathBuf,
}

impl Config {
    pub(crate) fn new() -> Result<Self, std::io::Error> {
        let config = Self::from_app_dir(Self::app_dir());
        config.ensure_dirs()?;
        Ok(config)
    }

    fn from_app_dir(app_dir: PathBuf) -> Self {
        let books_dir = app_dir.join("books");
        Self { app_dir, books_dir }
    }

    fn app_dir() -> PathBuf {
        let dirs = directories::ProjectDirs::from("com", "dimaportenko", "ook-reader")
            .expect("a home directory should exist");
        dirs.data_dir().to_path_buf()
    }

    fn ensure_dirs(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.books_dir)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_app_dir_is_named_after_the_project() {
        let app_dir = Config::app_dir();

        assert!(
            app_dir
                .to_str()
                .expect("app dir is valid UTF-8")
                .contains("com.dimaportenko.ook-reader"),
            "got {app_dir:?}"
        );
    }

    #[test]
    fn the_books_dir_sits_under_the_app_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = Config::from_app_dir(dir.path().to_path_buf());

        assert_eq!(config.app_dir, dir.path());
        assert_eq!(config.books_dir, dir.path().join("books"));
    }

    #[test]
    fn ensuring_the_dirs_creates_a_missing_tree_and_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = Config::from_app_dir(dir.path().join("never-created"));

        config.ensure_dirs().expect("dirs are creatable");

        assert!(config.app_dir.is_dir());
        assert!(config.books_dir.is_dir());

        config.ensure_dirs().expect("a second run is a no-op");
    }
}
