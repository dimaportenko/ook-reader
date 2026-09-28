# Phase 23 — Second provider: OpenCode Zen — build log

[← Phase 23](phase-23-opencode-zen.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

Models stop being a closed set. Zen's list is fetched at runtime, so the chosen model becomes
a value, `ChatModel { provider, id: String }`. It lives beside `Settings`, not inside it,
because `Settings` is `Copy` and a `String` would end that. Providers stay a closed enum,
and `AnyProvider` picks between the two clients at runtime without `dyn`. The Zen wire
format reuses the SSE buffer; only its JSON and the `[DONE]` marker are new.

## Step plan

1. ~~**The Zen block and its key.**~~ `Provider`, a second key row in Settings → AI, and key status read from the store. **Done** — `73e2447`.
2. ~~**The Zen catalog.**~~ Fetch `/zen/v1/models`, show the ids, and parse under `#[test]`. **Done** — `e32e7bf`.
3. ~~**Choose models for chat.**~~ Ticks on the catalog, saved in an `ai_models` table. **Done** — `3487a65`.
4. **The model picker moves into the chat.** Gemini plus the ticked Zen models, with `ChatModel` kept out of `Settings`.
5. **The drawer talks to either provider.** `AnyProvider`, plus an `OpenCode` that returns a canned reply.
6. **The `chat/completions` wire format.** Request body, chunk text and `[DONE]`, under `#[test]`.
7. **The real Zen stream.** POST with Bearer auth, checked end to end with a real key.
8. **Review and refactor.**

**Why this order.** Each step ends with something to see in the settings screen or the
drawer:

- **Steps 1–3** build the Zen block top to bottom: the key, then the list, then the ticks.
  The catalog parser lands in Step 2 next to its first caller, not ahead of it.
- **Step 4** is where `ChatModel` has to exist. The drawer's picker is its first real
  consumer, and before that the ticked ids are only strings in a table.
- **Step 5** brings in `AnyProvider` once the drawer can choose a Zen model and has to
  route it somewhere.
- **Step 6** breaks the rule on purpose, as Phase 22's Step 3 did: a parser belongs under
  tests before real bytes go through it.

> **Replanned 2026-09-27, before any code.** The first plan's Step 1 hard-coded two Zen models
> as `AiModel` variants, with a `Provider` mapping and a guard in `gemini_from`. The learner
> chose a live catalog and a picker in the chat instead, and inserted
> [Phase 24 — Settings screen](../06-settings-screen/phase-24-settings-screen.md) first to give
> it room. Two ideas from the old Step 1 survive: `Provider` as a closed enum (now Step 1),
> and "Gemini only serves Gemini models" (now enforced by `ChatModel.provider` in Step 4).
> Step details are written by `lbb:next` when this phase resumes.

## Step 1 — The Zen block and its key

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `73e2447` (209 tests green, 2 ignored; clippy clean; save, relaunch and forget confirmed by hand on a real iPhone). The iOS simulator build from `just install-ios` couldn't reach the keychain (`A required entitlement is not present`, Gemini included), so the check moved to the device.

**What it is.** Settings → AI now shows two blocks: *Gemini* (key row and model select, as
before) and *OpenCode Zen* (key row only). Each key goes to its own keychain entry, so
saving or forgetting one never touches the other. Nothing uses the Zen key yet.

### Runnable check first

`src/ai/mod.rs` — the provider names shown in the UI:

```rust
#[test]
fn each_provider_has_a_reader_facing_label() {
    assert_eq!(
        [Provider::Gemini, Provider::OpenCodeZen].map(Provider::label),
        ["Gemini", "OpenCode Zen"]
    );
}
```

`src/secrets/api_key.rs` — one keychain entry per provider:

```rust
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
```

Red against `todo!()` stubs: 3 failed, 208 passed. **Eyeball under `dx serve`:** gear →
*AI* shows *Gemini* then *OpenCode Zen*, with a gap between them. Save a Zen key and the
Zen status reads *Key set* while Gemini's doesn't change. Relaunch the app and the Zen status
is still *Key set*. *Forget* returns it to *Not set*.

### Minimal implementation

- **`src/ai/mod.rs`** — `enum Provider { Gemini, OpenCodeZen }`, `Copy`, with `label()`.
- **`src/secrets/api_key.rs`** (new submodule) — the keychain names `GEMINI_API_KEY` and
  `OPENCODE_ZEN_API_KEY` move here from `secrets/mod.rs` as private constants. A private
  `entry(provider)` maps a provider to its name. `get`, `is_set`, `save` (trims, stores,
  returns the stored key) and `forget` go through it.
- **`src/secrets/mod.rs`** — declares `pub(crate) mod api_key` and loses the constant. Its
  own tests use a neutral `"a-secret"` name, since the generic store shouldn't borrow a
  feature's entry.
- **`src/gemini_key/mod.rs`** — `save` and `forget` are gone, replaced by `api_key`.
  `gemini_from` reads through `api_key::get(store, Provider::Gemini)`, and its test sets
  the key with `api_key::save` rather than naming the entry.
- **`src/ui/settings.rs`** — `ApiKeyControl` takes `provider`, `key_set: bool` and
  `on_change: EventHandler<Option<String>>`. It does the storing itself and reports
  `Some(key)` after a save and `None` after a forget. `GeminiSettings` turns that into
  `Signal<Option<Gemini>>`. The new `OpenCodeZenSettings` keeps a local
  `use_signal(bool)`, read once from the store when it mounts.
- **`src/ui/settings_screen.rs`** — the *AI* arm renders both blocks.
- **`src/ui/settings.css`** — `.settings_group + .settings_group_title` adds space above a
  second heading.

### Why it works

- **The row stopped owning Gemini.** Before this step, `ApiKeyControl` read the Gemini
  signal and built a `Gemini`. Now it only stores and reports. Each caller decides what a
  saved key *means*: Gemini builds a client, Zen only flips a bool. That is how one
  component serves two providers without an `if provider == Gemini` inside it.
- **`EventHandler<Option<String>>` is a single channel for both outcomes.** `Some` means
  saved, `None` means forgotten, so there's one prop instead of two.
- **`Provider` is `Copy`**, so the `move` closures in `ApiKeyControl` each take their own
  copy with no `clone()`, the same way `SettingsSection` works in the sidebar.
- **`entry` and the names are private.** Only `secrets::api_key` knows the keychain names,
  so nothing can read a provider's key without going through `Provider`. A third provider
  means one constant and one match arm, and the compiler flags every `match` missing it.
- **Why a submodule of `secrets`.** The key helpers only talk to the secret store, so they
  sit with it. The cost is that `secrets` now depends on `ai::Provider`. Keeping that
  dependency in `api_key.rs` leaves `secrets/mod.rs` a generic store. Milestone 5's OAuth
  token can add a sibling submodule the same way.
- **`use_signal(|| …)` runs its closure once, on mount.** The keychain read doesn't repeat on
  every render.

### Scope note

- The Zen key has no consumer yet, so its state is local to the settings block. Step 4 or 5
  moves it into context, when the drawer needs it.
- No catalog, no model list, no ticks. That's Steps 2–3.

### Review notes (from the `simplify` pass)

- **Applied:** `gemini_from` read `GEMINI_API_KEY` directly, which put the Gemini↔entry
  mapping in two places. It now goes through `api_key::get`.
- **Skipped:** Gemini and Zen hold "key set" in two shapes: a `Signal<Option<Gemini>>` in
  context, and a local bool. That's deliberate. Gemini's shape exists because the drawer
  needs a client, and Zen has no client until Step 5.
- **Skipped:** `format!` for the `or_log` message allocates once per click. That's
  negligible, and a lazy `or_log_with` would mean editing `ui/mod.rs`, which is outside
  this step.

## Step 2 — The Zen catalog

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `e32e7bf` (210 tests green, 3 ignored; the ignored network test passes against the live endpoint; clippy clean; the list confirmed by eye).

**What it is.** Once a Zen key is saved, the *OpenCode Zen* block lists every model id
Zen offers, fetched live from `GET https://opencode.ai/zen/v1/models`. It shows *Loading…*
while the request is in flight and a red message if it fails. There are no ticks yet.

**The crux.** Until now, every network call went through a `spawn` started by a click in
the drawer. This is the first request whose job is to fill a view. `use_resource` owns that
job: it starts the future when the component mounts, gives you `Option<T>` (`None` while
loading), and re-renders the component when the value lands. Because the resource lives in
the component, mounting it is what fetches. So the list sits behind `if key_set()`, and
nothing hits the network until a key exists.

### Runnable check first

`src/ai/opencode.rs` — the parser, against a trimmed copy of a real response:

```rust
#[test]
fn a_captured_model_list_yields_its_ids_in_order() {
    let payload = r#"{
        "object": "list",
        "data": [
            { "id": "deepseek-v4-flash", "object": "model", "created": 1790582083, "owned_by": "opencode" },
            { "id": "glm-5.3-flash", "object": "model", "created": 1790582083, "owned_by": "opencode" },
            { "id": "kimi-k2.6", "object": "model", "created": 1790582083, "owned_by": "opencode" }
        ]
    }"#;

    assert_eq!(
        model_ids(serde_json::from_str(payload).unwrap()),
        ["deepseek-v4-flash", "glm-5.3-flash", "kimi-k2.6"]
    );
}
```

Also a `#[tokio::test] #[ignore = "needs the network"]` test that fetches the real list and
expects `deepseek-v4-flash` in it. Run it with `cargo test opencode -- --ignored`.

Red against a `todo!()` stub: the test panicked at `model_ids`. **Eyeball under `dx serve`:**
with a Zen key saved, Settings → AI shows *Models: Loading…* for a moment, then one row per
id under the key row. Forget the key and the list goes away.

### Minimal implementation

- **`src/ai/opencode.rs`** (new) — `ModelList { data: Vec<ModelEntry> }` and
  `ModelEntry { id }` deserialize only the fields we read, since serde ignores unknown
  keys. `model_ids` maps entries to ids. `models()` is `reqwest::get(MODELS_URL)`. A
  non-2xx status becomes `ChatError::Api { status, body }`, and a 2xx body goes through
  `response.json()` into `model_ids`.
- **`src/ui/settings.rs`** — `ZenCatalog` holds `use_resource(opencode::models)` and
  matches on it: `None` → a *Models / Loading…* row, `Some(Err)` → a
  `.catalog_error` paragraph, `Some(Ok(ids))` → one `settings_row` per id, keyed by id.
  `OpenCodeZenSettings` renders it only when `key_set()`.
- **`src/ui/settings.css`** — `.catalog_error` in `--primary-error-color`, the colour
  *Forget* already uses.

### Why it works

- **`use_resource(opencode::models)` takes the function by name.** `models` is an
  `async fn` with no arguments, so it already has the closure-returning-a-future shape
  `use_resource` wants. The future reads no signals, so it runs once per mount.
- **`match` goes *inside* `rsx!`.** The first draft returned `match &*catalog.read() { … }`
  as the function's tail expression. That didn't compile: `E0597: catalog does not live
  long enough`. The read guard is a temporary in the tail expression, and Rust drops those
  *after* the function's locals, so the guard would outlive `catalog`. Inside `rsx!`, the
  match is evaluated and its guard dropped before the function returns.
  `settings_screen.rs` already does it this way.
- **The error type is `ChatError`.** It already has `Http`, `Api` and `Json`, which is
  every way this call can fail. The name says "chat", but a new `CatalogError` with the
  same three variants would be duplication.
- **No key on the request.** Zen's `/models` endpoint is public. The key only decides
  whether the list is worth showing.

### Scope note

- No ticks and no saving. Step 3 turns each row into a checkbox backed by an `ai_models`
  table.
- The list refetches every time the block mounts: switching sections, or saving the key
  again. It's one small GET, and caching it can wait until it's a problem.
- There's no retry button on the error. Leaving the section and coming back retries.

### Review notes (from the `simplify` pass)

- **Skipped:** hoisting `use_resource` into `OpenCodeZenSettings` so a key toggle
  doesn't refetch. Toggles are rare, and switching sections unmounts the whole block
  anyway.
- **Skipped, for later:** `gemini.rs` has a private `api_error(status, body)`, and
  `models()` builds the same `ChatError::Api` inline. Moving it into `ai/mod.rs` touches
  `gemini.rs`, which is outside this step. It's worth doing in Step 7, when the Zen stream
  becomes the third caller.

## Step 3 — Choose models for chat

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `3487a65` (213 tests green, 3 ignored; clippy clean; ticking, the themed checkbox in Day/Sepia/Night, and ticks surviving a relaunch confirmed by eye).

**What it is.** Each row of the Zen catalog is now a checkbox. Ticking a model saves it to
a new `ai_models` table, and unticking removes it. After a relaunch, the ticks are still
there. Nothing reads them yet: Step 4's chat picker is their first consumer.

**The crux.** This is the first user-owned data that isn't in `Settings`. The phase's crux
explains why: a model id is a `String`, and `Settings` is `Copy`. So the ticks get their own
table, keyed by `(provider, model_id)`, instead of a column. A set of rows is also the
natural shape for "which of these n things did you pick". A column would have to pack a
list into text.

### Runnable check first

`src/db/ai_models.rs` — ticks round-trip, ticking twice is harmless, unticking removes one:

```rust
#[test]
fn ticked_models_round_trip_and_untick_removes_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(dir.path()).expect("open");

    assert!(db.ticked_models(Provider::OpenCodeZen).expect("empty").is_empty());

    db.set_ticked(Provider::OpenCodeZen, "kimi-k2.6", true).expect("tick kimi");
    db.set_ticked(Provider::OpenCodeZen, "deepseek-v4-flash", true).expect("tick deepseek");
    db.set_ticked(Provider::OpenCodeZen, "deepseek-v4-flash", true).expect("tick deepseek again");
    assert_eq!(
        db.ticked_models(Provider::OpenCodeZen).expect("read ticks"),
        ["deepseek-v4-flash", "kimi-k2.6"]
    );

    db.set_ticked(Provider::OpenCodeZen, "kimi-k2.6", false).expect("untick kimi");
    assert_eq!(
        db.ticked_models(Provider::OpenCodeZen).expect("read after untick"),
        ["deepseek-v4-flash"]
    );
}

#[test]
fn each_provider_keeps_its_own_ticks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(dir.path()).expect("open");

    db.set_ticked(Provider::OpenCodeZen, "shared-id", true).expect("tick for Zen");

    assert!(db.ticked_models(Provider::Gemini).expect("read Gemini").is_empty());
}
```

Plus `each_provider_has_a_stable_storage_slug` in `src/ai/mod.rs`, which pins
`"gemini"` and `"opencode-zen"`. Those strings are now stored data, and renaming one would
orphan every tick saved under it.

Red against `todo!()` stubs: both `ai_models` tests panicked with `not yet implemented`.
**Eyeball under `dx serve`:** with a Zen key saved, every catalog row shows a checkbox.
Clicking the model name toggles it too, because the row is a `<label>`. Tick
`deepseek-v4-flash` and `kimi-k2.6`, relaunch, and both are still ticked.

### Minimal implementation

- **`src/ai/mod.rs`** — `Provider::slug()`: `"gemini"`, `"opencode-zen"`.
- **`src/db/mod.rs`** — `CREATE TABLE IF NOT EXISTS ai_models (provider, model_id,
  PRIMARY KEY (provider, model_id))` in `migrate`, plus `mod ai_models`.
- **`src/db/ai_models.rs`** (new) — `ticked_models(provider)` selects ids ordered by
  `model_id`. `set_ticked(provider, id, ticked)` runs `INSERT OR IGNORE` or `DELETE`.
- **`src/main.rs`** — `use_context_provider(|| db.clone())`.
- **`src/ui/settings.rs`** — `ZenCatalog` reads the ticks once, on mount, into a
  `Signal<HashSet<String>>`. Each row is a `label` with the id and a checkbox. `onchange`
  saves first and updates the set only if the save succeeded.
- **`src/ui/settings.css`** — `.checkbox` restyles the native box to match the app. With
  `appearance: none` it becomes a rounded square with a `--tint-border` outline. When
  checked, it fills with the text colour and shows a check cut out in
  `--USER__backgroundColor`, so it follows Day, Sepia and Night like the rest of the
  settings. It shares the pill buttons' focus ring and the steppers' press-scale.

### Why it works

- **The composite primary key does the set semantics.** `(provider, model_id)` can't
  repeat, so `INSERT OR IGNORE` makes a second tick a no-op instead of an error or a
  duplicate row. `DELETE` of a missing row is also a no-op. So `set_ticked` is idempotent
  in both directions, and the UI never has to ask "is it already there?"
- **`query_map(..)?.collect()` into `Result<Vec<_>, _>`.** Each row is a
  `Result<String, rusqlite::Error>`. `collect` into a `Result<Vec<_>, _>` stops at the
  first error. It's the same trick as `?`, applied to a whole iterator.
- **`for id in ids.iter().cloned()`.** The ids are borrowed from the resource's read guard.
  The `onchange` closure is `move` and outlives this render, so it needs its own `String`.
  `cloned()` gives each iteration an owned id, and the closure takes it.
- **`let db = db.clone()` inside the attribute block.** Each row's closure moves in its own
  `Rc<Db>`. Cloning an `Rc` bumps a counter; it doesn't copy the database.
- **Save, then update the signal.** If the write fails, the signal isn't touched, so the
  app's idea of what is ticked stays equal to what is stored. The box on screen doesn't
  follow, though. The browser flipped it on the click, and since no signal changed,
  Dioxus neither re-renders nor patches `checked`. It shows the unsaved tick until the
  section remounts. A failing SQLite write is rare enough to leave for now.
- **`HashSet`, not `Vec`, in the UI.** The component only asks whether an id is ticked, so
  a set fits: `insert`/`remove` instead of `push`/`retain`. The DB still returns a sorted
  `Vec`, because Step 4's picker will want a stable order.

### Scope note

- The ticks live in a signal local to `ZenCatalog`. Step 4 lifts them to an app-level
  signal, because the chat drawer reads them too.
- A ticked model that disappears from Zen's catalog stays in the table and simply has no
  row to show. Pruning it can wait until it bites.
- Gemini's models get no ticks. They stay fixed, per the phase's decisions.

### Review notes (from the `simplify` pass)

- **Applied:** `ticked` went from `Vec<String>` to `HashSet<String>`. It's only used for
  membership checks.
- **Skipped, for Step 4:** putting all of `Rc<Db>` in context gives every component access
  to every table. Until now, only `Library` and the startup code in `main.rs` touched `Db`.
  The better fix is the `Settings` pattern: load the ticks at the `App` root into a signal
  and share that. The drawer is the ticks' second consumer, so the fix belongs to Step 4.
- **Skipped:** implementing `Choice` for `Provider` to get `slug()`. `Choice` requires
  `Default`, and `from_slug` falls back to it silently. That suits a picker setting, but
  `Provider` has no natural default.
- **Skipped:** reusing `SettingRow` for the catalog rows. It takes a `&'static str`
  label and renders a `div`. A catalog row needs a runtime `String` and a `<label>`, so the
  checkbox toggles on a click on the name. Widening it touches every caller.
