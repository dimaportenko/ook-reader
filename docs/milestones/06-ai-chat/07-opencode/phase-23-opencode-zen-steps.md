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
4. **The model picker moves into the chat.** Split on 2026-09-28, since one diff was too big to review:
   - ~~**4a.**~~ The drawer's picker lists Gemini plus the ticked Zen models, as `ChatModel`s held outside `Settings`. **Done** — `4661b6a`.
   - ~~**4b.**~~ A `chat_model` table remembers any pick, and `ai_model` leaves `Settings`. **Done** — `3613315`.
5. ~~**The drawer talks to either provider.**~~ `AnyProvider`, plus an `OpenCode` that returns a canned reply. **Done** — `5474e3b`.
6. ~~**The `chat/completions` wire format.**~~ Request body, chunk text and `[DONE]`, under `#[test]`. **Done** — `adf6670`.
7. ~~**The real Zen stream.**~~ POST with Bearer auth, checked end to end with a real key. **Done** — `36f95b4`.
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

## Step 4a — The model picker moves into the chat

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `4661b6a` (217 tests green, 3 ignored; clippy clean; grouped pill picker, Gemini pick surviving a relaunch, the Zen note, live tick updates and the removed Settings row confirmed by eye).

**What it is.** The chat drawer now has a model select under its header. The options are
grouped: *Gemini* (Flash-Lite, Flash), then *OpenCode Zen* (the models ticked in Settings).
Picking a Gemini model chats with it. Picking a Zen model replaces the conversation with
*OpenCode Zen replies aren't available yet.* Settings → AI → Gemini loses its *Model* row,
because the choice now lives in the drawer.

**The crux.** This is the phase's crux #1 made real. The chosen model is now
`ChatModel { provider, id: String }`: a *value* with a runtime id, not a variant of a
closed enum. `String` isn't `Copy`, so `ChatModel` can't go into `Settings`, which is.
Adding it would break every `settings()` read in the app. So the chosen model gets its own
app-level `Signal<ChatModel>`, provided next to `Signal<Settings>`, not inside it.

**Why split.** The whole of Step 4 also meant persisting a Zen pick and removing
`Settings.ai_model`, which touches the migration, the settings round-trip tests and the
`Choice` tests. 4a is the picker and the type. 4b is where the choice is stored.

### Runnable check first

`src/ai/mod.rs`:

```rust
#[test]
fn the_chat_offers_every_gemini_model_then_the_ticked_zen_ones() {
    let ticked = BTreeSet::from(["kimi-k2.6".to_owned(), "deepseek-v4-flash".to_owned()]);
    let zen = |id: &str| ChatModel {
        provider: Provider::OpenCodeZen,
        id: id.to_owned(),
    };

    assert_eq!(
        chat_models(&ticked),
        [
            ChatModel::gemini(AiModel::FlashLite),
            ChatModel::gemini(AiModel::Flash),
            zen("deepseek-v4-flash"),
            zen("kimi-k2.6"),
        ]
    );
}

#[test]
fn a_gemini_chat_model_uses_the_api_name() { /* ChatModel::gemini(Flash) == { Gemini, "gemini-3.5-flash" } */ }

#[test]
fn a_chat_model_key_names_its_provider_and_id() { /* "opencode-zen/kimi-k2.6" */ }

#[test]
fn only_a_gemini_chat_model_maps_back_to_a_gemini_setting() {
    /* Gemini Flash → Some(AiModel::Flash); a Zen model with a Gemini-looking id → None */
}
```

Red against `todo!()` stubs: the first three failed with `not yet implemented`. The fourth
came from the `simplify` pass, after the code it tests. To prove it can fail, its expected
value was inverted: red, then restored. **Eyeball under `dx serve`:**

1. Open the chat. A pill-styled select shows `gemini-3.5-flash-lite` (or whatever
   Settings held).
2. It lists a *Gemini* group, plus an *OpenCode Zen* group only if you ticked something.
3. Pick `gemini-3.5-flash`, relaunch, and it's still picked.
4. Pick a Zen model and the note appears. Pick Gemini again and the compose box returns.
5. Tick or untick a model in Settings, and the drawer's list follows without a relaunch.
6. Settings → AI → Gemini has no *Model* row.

### Minimal implementation

- **`src/ai/mod.rs`** — `ChatModel { provider, id }`, `Clone` but not `Copy`.
  `ChatModel::gemini(AiModel)` builds one from the fixed API name. `gemini_model()` maps
  back and gives `None` for anything not served by Gemini. `key()` is
  `"{provider-slug}/{id}"`, the `<option>` value. `chat_models(&BTreeSet<String>)` chains
  the Gemini models onto the ticked Zen ids.
- **`src/main.rs`** — two app-level signals in context:
  - `zen_ticked: Signal<BTreeSet<String>>`, read from `ai_models` once at startup.
  - `chat_model: Signal<ChatModel>`, seeded from `settings.peek().ai_model`.
- **`src/ui/settings.rs`** — `ZenCatalog` uses the `zen_ticked` context signal instead of
  its own. The Gemini *Model* `ChoiceRow` is gone. `Styles` becomes `pub(crate)`, so the
  drawer can reuse `pill_button` and `choice`.
- **`src/ui/chat.rs`** — `ModelPicker` renders the select with one `optgroup` per
  provider that has models. `onchange` finds the model by key and, if it's a Gemini one,
  writes it to `settings.ai_model`. Then it sets `chat_model`. `ChatConversation` shows the
  Zen note when a Zen model is chosen, and `submit` returns early for a non-Gemini model.
- **`src/ui/chat.css`** — `.chat_panel__model` only positions the select, under the header.

### Why it works

- **`ChatModel` stays out of `Settings`.** `Settings` derives `Copy`, which needs every
  field to be `Copy`, and `String` owns a heap buffer, so it can't be. A field of type
  `ChatModel` would force `Settings` down to `Clone`, and every `settings()` read (a copy
  today) would turn into a clone or an error. A separate signal is also more honest:
  settings describe how the book looks, and the chat model is what the drawer talks to.
- **`BTreeSet`, not `HashSet`.** It's a set, so ticking is `insert`/`remove`, but it iterates
  in sorted order, and that's the picker's order. It's also the same order as the DB's
  `ORDER BY model_id`, so the app-level signal and the table agree without sorting.
- **The ticks moved up to `App`.** Settings writes them and the drawer reads them, so they
  need an owner above both. `ZenCatalog` still writes the table through `Db` and then
  updates the shared signal. That write is what re-renders the drawer's picker.
- **`settings.peek()` in `App`.** Reading `settings()` in `App`'s body would subscribe the
  whole app to every font-size tap. `peek` reads without subscribing. The seed only
  matters once.
- **Gemini picks still persist, through `Settings.ai_model`.** The Gemini client is still
  built from `settings().ai_model` by the effect in `App`. Writing it on a Gemini pick
  rebuilds the client *and* remembers the pick across a relaunch, with no new storage.
  That's the bridge 4b removes.
- **`onchange` rebuilds the list instead of capturing it.** The closure is `move` and
  outlives this render, and `models` is still borrowed by the `for` loops below it. Calling
  `chat_models` again inside the handler reads the ticks as they are *at click time*.
- **`selected: *model == *chosen.read()`** compares the values directly. `ChatModel`
  derives `PartialEq`, so no key string gets built.
- **Borrowing another module's CSS.** A `#[css_module]` struct injects its stylesheet the
  first time any class name is displayed, via a `OnceLock` in the generated `Deref`. So
  the drawer can use `SettingsStyles::pill_button` even if the settings screen was never
  opened.

### Scope note

- A Zen pick is forgotten on relaunch. The app comes back on the last *Gemini* model.
  That's 4b.
- `Settings.ai_model` still exists, and `chat_model` mirrors it for Gemini picks: two
  signals for one fact, bridged in `onchange`. 4b collapses them.
- A Zen model unticked in Settings while it's chosen stays chosen, but it's missing from
  the list, so the select shows the first option. 4b's persistence is the place to decide
  the fallback.
- Zen still can't answer. Step 5 adds `AnyProvider` and a canned `OpenCode` reply.

### Review notes (from the `simplify` pass)

- **Applied:** `gemini_model()` on `ChatModel`. The reverse mapping used to live in the
  picker's `onchange`, as a scan that built a `ChatModel` for each `AiModel`. Now it sits
  next to `ChatModel::gemini`, compares `api_name()` strings without allocating, and has a
  test.
- **Applied:** `selected` compares `ChatModel`s instead of formatting a key per option.
- **Applied:** the picker reuses the settings pill style (`pill_button` + `choice`)
  instead of rendering a bare native select.
- **Skipped, for 4b:** deriving `chat_model` from `settings.ai_model` plus a "chosen Zen id",
  to avoid the double write. 4b replaces `settings.ai_model` with a table, and then
  `chat_model` is the only source, so a memo now would be rewritten next step.
- **Skipped:** reusing `ChoiceRow`. It needs a `Choice` (a `Copy` enum with a fixed
  `all()`), and it has no `optgroup`s.
- **Skipped:** cloning `models` into `onchange` instead of calling `chat_models` again.
  Either works. The second call reads the ticks at click time, which is the more
  accurate choice.

### Found at commit (`lbb:commit` review)

- **A stale pick blocks the first option.** Untick the chosen Zen model and the select
  shows the first option, but `chat_model` still holds the Zen one. Picking that first
  option then fires no `onchange`, because the value didn't change, so you have to pick
  another model first. For 4b: when the chosen model leaves the list, reset `chat_model` to
  the first entry.
- **The toolbar's send is dropped while a Zen model is chosen.** `submit` returns early.
  The passage stays in the draft, hidden behind the note. Step 5 makes it moot.

## Step 4b — The chosen model persists

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `3613315` (219 tests green, 3 ignored; clippy clean; Zen and Gemini picks surviving a relaunch, the fallback after unticking, and re-saving the Gemini key confirmed by eye). The legacy-column test was watched fail by inverting its expected value.

**What it is.** The drawer's pick survives a relaunch, whether it's a Gemini or a Zen
model. It lives in its own one-row `chat_model` table, and `Settings.ai_model` is gone, so
`Signal<ChatModel>` is the only place the choice is held. Untick the chosen Zen model in
Settings and the pick falls back to the first model on offer.

**The crux.** 4a left two signals for one fact: `settings.ai_model` for Gemini picks and
`chat_model` for everything, bridged by a write in `onchange`. The fix isn't a better
bridge. It's deleting one side. Once the pick is stored as `(provider, model_id)` strings,
`ChatModel` round-trips on its own, and `AiModel` only matters at the edge where a Gemini
client needs an API name.

### Runnable check first

`src/db/chat_model.rs`:

```rust
#[test]
fn the_chosen_chat_model_round_trips_and_the_latest_pick_wins() {
    /* None at first; save a Zen kimi-k2.6 → reads back; save Gemini Flash → reads back */
}

#[test]
fn a_pick_from_an_unknown_provider_reads_as_no_pick() {
    /* corrupt provider to 'openai' → chat_model() is None, so the app seeds the default */
}
```

`src/ai/mod.rs`:

```rust
#[test]
fn a_provider_slug_reads_back_and_an_unknown_one_does_not() {
    /* every Provider::ALL slug parses back; "openai" → None */
}
```

`src/db/mod.rs` swaps the old "add the `ai_model` column" migration test for
`a_settings_table_that_still_has_the_ai_model_column_keeps_saving`: an existing database
keeps its `ai_model NOT NULL DEFAULT` column, and `save_settings` must still work without
naming it.

Red against `todo!()` stubs: the three new tests failed with `not yet implemented`. The
legacy-column test passes before and after. It's a regression guard for the column this
step stops writing, so it was never watched fail. **Eyeball under `dx serve`:**

1. Pick a Zen model in the drawer, relaunch, and it's still picked, with the Zen note.
2. Pick `gemini-3.5-flash`, relaunch, and it's still picked. Chat answers with it.
3. With a Zen model picked, untick it in Settings. The drawer switches to
   `gemini-3.5-flash-lite` and the compose box returns. Picking Flash-Lite is no longer
   needed to escape the stale pick.
4. Settings → AI → Gemini: forget and re-save the key while Flash is picked. Chat still
   uses Flash.

### Minimal implementation

- **`src/db/chat_model.rs`** (new) — `save_chat_model` upserts row `id = 1`, the same
  `ON CONFLICT(id) DO UPDATE` shape as `save_settings`. `chat_model()` reads the two
  strings and returns `None` when the provider slug is unknown.
- **`src/db/mod.rs`** — `CREATE TABLE IF NOT EXISTS chat_model`. The `ai_model` column
  leaves the `settings` `CREATE`, and the `ALTER TABLE … ADD COLUMN ai_model` migration is
  deleted.
- **`src/db/settings.rs`, `src/settings/mod.rs`** — `ai_model` leaves `Settings`, its
  `Default`, the upsert, the select and the round-trip tests.
- **`src/ai/mod.rs`** — `Provider::ALL` and `Provider::from_slug`. `impl Default for
  ChatModel` is Gemini Flash-Lite.
- **`src/main.rs`** — `chat_model` is seeded from `db.chat_model()`, falling back to
  `ChatModel::default()`. A save effect writes it back. A fallback effect resets it to the
  first offered model when the ticks no longer include it. The Gemini client's model memo
  reads `chat_model` instead of `settings`.
- **`src/ui/chat.rs`** — `onchange` only sets `chat_model`. The picker loops over
  `Provider::ALL`.
- **`src/ui/settings.rs`** — saving a Gemini key builds the client from the chosen model.
- **`src/settings/ai_model.rs`, `src/settings/choice.rs`** — the `Choice` impl on
  `AiModel` is deleted, with its label test and slug round-trip line. Nothing reads its
  slug or label once it's out of `Settings`.

### Why it works

- **Strings in, strings out.** `ChatModel` is `{ provider, id: String }`, so the table is
  two `TEXT` columns and the read is `(String, String) → ChatModel`. Only the provider
  needs parsing. Returning `Option<Provider>` from `from_slug`, not a default, lets
  `chat_model()` say "no usable pick" and leave the choice of default to `App`.
- **The old column stays in old databases.** It's `NOT NULL DEFAULT 'flash-lite'`, so an
  `INSERT` that doesn't name it still succeeds. Dropping it would need `ALTER TABLE … DROP
  COLUMN` for no user-visible gain, so the regression test pins the cheaper choice.
- **`peek` in the fallback effect.** The effect reads `zen_ticked` (subscribing) and
  `chat_model.peek()` (not subscribing). It re-runs when the ticks change, not when you
  pick, so a pick can't loop into a reset. On first run it also repairs a stale pick
  loaded from the table.
- **The fallback lives in `App`, not the picker.** The picker is only mounted while the
  drawer exists. Ticks change on the settings screen, so the invariant "the chosen model
  is on offer" has to be kept where both signals are owned.
- **The save effect subscribes by `read()`.** Every `set` on `chat_model`, from a pick or
  a fallback, writes the row. It also writes once at startup, the same trade the settings
  save makes.
- **`ai_model` is still a memo.** Switching between two Zen models maps to the same
  `AiModel::default()`, so the memo's value doesn't change and the Gemini client isn't
  rebuilt.

### Scope note

- A Gemini pick saved before this step isn't carried over. The first launch comes back on
  Flash-Lite, once. Seeding `chat_model` from the old column is a few lines, but it adds a
  migration branch for one pick on one machine.
- `chat_model.read().gemini_model().unwrap_or_default()` appears twice: the memo in `App`
  and the key handler in `GeminiSettings`. Step 5's `AnyProvider` gives the client one
  owner and removes both.
- With a Zen model chosen, the Gemini client quietly holds Flash-Lite. It's never used for
  chat then, and Step 5 routes by provider.
- `AiModel` still lives under `settings/` though it's no longer a setting. Moving it to
  `ai/` is a Step 8 candidate.

### Review notes (from the `simplify` pass)

- **Applied:** `Provider::ALL`, matching `AiModel::ALL` and `Theme::ALL`. `from_slug`, the
  picker and the new test read it, so a third provider is one edit.
- **Applied:** `impl Default for ChatModel`, so `App` seeds with `unwrap_or_default()` and
  doesn't import `AiModel`.
- **Applied:** the fallback effect uses `let Some(first) = offered.first() else { return }`
  instead of a nested `if let`.
- **Applied:** the dead `Choice` impl on `AiModel` is deleted.
- **Applied:** the key handler binds `model` before building the client, instead of one
  long nested line.
- **Skipped:** a `ChatModel::gemini_or_default()` helper for the duplicated expression.
  Step 5 deletes both call sites.
- **Skipped:** leaving the key handler to only save, and letting `App`'s effect rebuild
  the client. That effect doesn't watch the keychain, so a newly saved key wouldn't show
  up until the next pick.
- **Skipped:** `use_signal` instead of `use_hook(|| Signal::new(…))` for the seed. The
  latter is how `settings` and `zen_ticked` are seeded two lines above.
- **Skipped:** dropping the legacy-column test as too much setup. It's the only thing
  guarding saves on an existing database.

## Step 5 — The drawer talks to either provider

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `5474e3b` (219 tests green, 3 ignored; clippy clean; the canned Zen reply, the per-provider key note, Gemini replies and the toolbar send with a Zen pick confirmed by eye).

**What it is.** Pick a Zen model in the drawer, send, and a reply streams back. For now it's
a canned sentence naming the model, from an `OpenCode` stand-in. Gemini picks answer as
before. With no key for the chosen model's provider, the drawer says *Add your Gemini key* or
*Add your OpenCode Zen key*. The "Zen replies aren't available yet" note and the
Gemini-only guard in `submit` are gone.

**The crux.** `ChatProvider` has an `async fn`, so it isn't object-safe and
`Box<dyn ChatProvider>` doesn't compile. The runtime choice between two clients becomes an
enum, `AnyProvider { Gemini(Gemini), OpenCode(OpenCode) }`, and it implements the trait by
matching and delegating. The set of providers is closed, so a closed enum costs nothing a
trait object would have bought.

### Runnable check first

`src/ai/mod.rs`:

```rust
#[test]
fn any_provider_streams_through_the_client_it_holds() {
    /* AnyProvider::OpenCode(OpenCode::new("kimi-k2.6")) streams >1 piece,
       the pieces concat to reply.text, and the text names kimi-k2.6 */
}
```

`src/ai_client/mod.rs` (was `src/gemini_key/mod.rs`):

```rust
#[test]
fn the_chosen_model_needs_its_own_provider_s_key() {
    /* no keys → None for both; Zen key → Zen gets AnyProvider::OpenCode, Gemini still None;
       Gemini key → AnyProvider::Gemini; forget Zen → Zen None again */
}
```

Red against `todo!()` stubs: both failed with `not yet implemented` (218 passed, 2 failed).
**Eyeball under `dx serve`:**

1. With a Zen key saved, pick a Zen model in the drawer and send. The canned reply streams
   in and names the model.
2. Forget the Zen key, reopen the book. The drawer says *Add your OpenCode Zen key in
   settings to chat.*
3. Pick a Gemini model. Chat answers from Gemini with that model, as before.
4. Select a passage and tap the toolbar chat icon while a Zen model is picked. It sends
   and gets the canned reply, where 4a dropped it.

### Minimal implementation

- **`src/ai/opencode.rs`** — `OpenCode { model }` with `new`, and a `ChatProvider` impl
  that splits a canned sentence into words and feeds them to `on_text`.
- **`src/ai/mod.rs`** — `AnyProvider` and its delegating `ChatProvider` impl.
  `ChatModel::gemini_model()` and its test are deleted, since both callers are gone.
- **`src/ai_client/mod.rs`** — `gemini_key` renamed. `provider_for(store, &ChatModel)`
  reads the chosen provider's key and builds the matching client with `model.id`.
- **`src/main.rs`** — the `Signal<Option<Gemini>>` context, the `ai_model` memo and the
  effect that rebuilt the client are gone.
- **`src/ui/chat.rs`** — `ChatConversation` owns a `Signal<Option<AnyProvider>>`, filled by
  an effect that watches `chosen`. `submit` sends through whichever client it holds.
- **`src/ui/settings.rs`** — `use_key_set(provider)` reads "is a key saved" once. The Gemini
  and Zen blocks both use it, so saving a key only flips that flag.

### Why it works

- **The enum is the dispatch.** `match self` picks the arm, and each arm awaits the inner
  client's own `stream`. `on_text` moves into whichever arm runs, and only one does.
- **The client is built from the whole `ChatModel`.** Gemini gets `model.id` directly, so
  the `AiModel` round trip that 4b left in two places isn't needed anymore.
- **The drawer owns its client.** The settings screen opens from the library, so leaving
  it always remounts the reader and re-runs the effect with the fresh key. Settings no
  longer has to push a client into App, and App no longer holds one it can't name.
- **An effect and a signal, not `use_memo`.** `use_memo` needs `PartialEq` on its value, and
  `Gemini` holds a `reqwest::Client`, which has none.

### Scope note

- `OpenCode` holds no key yet, so `provider_for` checks the Zen key exists but drops it. Step
  7 adds the key with the Bearer header.
- Ticking a Claude or GPT Zen model still "works" here, because the reply is canned. The
  endpoint mismatch shows up in Step 7.
- Each pick builds a fresh `reqwest::Client`, as the App effect did before. Sharing one is
  a Step 8 candidate.

### Review notes (from the `simplify` pass)

- **Skipped:** `use_memo` for the drawer's provider. `AnyProvider` can't be `PartialEq`.
- **Skipped:** merging `GeminiSettings` and `OpenCodeZenSettings`. Only the Zen block has
  the catalog.
- **Skipped:** moving `provider_for` into `ai` as `AnyProvider::from_secrets`. That would
  make `ai` depend on `secrets`, and the bridge module already exists.
- **Skipped:** a shared keys signal between settings and the drawer. The drawer can't be
  mounted while settings is open, so the two reads can't disagree today.
- **Skipped:** a lazy `or_log_with` for the formatted log message. It runs on mount and on
  pick, not per delta.

## Step 6 — The `chat/completions` wire format

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `adf6670` (227 tests green, 3 ignored; clippy: only the expected `dead_code` warnings for the new items, until Step 7 calls them).

**What it is.** The pure half of talking to Zen, in `src/ai/opencode.rs`. `request_body`
turns the conversation into an OpenAI-style `chat/completions` body. `event` turns one SSE
payload into either the text it carries or the end of the stream. Nothing calls them yet;
Step 7 does. Like Phase 22's Step 3, this step breaks observability order on purpose: the
only check is `#[test]`.

**The crux.** Two differences from Gemini, and both are about where the end is.

- **The stream ends with a sentinel, not a closed connection.** The last event is
  `data: [DONE]`. That isn't JSON, so handing it to `serde_json` would fail on every
  successful reply. `event` checks for it *before* parsing and returns `Event::Done`. The
  check stays here, not in `SseBuffer`: `[DONE]` is an OpenAI convention, not part of SSE,
  and Gemini has no such marker.
- **Many chunks carry no text.** The first chunk names the role (`"content": ""`), the last
  has an empty `delta` and a `finish_reason`, a usage chunk has `"choices": []`, and some
  servers send `"content": null`. `#[serde(default)]` and `Option` turn every one of those
  into `Text("")` instead of an error, as Gemini's `text_of` does.

### Runnable check first

`cargo test ai::opencode`, in `src/ai/opencode.rs`:

```rust
#[test]
fn the_conversation_becomes_chat_completions_messages() {
    /* request_body("kimi-k2.6", [user, assistant]) serializes to
       { model, messages: [{role: "user", content}, {role: "assistant", content}], stream: true } */
}

#[test]
fn a_captured_chunk_yields_its_delta_text() { /* → Event::Text("Ankh-Morpork") */ }
#[test]
fn the_opening_chunk_names_the_role_and_yields_nothing() { /* → Text("") */ }
#[test]
fn a_null_content_yields_nothing() { /* → Text("") */ }
#[test]
fn the_closing_chunk_has_an_empty_delta_and_yields_nothing() { /* → Text("") */ }
#[test]
fn a_usage_chunk_has_no_choices_and_yields_nothing() { /* → Text("") */ }
#[test]
fn the_done_marker_ends_the_stream() { /* event("[DONE]") → Event::Done */ }
#[test]
fn a_payload_that_is_not_json_is_an_error() { /* event("[DONE") → Err */ }
```

Red against `todo!()` stubs: all 8 failed with `not yet implemented` (1 passed, 8 failed, 1
ignored in the module).

### Minimal implementation

- **Request:** `CompletionRequest { model, messages, stream: true }` and
  `CompletionMessage { role, content }`, both borrowing from the caller. A
  `From<&Message>` impl maps `Role::Assistant` to `"assistant"`, where Gemini's says
  `"model"`.
- **Response:** `CompletionChunk { choices }` → `Choice { delta }` → `Delta { content:
  Option<String> }`, with `#[serde(default)]` on the parts that can be missing.
- **`enum Event { Text(String), Done }`** and `event(payload)`. It returns `Done` for
  `[DONE]`. Otherwise it parses the payload and takes the first choice's `delta.content`,
  or `""`.

### Why it works

- **Borrowing request structs.** `CompletionRequest<'a>` holds `&'a str`s into the model id
  and the messages, so building the body copies no text. The lifetime ties the body to the
  slice it was built from. It only lives until `.json(&body)` serializes it.
- **`?` on `from_str`.** A payload that is neither `[DONE]` nor valid JSON is a real error.
  It flows out as `serde_json::Error`, and `ChatError: From<serde_json::Error>` lets Step 7's
  loop use `?` on it.
- **`.into_iter().next()`** takes the first choice by value, so the `String` moves out and
  isn't cloned. The request never asks for `n > 1`, so there is only ever one choice.

### Scope note

- No HTTP yet. The POST, the `Authorization: Bearer` header, and the loop that feeds
  `SseBuffer` output to `event` and stops on `Done` are Step 7. `OpenCode` still returns the
  canned reply.
- Until Step 7 calls them, `cargo clippy` reports `dead_code` warnings for the new items.
  They are left visible instead of silenced with `#[allow(dead_code)]`, since the next step
  clears them.
- The chunk shapes follow the OpenAI streaming format, not a capture from Zen. Step 7's
  real stream is where a Zen-specific field (for example DeepSeek's `reasoning_content`)
  would show up.
- Error events inside the stream (`data: {"error": …}`) parse as a chunk with no choices,
  so they yield `""`. If Step 7 sees them in practice, they become an `Event` variant.

### Review notes (from the `simplify` pass)

All four angles came back clean, so nothing was changed.

- **Skipped:** an `Option<String>` or `Skip` variant for empty chunks. Gemini's `text_of`
  also yields `""`, and the two providers stay consistent.
- **Skipped:** folding the four "yields nothing" tests into one loop. One test per shape
  matches `gemini.rs` and names each case.
- **Noted for Step 7:** reuse `non_empty_reply` and `api_error` from `gemini.rs`. Lift them
  into `ai/mod.rs` rather than copying them.

## Step 7 — The real Zen stream

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `36f95b4` (227 tests green, 4 ignored; clippy clean; a real Zen reply confirmed by hand in the drawer, and `gpt-6-luna` returned the expected `ModelProtocolUnsupported` 400).

**What it is.** Pick a Zen model in the drawer, send, and a real reply from Zen streams in.
`OpenCode` now holds the key and a `reqwest::Client`. It POSTs to
`https://opencode.ai/zen/v1/chat/completions` with `Authorization: Bearer <key>` and feeds
the response through the same `SseBuffer` Gemini uses. It stops on `[DONE]`. The canned
reply is gone.

**The crux.** Almost nothing is new. Step 6 did the parsing and Phase 22 did the buffering,
so this step is the loop that joins them, plus one control-flow detail. `[DONE]` shows up
*inside* the `for payload in sse.push(..)` loop, and the loop that has to stop is the *outer*
`while let Some(chunk)`. A labelled `break 'stream` leaves both at once. A plain `break`
would only leave the `for`, and the code would go back to waiting for chunks the server has
already stopped sending.

### Runnable check first

`src/ai/opencode.rs`. This test runs offline, because it builds the request without sending
it:

```rust
#[test]
fn a_request_posts_the_conversation_to_zen_with_a_bearer_key() {
    /* OpenCode::new("zen-key", "kimi-k2.6").request(&[user]).build():
       POST, url == COMPLETIONS_URL, authorization == "Bearer zen-key",
       body.model == "kimi-k2.6", body.stream == true, messages[0].content */
}

#[tokio::test]
#[ignore = "needs OPENCODE_API_KEY and the network"]
async fn a_real_zen_streams_through_the_trait() {
    /* deepseek-v4-flash, "Count from one to twenty in words": >1 piece, pieces concat to reply */
}
```

Red against a `todo!()` `request`: `not yet implemented` (0 passed, 1 failed). Signature
changes are also a compile-time red: `OpenCode::new` now takes the key, and
`ai_client::provider_for` passes it along.

**The real check is the learner's.** The agent has no Zen key, so neither of the following
was run while writing:

1. `OPENCODE_API_KEY=… cargo test a_real_zen -- --ignored`
2. `dx serve`: with a Zen key saved, tick `deepseek-v4-flash` (or `glm-5.3-flash`,
   `kimi-k2.6`), pick it in the drawer and send. A real answer streams in. *Stop* midway
   keeps what arrived.
3. Pick a ticked Claude or GPT Zen model and send. The drawer shows an API error, as the
   phase's design decisions predicted.

### Minimal implementation

- **`src/ai/mod.rs`:** `api_error` and `non_empty_reply` (and their tests) move here from
  `gemini.rs`. They stay private, because child modules can see their parent's private items.
  The canned-reply test `any_provider_streams_through_the_client_it_holds` is deleted,
  because `OpenCode` can no longer answer offline.
- **`src/ai/opencode.rs`:** `OpenCode { key, model, client }`, with `new(key, model)`.
  `request()` builds the POST: `.bearer_auth(&self.key).json(&request_body(..))`. `stream`
  sends it, turns a non-2xx status into `api_error`, then runs the `SseBuffer` → `event` loop
  with `break 'stream` on `Done`, and ends in `non_empty_reply`. `models()` uses `api_error`
  instead of building `ChatError::Api` inline (the item Step 2 left for this step).
- **`src/ai/gemini.rs`:** imports the two helpers from `super`.
- **`src/ai_client/mod.rs`:** `OpenCode::new(key, &model.id)`. The key that Step 5 read and
  dropped is now used.

### Why it works

- **`request()` returns an unsent `RequestBuilder`.** `stream` calls `.send()` on it, and the
  test calls `.build()` and inspects the result. That split is what lets the URL, the auth
  header and the body be checked without the network.
- **`bearer_auth`** writes `Authorization: Bearer <key>`, which is what Zen's
  OpenAI-compatible endpoint expects. Gemini's `x-goog-api-key` header is its own scheme.
- **`'stream:` labels the `while`.** `break 'stream` jumps past both loops to
  `non_empty_reply(text)`. When `response` is dropped, the connection is closed without
  reading anything after `[DONE]`.
- **`OpenCode` no longer derives `Debug`.** It holds the key now, and `Gemini` doesn't derive
  it either, so the key can't end up in a `{:?}` log line.

### Scope note

- There is no per-model format table. A Claude or GPT id ticked in the catalog gets an API
  error from `chat/completions`. The phase accepts this, and it is a later refinement if it
  bites.
- `AnyProvider`'s delegation no longer has an offline test. Its two arms each bind a
  different concrete type, so swapping them wouldn't compile. A `#[cfg(test)]` fake variant
  would put test scaffolding into the production enum.
- `reasoning_content`, and error events inside the stream, still yield `""`. If the real
  check shows either one, it becomes an `Event` variant.
- Each pick still builds a fresh `reqwest::Client`. That is a Step 8 candidate, now for both
  providers.

### Review notes (from the `simplify` pass)

All four angles came back clean, so nothing was changed.

- **Skipped:** a shared stream loop for Gemini and Zen. It would need a per-provider decode
  closure that returns `Text`/`Done`, and that would hide the loop behind indirection to save
  about ten lines.
- **Skipped:** a `#[cfg(test)]` `AnyProvider::Fake` to test delegation offline (see scope
  note).
- **Kept:** `cargo fmt` rewrapped two Step 6 test payloads that had been committed over the
  line width.
- **Agreed, deferred:** one shared `reqwest::Client` instead of one per pick (Step 8).
