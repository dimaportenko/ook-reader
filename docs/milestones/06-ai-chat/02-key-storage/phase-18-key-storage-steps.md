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
   - **3b. Persist the model setting.**
     - **3b-i. The schema bridge** — add and backfill the model column once.
     - **3b-ii. The model round trip** — `Settings` field, save and load.
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

---

## Step 3b-i — Add and backfill the model column

> **Status:** done — committed in `4283208` (160 tests green, 2 ignored; clippy clean).

**The crux.** `CREATE TABLE IF NOT EXISTS` creates a missing table; it does not reconcile
the columns of a table already on disk. This is the first settings change that has to keep
a real user's existing row, so `Db::open` must distinguish the old schema from the new one
and run `ALTER TABLE` exactly once.

Keep this as a one-column compatibility bridge rather than pulling Phase 11's full SQLx
migration system forward. SQLite exposes read-only PRAGMAs as table-valued functions, so
`pragma_table_info('settings')` can answer whether the column exists with an ordinary
`SELECT … WHERE`:
[SQLite PRAGMA functions](https://www.sqlite.org/pragma.html#pragma_functions) (checked
2026-09-13).

**Check (`cargo test db::test::a_pre_model_settings_row_is_upgraded_once_with_the_default_slug`)**
— pure Rust against a temporary on-disk SQLite database. Add this test module to the bottom
of `src/db/mod.rs`:

```rust
#[cfg(test)]
mod test {
    use super::*;

    use crate::settings::ai_model::AiModel;

    #[test]
    fn a_pre_model_settings_row_is_upgraded_once_with_the_default_slug() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(DB_FILENAME);
        let legacy = Connection::open(&path).expect("open legacy database");
        legacy
            .execute_batch(
                "CREATE TABLE settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    theme TEXT NOT NULL,
                    font_family TEXT NOT NULL,
                    font_size INTEGER NOT NULL,
                    line_height INTEGER NOT NULL,
                    page_margins INTEGER NOT NULL,
                    max_line_length INTEGER NOT NULL
                );
                INSERT INTO settings
                    (id, theme, font_family, font_size, line_height, page_margins, max_line_length)
                VALUES (1, 'night', 'humanist', 125, 170, 150, 55);",
            )
            .expect("seed legacy settings");
        drop(legacy);

        let db = Db::open(dir.path()).expect("migrate");
        drop(db);
        let db = Db::open(dir.path()).expect("reopen migrated database");
        let slug: String = db
            .conn
            .query_row(
                "SELECT ai_model FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("read migrated model");

        assert_eq!(slug, AiModel::default().slug());
    }
}
```

Run it before changing `migrate`: both calls to `Db::open` succeed because the existing
table is otherwise valid, but the final query fails with `no such column: ai_model`. That
is the red target. Reopening before the query is deliberate: an unconditional `ALTER TABLE`
could pass the first open and fail the second.

**Minimal implementation** — in `Db::migrate` in `src/db/mod.rs`:

1. Extend the canonical `CREATE TABLE IF NOT EXISTS settings` statement with the latest
   column:

```sql
ai_model TEXT NOT NULL DEFAULT 'flash-lite'
```

2. Immediately after that `CREATE TABLE` call, inspect an existing table and alter only the
   legacy shape:

```rust
let has_ai_model = self.conn.query_row(
    "SELECT EXISTS (
        SELECT 1
        FROM pragma_table_info('settings')
        WHERE name = 'ai_model'
    )",
    [],
    |row| row.get::<_, bool>(0),
)?;

if !has_ai_model {
    self.conn.execute(
        "ALTER TABLE settings
        ADD COLUMN ai_model TEXT NOT NULL DEFAULT 'flash-lite'",
        [],
    )?;
}
```

Then rerun the focused test, `cargo test`, and `cargo clippy`.

**Why it works.** A fresh database gets the latest schema directly from `CREATE TABLE`.
An existing database keeps its table and takes the `ALTER TABLE` branch; SQLite's
`NOT NULL DEFAULT 'flash-lite'` makes every pre-model row immediately readable without
discarding its other settings. On later opens the PRAGMA query sees the column, so the
branch is skipped. A crash before the `ALTER` leaves the old shape to retry; a crash after
it leaves the column visible, which makes the retry a no-op.

The literal duplicates Step 3a's default slug intentionally. A migration records what the
default meant when that schema version shipped; changing `AiModel::default()` later must
not rewrite history. The test pins today's two meanings together.

**Scope note.** Do not add `ai_model` to `Settings`, `save_settings`, or `settings()` yet.
This step only makes old and fresh databases share the same column. Step 3b-ii gives the
column a Rust field and proves the full save/load round trip. Do not introduce
`PRAGMA user_version` here: Phase 11 replaces this one-column bridge with SQLx's versioned
migrations.

---

## Step 3b-ii — Persist the model value

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `6d1cd29` (160 tests green, 2 ignored; clippy clean).

**The crux.** The column exists, but it is still disconnected from Rust: `Settings` cannot
carry a model and both SQL statements ignore it. The compiler can keep every Rust struct
literal honest, but it cannot see through SQL strings, so the existing round-trip test has
to exercise both the first insert and the conflict-update path with different models.

**Check (`cargo test db::settings::test`)** — strengthen the two existing tripwires in
`src/db/settings.rs` before changing the implementation.

Import `AiModel`, then make the first saved row non-default and make the second save switch
back to the default:

```rust
let saved = Settings {
    theme: Theme::Night,
    font_family: FontFamily::Humanist,
    font_size: 125,
    line_height: 170,
    page_margins: 150,
    max_line_length: 55,
    ai_model: AiModel::Flash,
};

let latest = Settings {
    theme: Theme::Sepia,
    ai_model: AiModel::FlashLite,
    ..saved
};
```

Give the fixture in `every_settings_field_differs_from_the_default_in_the_round_trip` the
same `ai_model: AiModel::Flash`, then add its tripwire:

```rust
assert_ne!(saved.ai_model, default.ai_model);
```

Run the focused module now. It fails to compile because `Settings` has no `ai_model` field;
that is the red target. The two model values are deliberate: if the SQL `INSERT` carries
the model but `ON CONFLICT … DO UPDATE` forgets it, the second read must fail instead of
quietly keeping `Flash`.

One existing assertion message in `src/settings/mod.rs` also becomes stale in this step.
Keep `vars.len()` at `theme.css_vars().len() + 5` — an AI model is not CSS — but change
“bump this when a setting is added” to “bump this when a CSS-backed setting is added”.

**Minimal implementation.** First, let `Settings` carry the value in `src/settings/mod.rs`:

```rust
use crate::settings::{ai_model::AiModel, font::FontFamily, theme::Theme};

pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_family: FontFamily,
    pub(crate) font_size: u16,
    pub(crate) line_height: u16,
    pub(crate) page_margins: u16,
    pub(crate) max_line_length: u16,
    pub(crate) ai_model: AiModel,
}
```

Add `ai_model: AiModel::default()` to the `Default` implementation. Then extend both SQL
directions in `src/db/settings.rs`.

The save statement gets an eighth column, a seventh parameter, and a conflict update:

```rust
"INSERT INTO settings
    (id, theme, font_family, font_size, line_height, page_margins, max_line_length, ai_model)
VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
ON CONFLICT(id) DO UPDATE SET
    theme = excluded.theme,
    font_family = excluded.font_family,
    font_size = excluded.font_size,
    line_height = excluded.line_height,
    page_margins = excluded.page_margins,
    max_line_length = excluded.max_line_length,
    ai_model = excluded.ai_model"
```

Append `settings.ai_model.slug()` to `params!`. The load statement appends `ai_model` to
the `SELECT` list and reconstructs the enum from column index 6:

```rust
ai_model: AiModel::from_slug(&row.get::<_, String>(6)?),
```

Then rerun `cargo test db::settings::test`, `cargo test`, and `cargo clippy`.

**Why it works.** `Settings` remains `Copy` because `AiModel` is `Copy`, so none of the
signal or struct-update behavior changes. SQLite receives the app-owned slug rather than
Google's endpoint name, preserving the stability boundary from Step 3a. On load,
`from_slug` turns valid text back into the enum and degrades an unknown future or corrupted
value to Flash-Lite without losing the other six settings.

Appending the SQL column keeps the existing row indices stable; only index 6 is new. The
non-default first fixture catches a missing insert parameter, while switching back to
Flash-Lite catches a missing conflict update. The separate “every field differs” assertion
prevents a future fixture edit from accidentally taking those teeth away.

**Scope note.** Do not change the migration, `Gemini`, context, or the settings UI. Keep the
temporary `#[allow(dead_code)]` on `settings::ai_model`: `ALL` remains unused until the
picker in Step 5. Step 3c is next and passes `settings.ai_model.api_name()` into Gemini.

---

## Step 3c — Give Gemini the chosen model

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `79ae06b` (161 tests green, 2 ignored; clippy clean).

**The crux.** `Gemini` already stores its model and builds the request URL from that field,
but callers can only construct the default. The smallest bridge is a builder that replaces
the default before the provider is shared. The check must read the resulting endpoint, not
only the field, so it proves the value reaches the part of the request where it matters.

**Check (`cargo test ai::gemini::test::a_chosen_model_reaches_the_endpoint`)** — add one
pure test beside the existing endpoint test:

```rust
#[test]
fn a_chosen_model_reaches_the_endpoint() {
    let gemini = Gemini::new("key".to_owned()).with_model("gemini-3.5-flash");

    assert_eq!(
        endpoint(&gemini.model),
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent"
    );
}
```

Run it before adding the builder. It fails to compile with `E0599`: no method named
`with_model` exists on `Gemini`. That is the red target.

**Minimal implementation** — add the builder to `impl Gemini`:

```rust
pub(crate) fn with_model(mut self, model: impl Into<String>) -> Self {
    self.model = model.into();
    self
}
```

Then rerun the focused test, `cargo test`, and `cargo clippy --all-targets`.

**Why it works.** `new` remains the one constructor and preserves Flash-Lite as the default.
`with_model` consumes that complete value, changes only its model, and returns it ready to
put into context; there is no partly configured provider for a caller to keep using by
mistake. `impl Into<String>` accepts the `&'static str` returned by `AiModel::api_name()`
without making `Gemini` depend on the settings enum, while the provider still owns the name
for as long as it exists. The test feeds the stored value through the same `endpoint`
function used by `complete`, pinning the whole model-to-URL path without touching the
network.

The cleanup pass kept the first draft unchanged: the builder matches the existing
`Message` constructors' `impl Into<String>` idiom, and extracting another helper or exposing
the model would add surface without making this two-line state change clearer.

**Scope note.** This step does not read `Settings`, rebuild a provider, or put one in Dioxus
context. Step 4 joins `settings.ai_model.api_name()` with the key at launch and owns the
provider lifecycle. Step 5 exposes the model choice in the settings UI.

---

## Step 4 — The provider in context

**The crux.** `main.rs` is the composition root: it is the one place allowed to know that
the native `SecretStore`, the persisted `AiModel`, and `Gemini` belong together. Keep that
join in a pure helper, then put only the finished `Option<Gemini>` in a signal. The keychain
itself is not reactive, so a model change can rerun a Dioxus effect, while Step 5 will call
the same helper explicitly after saving or forgetting a key — without ever putting the key
in its own signal.

**Check (`cargo test test::a_stored_key_controls_provider_availability`)** — pure Rust.
Add this test to the existing `test` module at the bottom of `src/main.rs` before writing
the helper:

```rust
#[test]
fn a_stored_key_controls_provider_availability() {
    let store = secrets::Memory::default();

    assert!(
        load_gemini(&store, AiModel::FlashLite)
            .expect("read a missing key")
            .is_none()
    );

    store.set(GEMINI_API_KEY, "key").expect("store the key");
    assert!(
        load_gemini(&store, AiModel::Flash)
            .expect("read the stored key")
            .is_some()
    );

    store.forget(GEMINI_API_KEY).expect("forget the key");
    assert!(
        load_gemini(&store, AiModel::Flash)
            .expect("read after forgetting")
            .is_none()
    );
}
```

It fails to compile because `load_gemini` does not exist. The non-default `Flash` value
also makes the test compile through the model-selection path. Steps 3a and 3c already pin
the two halves of that path — `AiModel::api_name()` and `Gemini::with_model` reaching the
endpoint — so this test only needs to pin the new composition rule: missing key means no
provider, stored key means one is ready, and forgetting returns to none.

After the pure check is green, run `cargo test`, `cargo clippy --all-targets`, and
`dx serve --platform desktop`. The visual check has intentionally no new UI: the normal
library or reader must open without a keychain panic or visible regression. On a machine
whose keychain opens normally, the terminal should not report that it could not open the
secret store or read the Gemini API key.

**Minimal implementation.** Extend `src/main.rs`'s existing `use crate::{...}` block with
these types:

```rust
ai::gemini::Gemini,
secrets::{keychain::Keychain, SecretError, SecretStore, GEMINI_API_KEY},
settings::ai_model::AiModel,
```

Put the helper above `main`:

```rust
fn load_gemini(
    store: &dyn SecretStore,
    model: AiModel,
) -> Result<Option<Gemini>, SecretError> {
    Ok(store
        .get(GEMINI_API_KEY)?
        .map(|key| Gemini::new(key).with_model(model.api_name())))
}
```

Then, in `App`, immediately after the `settings` hook, create the native store, the
provider signal, and a memo containing only the model:

```rust
let secret_store = use_hook(|| {
    Keychain::new()
        .or_log("open the secret store")
        .map(|store| Rc::new(store) as Rc<dyn SecretStore>)
});
let mut provider = use_signal(|| None::<Gemini>);
let ai_model = use_memo(move || settings().ai_model);
```

Provide both handles beside the existing contexts:

```rust
use_context_provider(|| secret_store.clone());
use_context_provider(|| provider);
```

Finally, add an effect beside the settings-persistence and root-theme effects:

```rust
use_effect({
    let secret_store = secret_store.clone();
    move || {
        provider.set(secret_store.as_deref().and_then(|store| {
            load_gemini(store, ai_model())
                .or_log("read the Gemini API key")
                .flatten()
        }));
    }
});
```

Keep the store context's inferred type as `Option<Rc<dyn SecretStore>>`. `None` here means
the native store itself could not be opened; Step 5 can distinguish that from an available
store whose `get` returns no key and show the appropriate status without constructing a
second `Keychain` inside the UI.

**Why it works.**

- **`Result<Option<Gemini>, SecretError>` preserves all three outcomes.** `Ok(None)` is the
  ordinary first-launch or forgotten-key state, `Ok(Some(_))` means the provider is ready,
  and `Err` means the system service failed. `?` propagates only that third case; `map`
  transforms a present key without exposing it anywhere else.
- **The secret remains confined.** The temporary `String` returned by `get` moves straight
  into `Gemini`. Dioxus stores the provider, not a second copy of the key, so no component
  can accidentally render or retain the raw secret as UI state.
- **`Rc<dyn SecretStore>` shares the boundary, not the concrete keychain.** `App` constructs
  the native implementation once and descendants receive only the trait they need. Step 5
  can therefore exercise the same UI operations against `Memory` in a later pure helper
  without teaching the settings code about Apple APIs.
- **`Signal<Option<Gemini>>` is a stable context handle.** Replacing its value schedules
  consumers that read it; Phase 19 can react to provider availability without recreating
  the context value or knowing where the provider came from.
- **The memo narrows reactivity to the model.** An effect reruns when it reads a reactive
  value that changes. Reading the whole `Settings` signal directly would rebuild a
  `reqwest::Client` after every font, margin, or theme adjustment. `use_memo` still follows
  `settings`, but only notifies this effect when its `AiModel` output actually differs.
- **Store construction degrades to unavailable instead of ending the app.** `or_log` turns
  a native-store setup failure into `None`; that leaves the provider absent and gives Step
  5 a state it can display. A normal missing key remains a different state because the
  store context is still `Some`.

**Scope note.** This step does not add controls or display key status, and it does not make
the keychain reactive. Step 5 consumes `Option<Rc<dyn SecretStore>>` and
`Signal<Option<Gemini>>`; after a successful `set` or `forget`, its event handler calls
`load_gemini` and replaces the provider immediately. Changing `settings.ai_model` needs no
extra call because the memo-driven effect handles it. Do not add key validation here — the
first real request in Phase 19 remains the validator.
