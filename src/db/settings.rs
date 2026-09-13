use rusqlite::{params, OptionalExtension};

use crate::db::Db;
use crate::settings::{ai_model::AiModel, font::FontFamily, theme::Theme, Settings};

impl Db {
    pub(crate) fn save_settings(&self, settings: &Settings) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO settings
                (id, theme, font_family, font_size, line_height, page_margins, max_line_length, ai_model)
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                theme = excluded.theme,
                font_family = excluded.font_family,
                font_size = excluded.font_size,
                line_height = excluded.line_height,
                page_margins = excluded.page_margins,
                max_line_length = excluded.max_line_length,
                ai_model = excluded.ai_model",
            params![
                settings.theme.slug(),
                settings.font_family.slug(),
                settings.font_size,
                settings.line_height,
                settings.page_margins,
                settings.max_line_length,
                settings.ai_model.slug(),
            ],
        )?;

        Ok(())
    }

    pub(crate) fn settings(&self) -> Result<Option<Settings>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT theme, font_family, font_size, line_height, page_margins, max_line_length, ai_model
                FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok(Settings {
                        theme: Theme::from_slug(&row.get::<_, String>(0)?),
                        font_family: FontFamily::from_slug(&row.get::<_, String>(1)?),
                        font_size: row.get(2)?,
                        line_height: row.get(3)?,
                        page_margins: row.get(4)?,
                        max_line_length: row.get(5)?,
                        ai_model: AiModel::from_slug(&row.get::<_, String>(6)?),
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
    fn settings_round_trip_and_the_latest_save_wins() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        assert_eq!(db.settings().expect("empty settings"), None);

        let saved = Settings {
            theme: Theme::Night,
            font_family: FontFamily::Humanist,
            font_size: 125,
            line_height: 170,
            page_margins: 150,
            max_line_length: 55,
            ai_model: AiModel::Flash,
        };
        db.save_settings(&saved).expect("first save");
        assert_eq!(db.settings().expect("first read"), Some(saved));

        let latest = Settings {
            theme: Theme::Sepia,
            ai_model: AiModel::FlashLite,
            ..saved
        };
        db.save_settings(&latest).expect("second save");
        assert_eq!(db.settings().expect("second read"), Some(latest));
    }

    #[test]
    fn every_settings_field_differs_from_the_default_in_the_round_trip() {
        let default = Settings::default();
        let saved = Settings {
            theme: Theme::Night,
            font_family: FontFamily::Humanist,
            font_size: 125,
            line_height: 170,
            page_margins: 150,
            max_line_length: 55,
            ai_model: AiModel::Flash,
        };

        assert_ne!(saved.theme, default.theme);
        assert_ne!(saved.font_family, default.font_family);
        assert_ne!(saved.font_size, default.font_size);
        assert_ne!(saved.line_height, default.line_height);
        assert_ne!(saved.page_margins, default.page_margins);
        assert_ne!(saved.max_line_length, default.max_line_length);
        assert_ne!(saved.ai_model, default.ai_model);
    }

    #[test]
    fn an_unknown_stored_theme_slug_falls_back_to_the_default() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open(dir.path()).expect("open");

        db.save_settings(&Settings::default())
            .expect("seed the row");
        db.conn
            .execute("UPDATE settings SET theme = 'chartreuse' WHERE id = 1", [])
            .expect("corrupt the slug");

        let read = db.settings().expect("read").expect("a row exists");
        assert_eq!(read.theme, Theme::default());
    }
}
