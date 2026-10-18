# Phase 25 — Third provider: OpenAI — build log

[← Phase 25](phase-25-openai.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

OpenAI's `chat/completions` is the wire format Zen already speaks, so the third provider is
a second **host** for the existing client, not a new client. Zen's `COMPLETIONS_URL` const
becomes a base URL the client is built with. `Provider` gains `OpenAi`, and Luna is a fixed
id, the way Gemini's models are. A per-host `reasoning_effort` comes last, after a
measurement, and never reaches Zen.

## Step plan

1. ~~**The OpenAI block and its key.**~~ `Provider::OpenAi` (slug `openai`), a third keychain
   entry, and an *OpenAI* block in Settings → AI with only a key row. Nothing uses the key
   yet. The `Provider::ALL` length and `from_slug` round trip pin the new variant. **Done** —
   `f6ae53e`.
2. **Luna in the picker.** `chat_models` always offers `ChatModel { provider: OpenAi, id:
   "gpt-6-luna" }`, and the `chat_model` table remembers it across launches. `provider_for`
   needs an arm for it, so the drawer can pick Luna but gets a canned reply or a clear
   "not wired yet" error until Step 3.
3. **The real Luna stream.** The `chat/completions` client takes its base URL. Zen keeps
   `https://opencode.ai/zen/v1`, and OpenAI uses `https://api.openai.com/v1`. Tests pin the
   URL each provider posts to, and an `#[ignore]` test streams a real Luna reply with
   `OPENAI_API_KEY`, as Zen's does with `OPENCODE_API_KEY`.
4. **Luna starts sooner.** First a probe: time to first text for `reasoning_effort` `none`,
   `low` and the default `medium` on a short prompt, recorded in a table as in Phase 23 Step
   10. Then the request body gains an optional `reasoning_effort`, set for OpenAI and
   skipped when serializing for Zen. A test asserts Zen's body has no such key.
5. **Review and refactor.**

**Why this order.** Each step ends with something to see:

- **Step 1** is the settings block, as in Phase 23. It's the entry point, and the key has
  to exist before anything can use it.
- **Step 2** puts Luna in the drawer before it can talk, so the routing (`ChatModel` →
  `Provider::OpenAi` → client) is visible and tested on its own.
- **Step 3** is where the base-URL extraction lands, alongside its second caller.
- **Step 4** comes after a working stream, because it can't be measured without one, and
  because Phase 23 showed that measuring can change the plan.

Step details are written by `lbb:next` when this phase starts.

## Step 1 — The OpenAI block and its key

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `f6ae53e` (235 tests green, 4 ignored; clippy clean). The settings block is still to be confirmed by hand under `dx serve`.

**What it is.** Settings → AI now shows three blocks: *Gemini*, *OpenCode Zen* and
*OpenAI*. The OpenAI block has only a key row, with a one-line note under it saying that
OpenAI keeps API requests for 30 days by default. The key goes to its own keychain entry.
Nothing uses it for chat yet.

### Runnable check first

`src/ai/mod.rs` — the label and slug tests now read `Provider::ALL`, so they pin the
order and the length of the list, not only the two variants they name:

```rust
#[test]
fn each_provider_has_a_reader_facing_label() {
    assert_eq!(
        Provider::ALL.map(Provider::label),
        ["Gemini", "OpenCode Zen", "OpenAI"]
    );
}

#[test]
fn each_provider_has_a_stable_storage_slug() {
    assert_eq!(
        Provider::ALL.map(Provider::slug),
        ["gemini", "opencode-zen", "openai"]
    );
}
```

The slug round trip loops over `ALL` as before. Its "unknown slug" case used `"openai"`,
which is now known, so it uses `"anthropic"`.

`src/secrets/api_key.rs` — the OpenAI key lives apart from the other two:

```rust
#[test]
fn the_openai_key_lives_apart_from_the_other_two() {
    let store = Memory::default();
    save(&store, Provider::Gemini, "gemini").expect("save the Gemini key");
    save(&store, Provider::OpenCodeZen, "zen").expect("save the Zen key");

    save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");
    assert_eq!(
        get(&store, Provider::OpenAi).expect("read the OpenAI key"),
        Some("openai".to_owned())
    );

    forget(&store, Provider::OpenAi).expect("forget the OpenAI key");
    assert!(!is_set(&store, Provider::OpenAi).expect("read the OpenAI key"));
    assert_eq!(
        [Provider::Gemini, Provider::OpenCodeZen]
            .map(|provider| get(&store, provider).expect("read a key")),
        [Some("gemini".to_owned()), Some("zen".to_owned())]
    );
}
```

`src/ai_client/mod.rs` — a saved OpenAI key builds no client yet:

```rust
#[test]
fn an_openai_key_builds_no_client_yet() {
    let store = Memory::default();
    let luna = ChatModel {
        provider: Provider::OpenAi,
        id: "gpt-6-luna".to_owned(),
    };
    api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");

    assert!(provider_for(&store, &luna).expect("read the key").is_none());
}
```

Red against `todo!()` arms for `OpenAi` in `label`, `slug`, `entry` and `provider_for`:
6 failed, 229 passed, 4 ignored.

```
test ai::test::a_provider_slug_reads_back_and_an_unknown_one_does_not ... FAILED
test ai::test::each_provider_has_a_reader_facing_label ... FAILED
test ai_client::test::an_openai_key_builds_no_client_yet ... FAILED
test ai::test::each_provider_has_a_stable_storage_slug ... FAILED
test db::chat_model::test::a_pick_from_an_unknown_provider_reads_as_no_pick ... FAILED
test secrets::api_key::test::the_openai_key_lives_apart_from_the_other_two ... FAILED
```

The fifth one wasn't planned. `db::chat_model`'s test wrote `'openai'` into the table as a
provider the app doesn't know. That was a tripwire nobody meant to plant, and it fired here.
It now writes `'anthropic'`.

Green after: 235 passed, 4 ignored, and clippy was clean.

**Eyeball under `dx serve`:** gear → *AI* shows *Gemini*, *OpenCode Zen*, then *OpenAI*,
with the 30-day note under the OpenAI block. Save an OpenAI key and its status reads *Key
set* while the other two don't change. Relaunch the app and it's still *Key set*. *Forget*
returns it to *Not set*. The drawer's picker shows no *OpenAI* group yet.

### Minimal implementation

- **`src/ai/mod.rs`** — `Provider::OpenAi`, labelled `"OpenAI"`, slug `"openai"`, and
  `ALL` grows to `[Provider; 3]`.
- **`src/secrets/api_key.rs`** — a private `OPENAI_API_KEY = "openai-api-key"` and a third
  arm in `entry`.
- **`src/ai_client/mod.rs`** — `provider_for` maps the key with `and_then` instead of
  `map`. Each arm returns `Option<AnyProvider>`, and `OpenAi` returns `None`.
- **`src/ui/settings.rs`** — `OpenAiSettings`: a heading, a group with one
  `ApiKeyControl`, and a `p` note after the group.
- **`src/ui/settings_screen.rs`** — the *AI* arm renders `OpenAiSettings {}` after Zen.
- **`src/ui/settings.css`** — `.settings_group_note`, the same small, faded type as the
  group heading, without the capitals.

### Why it works

- **The compiler is the checklist.** Adding a variant to a closed enum breaks every
  exhaustive `match` on it: `label`, `slug`, `entry` and `provider_for`. Those four are
  everything that has to answer for OpenAI. `from_slug` and the drawer's picker loop over
  `ALL`, so they pick up the variant without an edit.
- **`ALL` in the tests.** Writing `Provider::ALL.map(...)` instead of a hand-made array
  means a fourth variant fails the test until someone adds its label. A hand-made array
  would pass quietly with the new variant missing.
- **`map` → `and_then`.** `map` can only turn a key into a client. `and_then` lets an arm say
  "a key, but no client", which is what OpenAI is until Step 3. The two other arms wrap
  their client in `Some`.
- **The picker skips an empty provider.** `ModelPicker` renders an `optgroup` only when
  `chat_models` returns a model for that provider. `chat_models` doesn't know OpenAI yet,
  so there's no empty *OpenAI* heading in the drawer.
- **The note sits outside the group.** Inside `.settings_group`, every child after the
  first gets a divider (`> * + *`), so the note would look like a row. Below the group it
  reads as a footnote, the way iOS settings put explanations under a section.

### Forks taken

- **`provider_for` returns `None` for OpenAI, over a canned reply or an error.** With
  `None`, choosing an OpenAI model looks the same as having no key, so the drawer shows its
  existing "no key" state. Nothing can pick an OpenAI model before Step 2 anyway. The step
  plan says Step 2 gives the drawer a canned reply or a clear "not wired yet" error, and
  that's the place to choose between them, because that's when it can be seen.
- **A local `use_key_set` bool, like Zen in Phase 23 Step 1.** The key has no consumer, so it
  isn't put in context.
- **A third near-copy of the key block, over a shared `ProviderKeyBlock { provider }`.**
  `GeminiSettings` and `OpenAiSettings` differ only in the provider and the note, and Zen
  adds a catalog. Folding them is a refactor of code this step didn't write, so it's left
  for Step 5.
- **The note is plain text, with no link to OpenAI's data-controls page.** A link would
  have to open outside the app's webview, which nothing in Settings does yet.

### Scope note

- No Luna in the picker. That's Step 2.
- No OpenAI client and no base-URL parameter. That's Step 3.
- If a block is ever added after OpenAI, `.settings_group + .settings_group_title` won't
  match across the note, so the new heading loses its top margin. OpenAI is last, so it
  doesn't come up yet.

### Review notes (from the `simplify` pass)

- **Applied:** nothing. The diff is one variant, one constant, one arm change and one block
  copied from an existing one.
- **Skipped:** extracting a shared key-block component (see *Forks taken*). It's a
  candidate for the Step 5 review.
- **Skipped:** naming the OpenAI arm `_ => None` in `provider_for`. A wildcard would
  silently cover a fourth provider, which is exactly what the exhaustive match is for.
