use std::rc::Rc;
#[cfg(test)]
use std::{cell::RefCell, collections::HashMap};

use crate::ui::OrLog;

pub(crate) mod api_key;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod keychain;

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

pub(crate) fn open_native() -> Option<Rc<dyn SecretStore>> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        keychain::Keychain::new()
            .or_log("open the secret store")
            .map(|store| Rc::new(store) as Rc<dyn SecretStore>)
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        None
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Memory {
    entries: RefCell<HashMap<String, String>>,
}

#[cfg(test)]
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

        assert_eq!(store.get("a-secret").expect("read"), None);
    }

    #[test]
    fn a_secret_round_trips_and_the_latest_set_wins() {
        let store = Memory::default();

        store.set("a-secret", "first").expect("first set");
        assert_eq!(
            store.get("a-secret").expect("read").as_deref(),
            Some("first")
        );

        store.set("a-secret", "second").expect("second set");
        assert_eq!(
            store.get("a-secret").expect("read").as_deref(),
            Some("second")
        );
    }

    #[test]
    fn forgetting_removes_the_secret_and_is_idempotent() {
        let store = Memory::default();
        store.set("a-secret", "gone soon").expect("set");

        store.forget("a-secret").expect("first forget");
        assert_eq!(store.get("a-secret").expect("read"), None);

        store
            .forget("a-secret")
            .expect("a second forget is a no-op");
    }

    #[test]
    fn secrets_are_keyed_by_name() {
        let store = Memory::default();
        store.set("a-secret", "gemini").expect("set");

        assert_eq!(store.get("something-else").expect("read"), None);
    }
}
