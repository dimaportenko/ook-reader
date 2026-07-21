use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use rbook::Epub;
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::epub;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Book {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) author: Option<String>,
    pub(crate) cover_path: Option<String>,
}

impl Book {
    pub(crate) fn get_book_cover_name(&self) -> Option<&str> {
        self.cover_path
            .as_deref()
            .and_then(|cover| std::path::Path::new(cover).file_name())
            .and_then(|name| name.to_str())
    }
}

pub(crate) struct Library {
    conn: Connection,
    books_dir: PathBuf,
}

impl Library {
    pub(crate) fn open(
        db_path: impl AsRef<std::path::Path>,
        books_dir: impl AsRef<std::path::Path>,
    ) -> rusqlite::Result<Self> {
        Self::init(Connection::open(db_path)?, books_dir.as_ref().to_path_buf())
    }

    fn init(conn: Connection, books_dir: PathBuf) -> rusqlite::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                source_path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                author TEXT,
                cover_path TEXT
            )",
            [],
        )?;
        Ok(Self { conn, books_dir })
    }

    pub(crate) fn books_dir(&self) -> &Path {
        self.books_dir.as_path()
    }

    fn read_book(row: &Row<'_>) -> rusqlite::Result<Book> {
        Ok(Book {
            id: row.get(0)?,
            path: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            cover_path: row.get(4)?,
        })
    }

    pub(crate) fn add_from_path(
        &self,
        source_path: &Path,
    ) -> Result<Book, Box<dyn std::error::Error>> {
        let source_path = source_path.canonicalize()?;
        let source_path_text = source_path.to_string_lossy().into_owned();

        let previous: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT path, cover_path FROM books WHERE source_path = ?1",
                params![&source_path_text],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let managed_path = self.books_dir.join(format!("{}.epub", Uuid::new_v4()));

        if let Err(error) = fs::copy(&source_path, &managed_path) {
            cleanup_managed_file(&managed_path);
            return Err(Box::new(error));
        }

        let mut cover_path: Option<String> = None;

        let result = (|| -> Result<Book, Box<dyn std::error::Error>> {
            let epub = Epub::open(&managed_path)?;
            let meta = epub::read_metadata(&epub)?;
            let managed_path_text = managed_path.to_string_lossy().into_owned();

            cover_path = meta.cover.as_ref().and_then(|cover| {
                let ext = epub::extension_for(&cover.media_type)?;
                let path = managed_path.with_extension(format!("cover.{ext}"));
                fs::write(&path, &cover.bytes).ok()?;
                Some(path.to_string_lossy().into_owned())
            });

            let book = self.conn.query_row(
                "INSERT INTO books (path, source_path, title, author, cover_path)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(source_path) DO UPDATE SET
                    path = excluded.path,
                    title = excluded.title,
                    author = excluded.author,
                    cover_path = excluded.cover_path
                RETURNING id, path, title, author, cover_path",
                params![
                    &managed_path_text,
                    &source_path_text,
                    &meta.title,
                    meta.author.as_deref(),
                    cover_path.as_deref()
                ],
                Self::read_book,
            )?;

            Ok(book)
        })();

        match &result {
            Err(_) => {
                cleanup_managed_file(&managed_path);
                if let Some(path) = &cover_path {
                    cleanup_managed_file(Path::new(path));
                }
            }
            Ok(_) => {
                if let Some((previous_path, previous_cover)) = previous {
                    cleanup_managed_file(Path::new(&previous_path));
                    if let Some(cover) = previous_cover {
                        cleanup_managed_file(Path::new(&cover));
                    }
                }
            }
        }

        result
    }

    pub(crate) fn remove(&self, id: i64) -> rusqlite::Result<bool> {
        let removed: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "DELETE FROM books WHERE id = ?1 RETURNING path, cover_path",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        if let Some((removed_path, removed_cover)) = removed {
            cleanup_managed_file(Path::new(&removed_path));
            if let Some(cover) = removed_cover {
                cleanup_managed_file(Path::new(&cover));
            }
            return Ok(true);
        };

        Ok(false)
    }

    pub(crate) fn list(&self) -> rusqlite::Result<Vec<Book>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, title, author, cover_path  FROM books ORDER BY title")?;
        let rows = stmt.query_map([], Self::read_book)?;
        rows.collect()
    }
}

fn cleanup_managed_file(path: &Path) {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_then_list_round_trips_books() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, first_source, books_dir) = library_with_source(&dir);
        let second_source = dir.path().join("holmes-second-source.epub");
        std::fs::copy(crate::BOOK, &second_source).expect("second fixture source");

        let first = library.add_from_path(&first_source).expect("first import");
        let second = library.add_from_path(&second_source).expect("second import");

        // Distinct source paths are distinct books, with metadata read from the file.
        assert_ne!(first.id, second.id);
        assert!(Path::new(&first.path).starts_with(&books_dir));
        assert!(first.title.contains("Sherlock Holmes"));
        assert!(first.author.as_deref().unwrap_or("").contains("Doyle"));

        // Both sources are the same fixture, so titles tie and ORDER BY title
        // leaves their relative order unspecified — assert contents, not order.
        let books = library.list().expect("list succeeds");
        assert_eq!(books.len(), 2);
        assert!(books.contains(&first));
        assert!(books.contains(&second));
    }

    #[test]
    fn file_backed_library_survives_reopen_and_reimport_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let first = library.add_from_path(&source).expect("first import");
        drop(library);

        let library = Library::open(dir.path().join("library.sqlite3"), &books_dir)
            .expect("database reopens");
        let second = library.add_from_path(&source).expect("second import");
        let books = library.list().expect("list succeeds");

        assert_eq!(second.id, first.id);
        assert_eq!(books, vec![second]);
    }

    #[test]
    fn remove_drops_the_row_and_is_a_noop_for_unknown_ids() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, first_source, _) = library_with_source(&dir);
        let second_source = dir.path().join("holmes-second-source.epub");
        std::fs::copy(crate::BOOK, &second_source).expect("second fixture source");
        let first = library.add_from_path(&first_source).expect("first import");
        let second = library.add_from_path(&second_source).expect("second import");

        // Remove by the DB-assigned id, not by path.
        let removed = library.remove(first.id).expect("remove succeeds");
        assert!(removed, "expected an existing row to report true");

        let books = library.list().expect("list succeeds");
        assert_eq!(books, vec![second]);

        // Unknown id: no error, no change, reports false.
        let removed_again = library.remove(first.id).expect("missing id is Ok(false)");
        assert!(!removed_again);
        assert_eq!(library.list().expect("list still one").len(), 1);
    }

    #[test]
    fn import_opens_from_managed_copy_after_source_is_deleted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let added = library.add_from_path(&source).expect("import succeeds");

        assert!(std::path::Path::new(&added.path).starts_with(&books_dir));
        assert_ne!(std::path::Path::new(&added.path), source.as_path());

        std::fs::remove_file(&source).expect("delete source");
        let epub = rbook::Epub::open(&added.path).expect("managed copy opens");
        let meta = crate::epub::read_metadata(&epub).expect("managed metadata");

        assert!(meta.title.contains("Sherlock Holmes"));
        assert_eq!(added.title, meta.title);
    }

    #[test]
    fn reimport_replaces_the_managed_copy_without_leaking_the_old_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, books_dir) = library_with_source(&dir);
        let first = library.add_from_path(&source).expect("first import");
        let second = library.add_from_path(&source).expect("reimport");

        // Same logical book, fresh bytes: id stable, managed path replaced.
        assert_eq!(second.id, first.id);
        assert_ne!(second.path, first.path);
        assert!(!Path::new(&first.path).exists());
        assert!(Path::new(&second.path).exists());

        // 2 managed file (epub and cover image) and one row — nothing leaked, nothing duplicated.
        let files = std::fs::read_dir(&books_dir)
            .expect("read books dir")
            .count();
        assert_eq!(files, 2);
        assert_eq!(library.list().expect("list succeeds"), vec![second]);
    }

    #[test]
    fn reimport_repairs_a_missing_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let first = library.add_from_path(&source).expect("first import");

        // Simulate a hand-deleted managed file: the row now points at nothing.
        std::fs::remove_file(&first.path).expect("delete managed copy");

        let repaired = library.add_from_path(&source).expect("reimport repairs");

        assert_eq!(repaired.id, first.id);
        rbook::Epub::open(&repaired.path).expect("repaired copy opens");
    }

    #[test]
    fn remove_deletes_the_row_and_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let added = library.add_from_path(&source).expect("import succeeds");

        let removed = library.remove(added.id).expect("remove succeeds");

        assert!(removed, "expected an existing row to report true");
        assert!(library.list().expect("list succeeds").is_empty());
        assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
        assert!(source.exists(), "the user's original source is untouched");
    }

    #[test]
    fn remove_succeeds_when_the_managed_copy_is_already_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let added = library.add_from_path(&source).expect("import succeeds");
        // Simulate a hand-deleted managed file: the row now points at nothing.
        std::fs::remove_file(&added.path).expect("delete managed copy");

        let removed = library
            .remove(added.id)
            .expect("missing file is not an error");

        assert!(removed, "a stale row is still removable");
        assert!(library.list().expect("list succeeds").is_empty());
    }

    fn library_with_source(dir: &tempfile::TempDir) -> (Library, PathBuf, PathBuf) {
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");
        let library =
            Library::open(dir.path().join("library.sqlite3"), &books_dir).expect("library opens");
        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");
        (library, source, books_dir)
    }

    #[test]
    fn import_writes_a_cover_file_next_to_the_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);
        let added = library.add_from_path(&source).expect("import succeeds");

        let cover_path = added.cover_path.expect("bundled book has a cover");
        assert!(Path::new(&cover_path).starts_with(dir.path().join("books")));
        assert!(Path::new(&cover_path).exists());
        // The stored extension round-trips through the serve-time content-type lookup.
        assert!(crate::epub::content_type_for(&cover_path).starts_with("image/"));
    }

    #[test]
    fn reimport_replaces_the_cover_without_leaking_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let first = library.add_from_path(&source).expect("first import");
        let second = library.add_from_path(&source).expect("reimport");

        let first_cover = first.cover_path.expect("first import has a cover");
        let second_cover = second.cover_path.expect("reimport has a cover");

        // Same logical book, fresh files: the old cover is gone, the new one exists.
        assert_ne!(second_cover, first_cover);
        assert!(!Path::new(&first_cover).exists());
        assert!(Path::new(&second_cover).exists());

        // Exactly one .epub + one cover — nothing leaked, nothing duplicated.
        // (This is the assertion that goes red in the *old* reimport test: its
        // `files == 1` becomes `files == 2` once covers land next to the copies.)
        let files = std::fs::read_dir(dir.path().join("books"))
            .expect("read books dir")
            .count();
        assert_eq!(files, 2);
    }

    #[test]
    fn remove_deletes_the_cover_file_too() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (library, source, _) = library_with_source(&dir);

        let added = library.add_from_path(&source).expect("import succeeds");
        let cover_path = added.cover_path.clone().expect("import has a cover");

        let removed = library.remove(added.id).expect("remove succeeds");

        assert!(removed, "expected an existing row to report true");
        assert!(library.list().expect("list succeeds").is_empty());
        assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
        assert!(!Path::new(&cover_path).exists(), "cover file is deleted");
        assert!(source.exists(), "the user's original source is untouched");
    }
}
