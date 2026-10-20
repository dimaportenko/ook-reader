use rusqlite::{params, OptionalExtension};

use crate::db::Db;

impl Db {
    pub(crate) fn save_global_prompt_template(
        &self,
        template: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO global_prompt_template (id, template)
            VALUES (1, ?1)
            ON CONFLICT(id) DO UPDATE SET template = excluded.template",
            params![template],
        )?;

        Ok(())
    }

    pub(crate) fn clear_global_prompt_template(&self) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM global_prompt_template WHERE id = 1", [])?;

        Ok(())
    }

    pub(crate) fn global_prompt_template(&self) -> Result<Option<String>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT template FROM global_prompt_template WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
    }
}
