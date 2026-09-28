use crate::{
    ai::Provider,
    secrets::{SecretError, SecretStore},
};

const GEMINI_API_KEY: &str = "gemini-api-key";
const OPENCODE_ZEN_API_KEY: &str = "opencode-zen-api-key";

fn entry(provider: Provider) -> &'static str {
    match provider {
        Provider::Gemini => GEMINI_API_KEY,
        Provider::OpenCodeZen => OPENCODE_ZEN_API_KEY,
    }
}

pub(crate) fn get(
    store: &dyn SecretStore,
    provider: Provider,
) -> Result<Option<String>, SecretError> {
    store.get(entry(provider))
}

pub(crate) fn is_set(store: &dyn SecretStore, provider: Provider) -> Result<bool, SecretError> {
    Ok(get(store, provider)?.is_some())
}

pub(crate) fn save(
    store: &dyn SecretStore,
    provider: Provider,
    key: &str,
) -> Result<String, SecretError> {
    let key = key.trim();
    store.set(entry(provider), key)?;
    Ok(key.to_owned())
}

pub(crate) fn forget(store: &dyn SecretStore, provider: Provider) -> Result<(), SecretError> {
    store.forget(entry(provider))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::secrets::Memory;

    #[test]
    fn saving_a_key_stores_it_trimmed_and_hands_it_back() {
        let store = Memory::default();

        let saved = save(&store, Provider::OpenCodeZen, "  key\n").expect("save the key");

        assert_eq!(saved, "key");
        assert!(is_set(&store, Provider::OpenCodeZen).expect("read the key"));
    }

    #[test]
    fn each_provider_keeps_its_key_in_its_own_entry() {
        let store = Memory::default();

        save(&store, Provider::OpenCodeZen, "zen").expect("save the Zen key");
        assert!(!is_set(&store, Provider::Gemini).expect("read the Gemini key"));

        save(&store, Provider::Gemini, "gemini").expect("save the Gemini key");
        forget(&store, Provider::OpenCodeZen).expect("forget the Zen key");

        assert!(is_set(&store, Provider::Gemini).expect("read the Gemini key"));
        assert!(!is_set(&store, Provider::OpenCodeZen).expect("read the Zen key"));
    }
}
