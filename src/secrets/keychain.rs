use keyring_core::{Entry, Error};

use super::{SecretError, SecretStore};

const SERVICE: &str = "com.dimaportenko.ook-reader";

pub(crate) struct Keychain;

impl Keychain {
    /// Opens the Apple keychain and installs it as `keyring-core`'s default store.
    ///
    /// `set_default_store` is process-global: a second call replaces the first store for
    /// every `Entry` in the process. `App` calls this once, inside `use_hook`, and nothing
    /// else may.
    pub(crate) fn new() -> Result<Self, SecretError> {
        #[cfg(target_os = "macos")]
        let store = apple_native_keyring_store::keychain::Store::new();
        #[cfg(target_os = "ios")]
        let store = apple_native_keyring_store::protected::Store::new();

        keyring_core::set_default_store(store.map_err(unavailable)?);
        Ok(Keychain)
    }

    fn entry(name: &str) -> Result<Entry, SecretError> {
        Entry::new(SERVICE, name).map_err(unavailable)
    }
}

impl SecretStore for Keychain {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError> {
        absent_as_none(Self::entry(name)?.get_password())
    }

    fn set(&self, name: &str, value: &str) -> Result<(), SecretError> {
        Self::entry(name)?.set_password(value).map_err(unavailable)
    }

    fn forget(&self, name: &str) -> Result<(), SecretError> {
        absent_as_none(Self::entry(name)?.delete_credential()).map(|_| ())
    }
}

fn absent_as_none<T>(result: Result<T, Error>) -> Result<Option<T>, SecretError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(other) => Err(unavailable(other)),
    }
}

fn unavailable(error: Error) -> SecretError {
    SecretError::Unavailable(error.to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_missing_entry_translates_to_none() {
        let read: Result<Option<String>, SecretError> = absent_as_none(Err(Error::NoEntry));

        assert_eq!(read.expect("missing is not an error"), None);
    }

    #[test]
    fn a_found_entry_translates_to_some() {
        let read = absent_as_none(Ok("sk-…".to_owned()));

        assert_eq!(read.expect("found").as_deref(), Some("sk-…"));
    }

    #[test]
    fn any_other_keychain_error_is_unavailable() {
        let read = absent_as_none::<String>(Err(Error::NoDefaultStore));

        assert!(
            matches!(read, Err(SecretError::Unavailable(_))),
            "got {read:?}"
        );
    }

    #[test]
    #[ignore = "touches the real keychain"]
    fn a_secret_round_trips_through_the_keychain() {
        let store = Keychain::new().expect("keychain store");
        let name = format!("test-{}", uuid::Uuid::new_v4());

        assert_eq!(store.get(&name).expect("read before set"), None);

        store.set(&name, "round trip").expect("set");
        assert_eq!(
            store.get(&name).expect("read after set").as_deref(),
            Some("round trip")
        );

        store.forget(&name).expect("forget");
        assert_eq!(store.get(&name).expect("read after forget"), None);
        store.forget(&name).expect("a second forget is a no-op");
    }
}
