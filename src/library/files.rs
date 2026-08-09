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

    pub(crate) fn dir(&self) -> &Path {
        self.dir.as_path()
    }

    pub(crate) fn import(&self, source: &Path) -> Result<PathBuf, std::io::Error> {
        let managed = self.dir().join(format!("{}.epub", Uuid::new_v4()));

        if let Err(error) = fs::copy(source, &managed) {
            self.remove(&managed);
            return Err(error);
        }

        Ok(managed)
    }

    pub(crate) fn write_cover(&self, managed: &Path, ext: &str, bytes: &[u8]) -> Option<String> {
        let path = managed.with_extension(format!("cover.{ext}"));
        fs::write(&path, bytes).ok()?;
        Some(path.to_string_lossy().into_owned())
    }

    pub(crate) fn remove(&self, path: &Path) {
        match fs::remove_file(path) {
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
