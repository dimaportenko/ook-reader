use rusqlite::{params, OptionalExtension};

use crate::{
    ai::{ChatModel, Provider},
    db::Db,
};

impl Db {
    pub(crate) fn save_chat_model(&self, model: &ChatModel) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO chat_model (id, provider, model_id)
            VALUES (1, ?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model_id = excluded.model_id",
            params![model.provider.slug(), model.id],
        )?;

        Ok(())
    }

    pub(crate) fn chat_model(&self) -> Result<Option<ChatModel>, rusqlite::Error> {
        let row = self
            .conn
            .query_row(
                "SELECT provider, model_id FROM chat_model WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        Ok(row.and_then(|(provider, id)| {
            Provider::from_slug(&provider).map(|provider| ChatModel { provider, id })
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ai::gemini::GeminiModel;

    #[test]
    fn the_chosen_chat_model_round_trips_and_the_latest_pick_wins() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        assert_eq!(db.chat_model().expect("nothing chosen yet"), None);

        let gemini = ChatModel::gemini(GeminiModel::Flash);
        db.save_chat_model(&gemini).expect("pick a Gemini model");
        assert_eq!(db.chat_model().expect("read the Gemini pick"), Some(gemini));

        db.save_chat_model(&ChatModel::luna()).expect("pick Luna");
        assert_eq!(
            db.chat_model().expect("read the Luna pick"),
            Some(ChatModel::luna())
        );
    }

    #[test]
    fn a_pick_from_an_unknown_provider_reads_as_no_pick() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        db.save_chat_model(&ChatModel::gemini(GeminiModel::Flash))
            .expect("seed the row");
        db.conn
            .execute(
                "UPDATE chat_model SET provider = 'anthropic' WHERE id = 1",
                [],
            )
            .expect("corrupt the provider");

        assert_eq!(db.chat_model().expect("read"), None);
    }

    #[test]
    fn a_saved_pick_from_the_removed_opencode_zen_provider_reads_as_no_pick() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        db.conn
            .execute(
                "INSERT INTO chat_model (id, provider, model_id)
                VALUES (1, 'opencode-zen', 'kimi-k2.6')",
                [],
            )
            .expect("seed a Zen pick");

        assert_eq!(db.chat_model().expect("read"), None);
    }
}
