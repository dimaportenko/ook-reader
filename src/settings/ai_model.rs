#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AiModel {
    #[default]
    FlashLite,
    Flash,
}

impl AiModel {
    pub(crate) const ALL: [AiModel; 2] = [AiModel::FlashLite, AiModel::Flash];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            AiModel::FlashLite => "flash-lite",
            AiModel::Flash => "flash",
        }
    }

    pub(crate) fn from_slug(slug: &str) -> Self {
        match slug {
            "flash-lite" => AiModel::FlashLite,
            "flash" => AiModel::Flash,
            _ => AiModel::default(),
        }
    }

    pub(crate) fn api_name(self) -> &'static str {
        match self {
            AiModel::FlashLite => "gemini-3.5-flash-lite",
            AiModel::Flash => "gemini-3.5-flash",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            AiModel::FlashLite => "Flash-Lite",
            AiModel::Flash => "Flash",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn every_ai_model_survives_a_slug_round_trip() {
        for model in AiModel::ALL {
            assert_eq!(AiModel::from_slug(model.slug()), model);
        }
    }

    #[test]
    fn an_unknown_ai_model_slug_falls_back_to_flash_lite() {
        assert_eq!(AiModel::from_slug("unknown"), AiModel::FlashLite);
    }

    #[test]
    fn each_choice_names_its_stable_gemini_model() {
        assert_eq!(AiModel::FlashLite.api_name(), "gemini-3.5-flash-lite");
        assert_eq!(AiModel::Flash.api_name(), "gemini-3.5-flash");
    }

    #[test]
    fn each_ai_model_has_a_reader_facing_label() {
        assert_eq!(AiModel::ALL.map(AiModel::label), ["Flash-Lite", "Flash"]);
    }
}
