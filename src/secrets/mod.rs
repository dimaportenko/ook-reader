use std::{cell::RefCell, collections::HashMap};

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod keychain;

pub(crate) const GEMINI_API_KEY: &str = "gemini-api-key";

#[derive(Debug, thiserror::Error)]
pub(crate) enum SecretError {
    #[error("the secret store is unavailable: {0}")]
    Unavailable(String),
}

pub(crate) trait SecretStore {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, name: &str, value: &str) -> Result<(), SecretError>;
    fn forget(&self, name: &str) -> Result<(), SecretError>;
}

#[derive(Debug, Default)]
pub(crate) struct Memory {
    entries: RefCell<HashMap<String, String>>,
}

impl SecretStore for Memory {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError> {
        Ok(self.entries.borrow().get(name).cloned())
    }

    fn set(&self, name: &str, value: &str) -> Result<(), SecretError> {
        self.entries
            .borrow_mut()
            .insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn forget(&self, name: &str) -> Result<(), SecretError> {
        self.entries.borrow_mut().remove(name);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_missing_secret_reads_as_none() {
        let store = Memory::default();

        assert_eq!(store.get(GEMINI_API_KEY).expect("read"), None);
    }

    #[test]
    fn a_secret_round_trips_and_the_latest_set_wins() {
        let store = Memory::default();

        store.set(GEMINI_API_KEY, "first").expect("first set");
        assert_eq!(
            store.get(GEMINI_API_KEY).expect("read").as_deref(),
            Some("first")
        );

        store.set(GEMINI_API_KEY, "second").expect("second set");
        assert_eq!(
            store.get(GEMINI_API_KEY).expect("read").as_deref(),
            Some("second")
        );
    }

    #[test]
    fn forgetting_removes_the_secret_and_is_idempotent() {
        let store = Memory::default();
        store.set(GEMINI_API_KEY, "gone soon").expect("set");

        store.forget(GEMINI_API_KEY).expect("first forget");
        assert_eq!(store.get(GEMINI_API_KEY).expect("read"), None);

        store
            .forget(GEMINI_API_KEY)
            .expect("a second forget is a no-op");
    }

    #[test]
    fn secrets_are_keyed_by_name() {
        let store = Memory::default();
        store.set(GEMINI_API_KEY, "gemini").expect("set");

        assert_eq!(store.get("something-else").expect("read"), None);
    }
}
