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
2. **The keychain behind the trait** — native Apple keyring store, `#[ignore]` test,
   iOS build.
3. **The model as a setting.**
   - **3a. The model value** — `AiModel`, stable slugs and Gemini API names.
   - **3b. Persist the model setting** — `Settings` field, migration and db round trip.
   - **3c. Give Gemini the chosen model** — `Gemini::with_model` and endpoint check.
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

---

## Step 2 — The keychain behind the trait

> **Status:** done — committed in `17dcae9` (156 tests green, 2 ignored; the ignored
> keychain round trip and the iOS build passed separately).

**What changed from the plan.** The phase doc named `keyring` 4's `v1` API. Reading the
crate source rules that out: `keyring::v1::Entry::new` returns `NoDefaultStore` on iOS
unconditionally — the convenience layer only wires a default store for macOS, Windows and
desktop Linux. The pieces underneath are fine, and they are what we use directly:
`keyring-core` (the `Entry` type, the `Error` enum, `set_default_store`) plus
`apple-native-keyring-store` (two stores: `keychain::Store`, the classic login keychain on
macOS, and `protected::Store`, the data-protection keychain iOS uses). Same code, two more
lines of `Cargo.toml`, and no wrapper deciding for us which platforms count.

**Check — two kinds.** The error translation is pure and gets real `#[test]`s; the keychain
round trip is the phase's `#[ignore]` test, like Phase 17's live call.

`Cargo.toml`:

```toml
[dependencies]
keyring-core = "1"

[target.'cfg(any(target_os = "macos", target_os = "ios"))'.dependencies]
apple-native-keyring-store = { version = "1", features = ["keychain", "protected"] }
```

Then `src/secrets/keychain.rs`, with `#[cfg(any(target_os = "macos", target_os = "ios"))]
mod keychain;` in `secrets/mod.rs`, and the tests first:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::secrets::{SecretStore, GEMINI_API_KEY};
    use keyring_core::Error;

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

        assert!(matches!(read, Err(SecretError::Unavailable(_))), "got {read:?}");
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

        let _ = store.forget(GEMINI_API_KEY);
    }
}
```

Run `cargo test secrets::` for the three, then `cargo test secrets:: -- --ignored` once
for the round trip (macOS may ask for keychain access the first time — allow it; that
prompt is the login keychain doing its job). Finish with `dx build --platform ios`: that is
where a missing feature or a linker complaint about Security.framework would show.

Delete the last line of the ignored test before committing if you like — it is there only
so a stray key from manual poking never survives a test run.

**Minimal code** — `src/secrets/keychain.rs`:

```rust
use std::sync::Arc;

use keyring_core::{Entry, Error};

use super::{SecretError, SecretStore};

const SERVICE: &str = "com.dimaportenko.ook-reader";

pub(crate) struct Keychain;

impl Keychain {
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
```

(`Store::new()` returns `Result<Arc<Store>, Error>`; `set_default_store` takes the `Arc`
— the `use std::sync::Arc` is only needed if you name the type. Drop it if clippy says
unused.)

**Why it works.**

- **The one pure function is the whole translation.** `absent_as_none` is where the
  crate's vocabulary (a missing entry is an `Err`) becomes the trait's (a missing entry is
  `Ok(None)`). It is generic over `T` so the same function serves `get` (where `T` is
  `String`) and `forget` (where `T` is `()` and the `Some`/`None` is thrown away with
  `.map(|_| ())`). Three real tests cover it without a keychain in sight — that is the
  Phase 17 split again: the translation is testable, the socket is not.
- **`match` on the error, not `if let`.** There are three outcomes and they are different
  *kinds* of thing: a value, a normal absence, a real failure. A `match` with three arms
  reads like the truth table; `if let Err(Error::NoEntry)` would bury the third case in an
  `else`. `Error::NoEntry` is a unit variant so it can be matched without binding anything.
- **`Store::new()` once, in `new`, into a process-wide default.** `keyring-core` keeps the
  store in a global (`set_default_store`), and `Entry::new` reads it. That global is the
  crate's design, not ours; wrapping it in `Keychain::new()` means the rest of the app
  never sees it and `Keychain` is the only value that can reach it. Calling it twice is
  harmless (the second replaces the first), so the `Keychain` value stays a unit struct
  — there is nothing to hold.
- **`cfg` picks the store, the code below it is identical.** macOS gets the classic
  keychain because the data-protection one needs a signed, entitled binary and
  `dx serve` builds are neither; iOS has *only* the data-protection keychain. The `#[cfg]`
  sits on the `let` so the two binaries differ in exactly one line. Linux and Windows are
  not covered on purpose — nothing in the roadmap builds there yet — and the module-level
  `cfg` in `mod.rs` keeps the crate compiling for them rather than failing at link time.
- **`SERVICE` is the bundle id.** The keychain keys items by service + account. Using the
  same identifier the bundle uses means a user looking in Keychain Access sees the app's
  name, and the sync token in Milestone 5 lands in the same drawer.
- **`uuid` in the ignored test** because the real keychain is shared with every other run
  and with the app itself: a fixed test name would collide with a key you set by hand.
  `uuid` is already a dependency.

**Scope note.** Nothing reads `GEMINI_API_KEY` yet; Step 4 wires the store into `main.rs`
and feeds `Gemini`. The `Memory` store stays the one tests use everywhere else. No
"unavailable" UI copy yet — Step 5 shows it on the row.

---

## Step 3a — The model value

> **Status:** done — committed in `8643600` (159 tests green, 2 ignored).

**The crux.** The saved value and the provider's model name look similar, but they have
different stability promises. Save a short app-owned slug such as `flash`; map it to
Google's exact endpoint name at the boundary. Then a future API-model upgrade changes one
match arm instead of invalidating every existing settings row.

Google's model catalogue currently lists `gemini-3.5-flash-lite` and
`gemini-3.5-flash` as stable endpoint names:
[Gemini API models](https://ai.google.dev/gemini-api/docs/models) (checked 2026-09-13).

**Check (`cargo test settings::ai_model`)** — pure Rust. Add `pub mod ai_model;` to
`src/settings/mod.rs`, create `src/settings/ai_model.rs`, and begin with these tests:

```rust
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
        assert_eq!(
            AiModel::FlashLite.api_name(),
            "gemini-3.5-flash-lite"
        );
        assert_eq!(AiModel::Flash.api_name(), "gemini-3.5-flash");
    }
}
```

With only the module declaration and tests present, the check fails because `AiModel` does
not exist. That is the red target.

**Minimal implementation** — above the test module in `src/settings/ai_model.rs`:

```rust
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

    pub(crate) fn from_slug(slug: &str) -> AiModel {
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
}
```

Then rerun `cargo test settings::ai_model` and `cargo clippy`.

**Why it works.** The enum makes unsupported choices unrepresentable inside the app, and
`Copy` keeps it compatible with the existing `Copy` `Settings` signal. `Default` marks
Flash-Lite as both the first-launch choice and the safe fallback for an unknown stored
slug. `ALL` gives Step 5's picker one ordered source of choices. Keeping `slug()` separate
from `api_name()` prevents persistence from depending on Google's versioned endpoint text.

**Scope note.** Do not add an `AiModel` field to `Settings`, touch SQLite, or change
`Gemini` yet. Step 3b persists this value; Step 3c passes its API name to the provider.
