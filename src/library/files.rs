use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use uuid::Uuid;

pub(crate) struct BookFiles {
    dir: PathBuf,
}

impl BookFiles {
    pub(crate) fn new(dir: PathBuf) -> Self {
        BookFiles { dir }
    }

    pub(crate) fn path_of(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub(crate) fn import(&self, source: &Path) -> Result<String, std::io::Error> {
        let name = format!("{}.epub", Uuid::new_v4());

        if let Err(error) = fs::copy(source, self.path_of(&name)) {
            self.remove(&name);
            return Err(error);
        }

        Ok(name)
    }

    pub(crate) fn write_cover(&self, book: &str, ext: &str, bytes: &[u8]) -> Option<String> {
        let name = Path::new(book)
            .with_extension(format!("cover.{ext}"))
            .to_string_lossy()
            .into_owned();
        fs::write(self.path_of(&name), bytes).ok()?;
        Some(name)
    }

    pub(crate) fn remove(&self, name: &str) {
        let path = self.path_of(name);

        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                eprintln!(
                    "failed to clean up imported copy {}, {error}",
                    path.display()
                );
            }
        }
    }
}
