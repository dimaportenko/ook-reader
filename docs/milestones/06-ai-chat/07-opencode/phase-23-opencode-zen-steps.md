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
