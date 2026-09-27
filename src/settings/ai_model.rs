use crate::settings::choice::Choice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AiModel {
    #[default]
    FlashLite,
    Flash,
}

impl AiModel {
    pub(crate) const ALL: [AiModel; 2] = [AiModel::FlashLite, AiModel::Flash];

    pub(crate) fn api_name(self) -> &'static str {
        match self {
            AiModel::FlashLite => "gemini-3.5-flash-lite",
            AiModel::Flash => "gemini-3.5-flash",
        }
    }
}

impl Choice for AiModel {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn slug(self) -> &'static str {
        match self {
            AiModel::FlashLite => "flash-lite",
            AiModel::Flash => "flash",
        }
    }

    fn label(self) -> &'static str {
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
    fn each_choice_names_its_stable_gemini_model() {
        assert_eq!(AiModel::FlashLite.api_name(), "gemini-3.5-flash-lite");
        assert_eq!(AiModel::Flash.api_name(), "gemini-3.5-flash");
    }

    #[test]
    fn each_ai_model_has_a_reader_facing_label() {
        assert_eq!(AiModel::ALL.map(AiModel::label), ["Flash-Lite", "Flash"]);
    }
}
