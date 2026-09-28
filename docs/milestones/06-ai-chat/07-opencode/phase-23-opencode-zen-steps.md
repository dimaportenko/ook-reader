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
2. **The Zen catalog.** Fetch `/zen/v1/models`, show the ids, and parse under `#[test]`.
3. **Choose models for chat.** Ticks on the catalog, saved in an `ai_models` table.
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
