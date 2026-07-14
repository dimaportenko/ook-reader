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

        let existing = self
            .conn
            .query_row(
                "SELECT id, path, title, author
                FROM books
                WHERE source_path = ?1",
                params![&source_path_text],
                Self::read_book,
            )
            .optional()?;

        if let Some(book) = existing {
            return Ok(book);
        }

        let managed_path = self.books_dir.join(format!("{}.epub", Uuid::new_v4()));

        if let Err(error) = fs::copy(&source_path, &managed_path) {
            cleanup_failed_import(&managed_path);
            return Err(Box::new(error));
        }

        let result = (|| -> Result<Book, Box<dyn std::error::Error>> {
            let epub = Epub::open(&managed_path)?;
            let meta = epub::read_metadata(&epub)?;
            let managed_path_text = managed_path.to_string_lossy().into_owned();

            let book = self.conn.query_row(
                "INSERT INTO books (path, source_path, title, author)
                VALUES (?1, ?2, ?3, ?4)
                RETURNING id, path, title, author",
                params![
                    &managed_path_text,
                    &source_path_text,
                    &meta.title,
                    meta.author.as_deref(),
                ],
                Self::read_book,
            )?;

            Ok(book)
        })();

        if result.is_err() {
            cleanup_failed_import(&managed_path);
        }

        result
    }

    pub(crate) fn remove(&self, id: i64) -> rusqlite::Result<bool> {
        let removed = self
            .conn
            .execute("DELETE FROM books WHERE id = ?1", params![id])?;
        Ok(removed > 0)
    }

    pub(crate) fn list(&self) -> rusqlite::Result<Vec<Book>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, title, author FROM books ORDER BY title")?;
        let rows = stmt.query_map([], Self::read_book)?;
        rows.collect()
    }
}

fn cleanup_failed_import(path: &Path) {
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
        let library =
            Library::open_in_memory(dir.path().join("books")).expect("in-memory db opens");

        let holmes = BookMeta {
            title: "The Adventures of Sherlock Holmes".to_string(),
            author: Some("Arthur Conan Doyle".to_string()),
        };
        let added = library
            .add("/books/holmes.epub", &holmes)
            .expect("add succeeds");

        assert_eq!(added.id, 1);
        assert_eq!(added.path, "/books/holmes.epub");

        let beowulf = BookMeta {
            title: "Beowulf".to_string(),
            author: None,
        };
        library
            .add("/books/beowulf.epub", &beowulf)
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

        let holmes = BookMeta {
            title: "The Adventures of Sherlock Holmes".to_string(),
            author: Some("Arthur Conan Doyle".to_string()),
        };
        let beowulf = BookMeta {
            title: "Beowulf".to_string(),
            author: None,
        };
        let added = library
            .add("/books/holmes.epub", &holmes)
            .expect("add holmes");
        library
            .add("/books/beowulf.epub", &beowulf)
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
}
