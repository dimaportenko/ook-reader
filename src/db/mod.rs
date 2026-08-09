use std::path::Path;

use rusqlite::Connection;

mod books;
mod positions;
mod settings;

pub(crate) use books::{Book, NewBook};
pub(crate) use positions::Locator;

pub(crate) struct Db {
    conn: Connection,
}

impl Db {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let db = Db { conn };
        db.migrate()?;

        Ok(db)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                source_path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                author TEXT,
                cover_path TEXT,
                added_at INTEGER NOT NULL,
                last_opened_at INTEGER
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS positions (
                book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                spine_index INTEGER NOT NULL,
                selector TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                theme TEXT NOT NULL,
                font_family TEXT NOT NULL,
                font_size INTEGER NOT NULL,
                line_height INTEGER NOT NULL,
                page_margins INTEGER NOT NULL,
                max_line_length INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }
}
