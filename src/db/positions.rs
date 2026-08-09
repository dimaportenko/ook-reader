use rusqlite::{params, OptionalExtension};

use crate::db::Db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Locator {
    pub(crate) spine_index: usize,
    pub(crate) selector: String,
}

impl Db {
    pub(crate) fn save_position(
        &self,
        book_id: i64,
        locator: &Locator,
        now: i64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO positions (book_id, spine_index, selector, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(book_id) DO UPDATE SET
                spine_index = excluded.spine_index,
                selector = excluded.selector,
                updated_at = excluded.updated_at",
            params![book_id, locator.spine_index, locator.selector, now],
        )?;

        Ok(())
    }

    pub(crate) fn position(&self, book_id: i64) -> Result<Option<Locator>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT spine_index, selector FROM positions WHERE book_id = ?1",
                params![book_id],
                |row| {
                    Ok(Locator {
                        spine_index: row.get(0)?,
                        selector: row.get(1)?,
                    })
                },
            )
            .optional()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn saving_a_position_stamps_the_injected_clock() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path().join("library.sqlite3")).expect("db opens");
        db.conn
            .execute(
                "INSERT INTO books (id, path, source_path, title, added_at)
                VALUES (1, 'managed.epub', 'source.epub', 'A Book', 0)",
                [],
            )
            .expect("a book for the position to hang off");
        let locator = Locator {
            spine_index: 2,
            selector: "body > p:nth-child(3)".to_string(),
        };

        db.save_position(1, &locator, 2_000).expect("first save");
        db.save_position(1, &locator, 3_000).expect("latest save");

        // `updated_at` is intentionally not part of `Locator`, but the injected clock is
        // still a storage contract worth proving inside this module-level test.
        let updated_at: i64 = db
            .conn
            .query_row(
                "SELECT updated_at FROM positions WHERE book_id = ?1",
                params![1],
                |row| row.get(0),
            )
            .expect("stored timestamp");
        assert_eq!(updated_at, 3_000);
    }
}
