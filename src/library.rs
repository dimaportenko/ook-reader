use rusqlite::{params, Connection};

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
}

impl Library {
    pub(crate) fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                author TEXT
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub(crate) fn add(&self, path: &str, meta: &BookMeta) -> rusqlite::Result<Book> {
        self.conn.execute(
            "INSERT INTO books (path, title, author) VALUES (?1, ?2, ?3)",
            params![path, meta.title, meta.author],
        )?;
        Ok(Book {
            id: self.conn.last_insert_rowid(),
            path: path.to_string(),
            title: meta.title.clone(),
            author: meta.author.clone(),
        })
    }

    pub(crate) fn list(&self) -> rusqlite::Result<Vec<Book>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, title, author FROM books ORDER BY title")?;
        let rows = stmt.query_map([], |row| {
            Ok(Book {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
            })
        })?;

        rows.collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_then_list_round_trips_books() {
        let library = Library::open_in_memory().expect("in-memory db opens");

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
}
