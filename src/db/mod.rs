use std::path::Path;

use rusqlite::Connection;

mod ai_models;
mod books;
mod chat_model;
mod positions;
mod settings;

pub(crate) use books::{Book, NewBook};

pub(crate) const DB_FILENAME: &str = "library.sqlite3";

pub(crate) struct Db {
    conn: Connection,
}

impl Db {
    pub(crate) fn open(dir_path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(dir_path.as_ref().join(DB_FILENAME))?;
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

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_models (
                provider TEXT NOT NULL,
                model_id TEXT NOT NULL,
                PRIMARY KEY (provider, model_id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_model (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                provider TEXT NOT NULL,
                model_id TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::settings::{theme::Theme, Settings};

    #[test]
    fn a_settings_table_that_still_has_the_ai_model_column_keeps_saving() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(DB_FILENAME);
        let legacy = Connection::open(&path).expect("open legacy database");
        legacy
            .execute_batch(
                "CREATE TABLE settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    theme TEXT NOT NULL,
                    font_family TEXT NOT NULL,
                    font_size INTEGER NOT NULL,
                    line_height INTEGER NOT NULL,
                    page_margins INTEGER NOT NULL,
                    max_line_length INTEGER NOT NULL,
                    ai_model TEXT NOT NULL DEFAULT 'flash-lite'
                );
                INSERT INTO settings
                    (id, theme, font_family, font_size, line_height, page_margins, max_line_length, ai_model)
                VALUES (1, 'night', 'humanist', 125, 170, 150, 55, 'flash');",
            )
            .expect("seed legacy settings");
        drop(legacy);

        let db = Db::open(dir.path()).expect("open the legacy database");
        let saved = Settings {
            theme: Theme::Sepia,
            ..Settings::default()
        };
        db.save_settings(&saved).expect("save over the legacy row");

        assert_eq!(db.settings().expect("read"), Some(saved));
    }
}
