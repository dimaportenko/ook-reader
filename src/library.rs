use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use rbook::Epub;
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::epub;

#[cfg(test)]
use crate::epub::BookMeta;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Book {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) author: Option<String>,
}

pub(crate) struct Library {
    conn: Connection,
    books_dir: PathBuf,
}

impl Library {
    #[cfg(test)]
    fn open_in_memory(books_dir: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::init(
            Connection::open_in_memory()?,
            books_dir.as_ref().to_path_buf(),
        )
    }

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
                author TEXT
            )",
            [],
        )?;
        Ok(Self { conn, books_dir })
    }

    #[cfg(test)]
    pub(crate) fn add(&self, path: &str, meta: &BookMeta) -> rusqlite::Result<Book> {
        let id = self.conn.query_row(
            "INSERT INTO books (path, source_path, title, author)
                VALUES (?1, ?1, ?2, ?3)
                ON CONFLICT(path) DO UPDATE SET
                    title = excluded.title,
                    author = excluded.author
            RETURNING id",
            params![path, &meta.title, meta.author.as_deref()],
            |row| row.get(0),
        )?;
        Ok(Book {
            id,
            path: path.to_string(),
            title: meta.title.clone(),
            author: meta.author.clone(),
        })
    }

    fn read_book(row: &Row<'_>) -> rusqlite::Result<Book> {
        Ok(Book {
            id: row.get(0)?,
            path: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
        })
    }

    pub(crate) fn add_from_path(
        &self,
        source_path: &Path,
    ) -> Result<Book, Box<dyn std::error::Error>> {
        let source_path = source_path.canonicalize()?;
        let source_path_text = source_path.to_string_lossy().into_owned();

        let previous_path: Option<String> = self
            .conn
            .query_row(
                "SELECT path FROM books WHERE source_path = ?1",
                params![&source_path_text],
                |row| row.get(0),
            )
            .optional()?;

        let managed_path = self.books_dir.join(format!("{}.epub", Uuid::new_v4()));

        if let Err(error) = fs::copy(&source_path, &managed_path) {
            cleanup_managed_file(&managed_path);
            return Err(Box::new(error));
        }

        let result = (|| -> Result<Book, Box<dyn std::error::Error>> {
            let epub = Epub::open(&managed_path)?;
            let meta = epub::read_metadata(&epub)?;
            let managed_path_text = managed_path.to_string_lossy().into_owned();

            let book = self.conn.query_row(
                "INSERT INTO books (path, source_path, title, author)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(source_path) DO UPDATE SET
                    path = excluded.path,
                    title = excluded.title,
                    author = excluded.author
                RETURNING id, path, title, author",
                params![
                    &managed_path_text,
                    &source_path_text,
                    &meta.title,
                    meta.author.as_deref()
                ],
                Self::read_book,
            )?;

            Ok(book)
        })();

        match &result {
            Err(_) => cleanup_managed_file(&managed_path),
            Ok(_) => {
                if let Some(previous) = previous_path {
                    cleanup_managed_file(Path::new(&previous));
                }
            }
        }

        result
    }

    pub(crate) fn remove(&self, id: i64) -> rusqlite::Result<bool> {
        let removed_path: Option<String> = self
            .conn
            .query_row(
                "DELETE FROM books WHERE id = ?1 RETURNING path",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        let Some(removed_path) = removed_path else {
            return Ok(false);
        };

        cleanup_managed_file(Path::new(&removed_path));

        Ok(true)
    }

    pub(crate) fn list(&self) -> rusqlite::Result<Vec<Book>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, title, author FROM books ORDER BY title")?;
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
        let books_dir = dir.path().join("books");
        let library = Library::open_in_memory(&books_dir).expect("in-memory db opens");
        let holmes_path = books_dir.join("holmes.epub");
        let beowulf_path = books_dir.join("beowulf.epub");

        let holmes = BookMeta {
            title: "The Adventures of Sherlock Holmes".to_string(),
            author: Some("Arthur Conan Doyle".to_string()),
            cover: None,
        };
        let added = library
            .add(&holmes_path.to_string_lossy(), &holmes)
            .expect("add succeeds");

        assert_eq!(added.id, 1);
        assert_eq!(added.path, holmes_path.to_string_lossy());

        let beowulf = BookMeta {
            title: "Beowulf".to_string(),
            author: None,
            cover: None,
        };
        library
            .add(&beowulf_path.to_string_lossy(), &beowulf)
            .expect("add anon");

        let books = library.list().expect("list succeeds");
        assert_eq!(books.len(), 2);

        // ORDER BY title
        assert_eq!(books[0].title, "Beowulf");
        assert_eq!(books[0].author, None);
        assert_eq!(books[1], added);
        assert_eq!(books[1].author.as_deref(), Some("Arthur Conan Doyle"));
    }

    #[test]
    fn file_backed_library_survives_reopen_and_reimport_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("library.sqlite3");
        let meta = BookMeta {
            title: "The Adventures of Sherlock Holmes".to_string(),
            author: Some("Arthur Conan Doyle".to_string()),
            cover: None,
        };

        let books_dir = dir.path().join("books");
        let library = Library::open(&db_path, &books_dir).expect("file database opens");
        let first = library.add("/books/holmes.epub", &meta).expect("first add");
        drop(library);

        let library = Library::open(&db_path, &books_dir).expect("database reopnes");
        let second = library
            .add("/books/holmes.epub", &meta)
            .expect("second add");
        let books = library.list().expect("list succeeds");

        assert_eq!(second.id, first.id);
        assert_eq!(books, vec![second]);
    }

    #[test]
    fn remove_drops_the_row_and_is_a_noop_for_unknown_ids() {
        let dir = tempfile::tempdir().expect("temp dir");
        let library =
            Library::open_in_memory(dir.path().join("books")).expect("in-memory db opens");
        let books_dir = dir.path().join("books");
        let holmes_path = books_dir.join("holmes.epub");
        let beowulf_path = books_dir.join("beowulf.epub");

        let holmes = BookMeta {
            title: "The Adventures of Sherlock Holmes".to_string(),
            author: Some("Arthur Conan Doyle".to_string()),
            cover: None,
        };
        let beowulf = BookMeta {
            title: "Beowulf".to_string(),
            author: None,
            cover: None,
        };
        let added = library
            .add(&holmes_path.to_string_lossy(), &holmes)
            .expect("add holmes");
        library
            .add(&beowulf_path.to_string_lossy(), &beowulf)
            .expect("add beowulf");

        // Remove by the DB-assigned id, not by path.
        let removed = library.remove(added.id).expect("remove succeeds");
        assert!(removed, "expected an existing row to report true");

        let books = library.list().expect("list succeeds");
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Beowulf");
        assert_ne!(books[0].id, added.id);

        // Unknown id: no error, no change, reports false.
        let removed_again = library.remove(added.id).expect("missing id is Ok(false)");
        assert!(!removed_again);
        assert_eq!(library.list().expect("list still one").len(), 1);
    }

    #[test]
    fn import_opens_from_managed_copy_after_source_is_deleted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("library.sqlite3");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");

        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");

        let library = Library::open(&db_path, &books_dir).expect("library opens");
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
        let db_path = dir.path().join("library.sqlite3");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");

        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");

        let library = Library::open(&db_path, &books_dir).expect("library opens");
        let first = library.add_from_path(&source).expect("first import");
        let second = library.add_from_path(&source).expect("reimport");

        // Same logical book, fresh bytes: id stable, managed path replaced.
        assert_eq!(second.id, first.id);
        assert_ne!(second.path, first.path);
        assert!(!Path::new(&first.path).exists());
        assert!(Path::new(&second.path).exists());

        // Exactly one managed file and one row — nothing leaked, nothing duplicated.
        let files = std::fs::read_dir(&books_dir)
            .expect("read books dir")
            .count();
        assert_eq!(files, 1);
        assert_eq!(library.list().expect("list succeeds"), vec![second]);
    }

    #[test]
    fn reimport_repairs_a_missing_managed_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("library.sqlite3");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");

        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");

        let library = Library::open(&db_path, &books_dir).expect("library opens");
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
        let db_path = dir.path().join("library.sqlite3");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");

        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");

        let library = Library::open(&db_path, &books_dir).expect("library opens");
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
        let db_path = dir.path().join("library.sqlite3");
        let books_dir = dir.path().join("books");
        std::fs::create_dir_all(&books_dir).expect("books dir");

        let source = dir.path().join("holmes-source.epub");
        std::fs::copy(crate::BOOK, &source).expect("fixture source");

        let library = Library::open(&db_path, &books_dir).expect("library opens");
        let added = library.add_from_path(&source).expect("import succeeds");

        // Simulate a hand-deleted managed file: the row now points at nothing.
        std::fs::remove_file(&added.path).expect("delete managed copy");

        let removed = library
            .remove(added.id)
            .expect("missing file is not an error");

        assert!(removed, "a stale row is still removable");
        assert!(library.list().expect("list succeeds").is_empty());
    }
}
