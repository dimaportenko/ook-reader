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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn each_choice_names_its_stable_gemini_model() {
        assert_eq!(AiModel::FlashLite.api_name(), "gemini-3.5-flash-lite");
        assert_eq!(AiModel::Flash.api_name(), "gemini-3.5-flash");
    }
}
