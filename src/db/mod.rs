use std::path::Path;

use rusqlite::Connection;

mod books;
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
                max_line_length INTEGER NOT NULL,
                ai_model TEXT NOT NULL DEFAULT 'flash-lite'
            )",
            [],
        )?;

        let ai_model_exists: bool = self.conn.query_row(
            "SELECT EXISTS (
                SELECT 1
                FROM pragma_table_info('settings')
                WHERE name = 'ai_model'
            )",
            [],
            |row| row.get::<_, bool>(0),
        )?;

        if !ai_model_exists {
            self.conn.execute(
                "ALTER TABLE settings
                ADD COLUMN ai_model TEXT NOT NULL DEFAULT 'flash-lite'",
                [],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::settings::ai_model::AiModel;

    #[test]
    fn a_pre_model_settings_row_is_upgraded_once_with_the_default_slug() {
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
                    max_line_length INTEGER NOT NULL
                );
                INSERT INTO settings
                    (id, theme, font_family, font_size, line_height, page_margins, max_line_length)
                VALUES (1, 'night', 'humanist', 125, 170, 150, 55);",
            )
            .expect("seed legacy settings");
        drop(legacy);

        let db = Db::open(dir.path()).expect("migrate");
        drop(db);
        let db = Db::open(dir.path()).expect("reopen migrated database");
        let slug: String = db
            .conn
            .query_row("SELECT ai_model FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("read migrated model");

        assert_eq!(slug, AiModel::default().slug());
    }
}
