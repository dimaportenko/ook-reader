# Phase 18 — API key storage + settings row — build log

[← Phase doc](phase-18-key-storage.md)

Per-step check → minimal code → why, appended newest-last. The
[phase doc](phase-18-key-storage.md)'s "Planned steps" checklist is the high-level index;
this file is the detail and the build log.

Baseline when the phase opened: **149 tests green, 1 ignored**, clippy clean (`2b9e826`).

## The crux

**A secret is a setting with a different home, and the home is the only thing the app may
not test.** The keychain gets the Phase 17 treatment: a thin `SecretStore` trait, a `Memory`
store for the suite, a real `Keychain` store behind `#[ignore]`. The UI only ever asks
*is a key set?* and says *save* or *forget*; the key itself flows store → `Gemini` and
never to the screen.

## Step plan

1. **The secret boundary** — trait + error + `Memory` store.
2. **The keychain behind the trait** — `keyring`, `#[ignore]` test, iOS build.
3. **The model as a setting** — `AiModel`, slug, column, `Gemini::with_model`.
4. **The provider in context** — key + model → `Signal<Option<Gemini>>` on launch.
5. **The settings row** — input, status line, forget, model picker.
6. **Review and refactor** — punch-list, suite green, clippy clean.

---

## Step 1 — The secret boundary

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `0d82691` (153 tests green, 1 ignored).

**Check (`cargo test secrets::`)** — pure Rust, `#[test]`. Create `src/secrets/mod.rs`,
add `mod secrets;` to `main.rs` (with the same `#[allow(dead_code)]` treatment `ai` got, for
the same reason), and start with the tests:

```rust
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

        store.forget(GEMINI_API_KEY).expect("a second forget is a no-op");
    }

    #[test]
    fn secrets_are_keyed_by_name() {
        let store = Memory::default();
        store.set(GEMINI_API_KEY, "gemini").expect("set");

        assert_eq!(store.get("something-else").expect("read"), None);
    }
}
```

Three of these fail to compile until the items exist; the fourth pins that the store is a
map, not a single slot. Watch them go red, then green.

**Minimal code** — `src/secrets/mod.rs`, above the test module:

```rust
use std::cell::RefCell;
use std::collections::HashMap;

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
        self.entries.borrow_mut().insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn forget(&self, name: &str) -> Result<(), SecretError> {
        self.entries.borrow_mut().remove(name);
        Ok(())
    }
}
```

**Why it works.**

- **`&self` on all three methods, including the writes.** The keychain is a system service:
  writing to it does not mutate any Rust value you own, so `set` on the real store will be
  `&self` naturally. The trait has to match the *real* implementation's shape, which means
  `Memory` needs interior mutability to keep up — that is what `RefCell` is for. `borrow()`
  and `borrow_mut()` do at runtime what `&`/`&mut` do at compile time, and panic if you
  hold both at once. Here each method takes one short borrow and releases it before
  returning, so it cannot conflict with itself. The alternative — `&mut self` on the trait
  — would force every caller to hold a `&mut` to a store that never actually changes,
  which the UI's shared context would make awkward for no gain.
- **`get` returns `Result<Option<String>, _>`, not `Result<String, _>` with a "not found"
  error.** A missing key is the *normal* state on first launch, not a failure; the UI's
  *not set* line is literally `get(..)? .is_none()`. `keyring` reports missing as
  `Error::NoEntry`, and Step 2's job is to translate that into `Ok(None)` so the trait
  keeps this meaning.
- **`forget` is idempotent by contract.** The forget button may be pressed twice; removing
  a missing entry is a no-op, not an error. The test pins it so Step 2 has to honour it too
  (`keyring` returns `NoEntry` on a second delete — same translation).
- **`&str` in, owned `String` out.** The store gives you a fresh `String` because the
  keychain hands over a copy anyway; `.cloned()` on the `Option<&String>` from the map does
  the same for `Memory`. `.as_deref()` in the tests turns `Option<String>` into
  `Option<&str>` so it compares against a literal without allocating.
- **One error variant for now.** `SecretError::Unavailable(String)` is enough to surface a
  locked keychain or a missing entitlement to the UI as a sentence. If Step 2 finds a second
  *kind* of failure worth acting on differently, it grows a variant then, not
  speculatively.
- **The name is a string constant, not an enum.** There will be a second tenant (the sync
  refresh token, per ADR 0005) but it belongs to another milestone; a `const` per secret
  is the smallest thing that keeps both from typo-ing each other's name.

**Scope note.** Nothing here touches the real keychain — that is Step 2. Nothing here
knows what a key is *for*; `Gemini` is fed from the store in Step 4. `Memory` doubles as
the store for the web target later, if that ever comes, but that is not why it exists.
