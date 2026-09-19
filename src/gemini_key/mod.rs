use crate::{
    ai::gemini::Gemini,
    secrets::{SecretError, SecretStore, GEMINI_API_KEY},
    settings::ai_model::AiModel,
};

pub(crate) fn gemini_from(
    store: &dyn SecretStore,
    model: AiModel,
) -> Result<Option<Gemini>, SecretError> {
    Ok(store
        .get(GEMINI_API_KEY)?
        .map(|key| Gemini::new(key, model.api_name())))
}

pub(crate) fn save(
    store: &dyn SecretStore,
    key: &str,
    model: AiModel,
) -> Result<Gemini, SecretError> {
    let key = key.trim();
    store.set(GEMINI_API_KEY, key)?;
    Ok(Gemini::new(key.to_owned(), model.api_name()))
}

pub(crate) fn forget(store: &dyn SecretStore) -> Result<(), SecretError> {
    store.forget(GEMINI_API_KEY)
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

        store.set(GEMINI_API_KEY, "key").expect("store the key");
        assert!(gemini_from(&store, AiModel::Flash)
            .expect("read the stored key")
            .is_some());

        store.forget(GEMINI_API_KEY).expect("forget the key");
        assert!(gemini_from(&store, AiModel::Flash)
            .expect("read after forgetting")
            .is_none());
    }

    #[test]
    fn saving_a_key_stores_it_trimmed_and_readies_a_provider() {
        let store = Memory::default();

        save(&store, "  key\n", AiModel::Flash).expect("save the key");

        assert_eq!(
            store.get(GEMINI_API_KEY).expect("read back").as_deref(),
            Some("key")
        );
    }

    #[test]
    fn forgetting_a_key_leaves_nothing_for_the_next_launch_to_read() {
        let store = Memory::default();
        save(&store, "key", AiModel::Flash).expect("save the key");

        forget(&store).expect("forget the key");

        assert!(gemini_from(&store, AiModel::Flash)
            .expect("read after forgetting")
            .is_none());
    }
}
