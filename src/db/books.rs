use std::path::Path;

use rusqlite::{params, OptionalExtension, Row};

use crate::db::Db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Book {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) author: Option<String>,
    pub(crate) cover_path: Option<String>,
    pub(crate) added_at: i64,
    pub(crate) last_opened_at: Option<i64>,
}

impl Book {
    pub(crate) fn cover_name(&self) -> Option<&str> {
        self.cover_path
            .as_deref()
            .and_then(|cover| Path::new(cover).file_name())
            .and_then(|name| name.to_str())
    }
}

pub(crate) struct NewBook<'a> {
    pub(crate) path: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) title: &'a str,
    pub(crate) author: Option<&'a str>,
    pub(crate) cover_path: Option<&'a str>,
    pub(crate) added_at: i64,
}

impl Db {
    fn read_book(row: &Row<'_>) -> rusqlite::Result<Book> {
        Ok(Book {
            id: row.get(0)?,
            path: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            cover_path: row.get(4)?,
            added_at: row.get(5)?,
            last_opened_at: row.get(6)?,
        })
    }

    fn read_managed_paths(row: &Row<'_>) -> rusqlite::Result<(String, Option<String>)> {
        Ok((row.get(0)?, row.get(1)?))
    }

    pub(crate) fn list_books(&self) -> Result<Vec<Book>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, author, cover_path, added_at, last_opened_at
            FROM books
            ORDER BY COALESCE(last_opened_at, added_at) DESC, title",
        )?;
        let rows = stmt.query_map([], Self::read_book)?;

        rows.collect()
    }

    pub(crate) fn upsert_book(&self, book: NewBook<'_>) -> Result<Book, rusqlite::Error> {
        self.conn.query_row(
            "INSERT INTO books (path, source_path, title, author, cover_path, added_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(source_path) DO UPDATE SET
                path = excluded.path,
                title = excluded.title,
                author = excluded.author,
                cover_path = excluded.cover_path
            RETURNING id, path, title, author, cover_path, added_at, last_opened_at",
            params![
                book.path,
                book.source_path,
                book.title,
                book.author,
                book.cover_path,
                book.added_at
            ],
            Self::read_book,
        )
    }

    pub(crate) fn touch_opened(&self, id: i64, now: i64) -> Result<bool, rusqlite::Error> {
        let updated = self.conn.execute(
            "UPDATE books SET last_opened_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;

        Ok(updated == 1)
    }

    pub(crate) fn managed_paths_for_source(
        &self,
        source_path: &str,
    ) -> Result<Option<(String, Option<String>)>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT path, cover_path FROM books WHERE source_path = ?1",
                params![source_path],
                Self::read_managed_paths,
            )
            .optional()
    }

    pub(crate) fn delete_book(
        &self,
        id: i64,
    ) -> Result<Option<(String, Option<String>)>, rusqlite::Error> {
        self.conn
            .query_row(
                "DELETE FROM books WHERE id = ?1 RETURNING path, cover_path",
                params![id],
                Self::read_managed_paths,
            )
            .optional()
    }
}
