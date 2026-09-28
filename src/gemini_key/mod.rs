use crate::{
    ai::{gemini::Gemini, Provider},
    secrets::{api_key, SecretError, SecretStore},
    settings::ai_model::AiModel,
};

pub(crate) fn gemini_from(
    store: &dyn SecretStore,
    model: AiModel,
) -> Result<Option<Gemini>, SecretError> {
    Ok(api_key::get(store, Provider::Gemini)?.map(|key| Gemini::new(key, model.api_name())))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::secrets::Memory;

    #[test]
    fn a_stored_key_controls_provider_availability() {
        let store = Memory::default();

        assert!(gemini_from(&store, AiModel::FlashLite)
            .expect("read a missing key")
            .is_none());

        api_key::save(&store, Provider::Gemini, "key").expect("store the key");
        assert!(gemini_from(&store, AiModel::Flash)
            .expect("read the stored key")
            .is_some());

        api_key::forget(&store, Provider::Gemini).expect("forget the key");
        assert!(gemini_from(&store, AiModel::Flash)
            .expect("read after forgetting")
            .is_none());
    }
}
