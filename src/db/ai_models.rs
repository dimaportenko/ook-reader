use rusqlite::params;

use crate::{ai::Provider, db::Db};

impl Db {
    pub(crate) fn ticked_models(&self, provider: Provider) -> Result<Vec<String>, rusqlite::Error> {
        let mut statement = self.conn.prepare(
            "SELECT model_id FROM ai_models WHERE provider = ?1 ORDER BY model_id",
        )?;
        let ids = statement.query_map(params![provider.slug()], |row| row.get(0))?;

        ids.collect()
    }

    pub(crate) fn set_ticked(
        &self,
        provider: Provider,
        model_id: &str,
        ticked: bool,
    ) -> Result<(), rusqlite::Error> {
        let sql = if ticked {
            "INSERT OR IGNORE INTO ai_models (provider, model_id) VALUES (?1, ?2)"
        } else {
            "DELETE FROM ai_models WHERE provider = ?1 AND model_id = ?2"
        };
        self.conn.execute(sql, params![provider.slug(), model_id])?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ticked_models_round_trip_and_untick_removes_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        assert!(db.ticked_models(Provider::OpenCodeZen).expect("empty").is_empty());

        db.set_ticked(Provider::OpenCodeZen, "kimi-k2.6", true).expect("tick kimi");
        db.set_ticked(Provider::OpenCodeZen, "deepseek-v4-flash", true).expect("tick deepseek");
        db.set_ticked(Provider::OpenCodeZen, "deepseek-v4-flash", true).expect("tick deepseek again");
        assert_eq!(
            db.ticked_models(Provider::OpenCodeZen).expect("read ticks"),
            ["deepseek-v4-flash", "kimi-k2.6"]
        );

        db.set_ticked(Provider::OpenCodeZen, "kimi-k2.6", false).expect("untick kimi");
        assert_eq!(
            db.ticked_models(Provider::OpenCodeZen).expect("read after untick"),
            ["deepseek-v4-flash"]
        );
    }

    #[test]
    fn each_provider_keeps_its_own_ticks() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        db.set_ticked(Provider::OpenCodeZen, "shared-id", true).expect("tick for Zen");

        assert!(db.ticked_models(Provider::Gemini).expect("read Gemini").is_empty());
    }
}
