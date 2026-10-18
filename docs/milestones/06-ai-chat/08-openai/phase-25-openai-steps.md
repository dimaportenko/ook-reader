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
2. ~~**Luna in the picker.**~~ `chat_models` always offers `ChatModel { provider: OpenAi, id:
   "gpt-6-luna" }`, and the `chat_model` table remembers it across launches. `provider_for`
   needs an arm for it, so the drawer can pick Luna but gets a canned reply or a clear
   "not wired yet" error until Step 3. **Done** — `289accb`.
3. ~~**The real Luna stream.**~~ The `chat/completions` client takes its base URL. Zen keeps
   `https://opencode.ai/zen/v1`, and OpenAI uses `https://api.openai.com/v1`. Tests pin the
   URL each provider posts to, and an `#[ignore]` test streams a real Luna reply with
   `OPENAI_API_KEY`, as Zen's does with `OPENCODE_API_KEY`. **Done** — `463d0a3`.
4. ~~**Luna starts sooner.**~~ First a probe: time to first text for `reasoning_effort` `none`,
   `low` and the default `medium` on a short prompt and a book-chat prompt, recorded in a
   table as in Phase 23 Step 10. Then the request body gains an optional
   `reasoning_effort`, set to `none` for OpenAI and skipped when serializing for Zen. A test
   asserts Zen's body has no such key. **Done** — `8cf7672`.
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

## Step 2 — Luna in the picker

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `289accb` (236 tests green, 4 ignored; clippy clean). The picker and the "not wired yet" note are still to be confirmed by hand under `dx serve`.

**What it is.** The drawer's model picker always shows an *OpenAI* group with
`gpt-6-luna`, after Gemini and the ticked Zen models. The pick survives a relaunch. With
no OpenAI key, choosing Luna shows the drawer's existing "Add your OpenAI key in settings
to chat." note. With a key, the compose box appears, and sending a message shows a red
*OpenAI chat isn't wired up yet* under it. Step 3 replaces that with the real stream.

### Runnable check first

`src/ai/mod.rs` — the offer list ends with Luna, and Luna is one fixed id:

```rust
#[test]
fn the_chat_offers_every_gemini_model_then_the_ticked_zen_ones_then_luna() {
    // …the same ticked set as before…
    assert_eq!(
        chat_models(&ticked),
        [
            ChatModel::gemini(GeminiModel::FlashLite),
            ChatModel::gemini(GeminiModel::Flash),
            zen("deepseek-v4-flash"),
            zen("kimi-k2.6"),
            ChatModel::luna(),
        ]
    );
}

#[test]
fn luna_is_openai_s_gpt_6_luna() {
    assert_eq!(
        ChatModel::luna(),
        ChatModel {
            provider: Provider::OpenAi,
            id: "gpt-6-luna".to_owned(),
        }
    );
}
```

`src/db/chat_model.rs` — the round-trip test gains a third pick, after Zen and Gemini:

```rust
db.save_chat_model(&ChatModel::luna()).expect("pick Luna");
assert_eq!(
    db.chat_model().expect("read the Luna pick"),
    Some(ChatModel::luna())
);
```

`src/ai_client/mod.rs` — replaces Step 1's `an_openai_key_builds_no_client_yet`:

```rust
#[test]
fn an_openai_key_builds_a_client_that_says_it_is_not_wired_yet() {
    let store = Memory::default();
    let luna = ChatModel::luna();
    assert!(provider_for(&store, &luna).expect("read the key").is_none());

    api_key::save(&store, Provider::OpenAi, "openai").expect("save the OpenAI key");
    let client = provider_for(&store, &luna)
        .expect("read the key")
        .expect("an OpenAI key builds a client");
    let result = pollster::block_on(client.stream(&[Message::user("Hi")], |_| {}));

    assert!(matches!(result, Err(ChatError::NotWired)), "{result:?}");
}
```

Red against a `todo!()` body for `ChatModel::luna()` and the new `AnyProvider` arm, with
`provider_for` still returning `None` for OpenAI: 4 failed, 232 passed, 4 ignored.

```
test ai::test::luna_is_openai_s_gpt_6_luna ... FAILED
test ai_client::test::an_openai_key_builds_a_client_that_says_it_is_not_wired_yet ... FAILED
test ai::test::the_chat_offers_every_gemini_model_then_the_ticked_zen_ones_then_luna ... FAILED
test db::chat_model::test::the_chosen_chat_model_round_trips_and_the_latest_pick_wins ... FAILED
```

Green after: 236 passed, 4 ignored, and clippy was clean.

**Eyeball under `dx serve`:** open the chat drawer. The picker shows *Gemini*, *OpenCode
Zen* (if any are ticked), then *OpenAI* with `gpt-6-luna`. Pick it without an OpenAI key:
the drawer asks for the OpenAI key. Save a key in Settings → AI and come back: the compose
box shows, and sending a message puts *OpenAI chat isn't wired up yet* in red under it.
Relaunch the app and Luna is still picked.

### Minimal implementation

- **`src/ai/mod.rs`** — `ChatModel::luna()` builds `{ OpenAi, "gpt-6-luna" }`.
  `chat_models` chains `[ChatModel::luna()]` after the Zen models. `ChatError` gains
  `NotWired` ("OpenAI chat isn't wired up yet"). `AnyProvider` gains a unit variant
  `OpenAiNotWired`, whose `stream` returns `Err(ChatError::NotWired)` straight away.
- **`src/ai_client/mod.rs`** — the OpenAI arm builds `AnyProvider::OpenAiNotWired`. Every
  arm now builds a client, so `and_then` goes back to `map`.
- **`src/db/chat_model.rs`** — no code change. The table stores the slug and the id, and
  `from_slug` already knows `openai` since Step 1.

### Why it works

- **The picker reads `chat_models`.** `ModelPicker` renders an `optgroup` for each
  provider that has a model in the list, so one more entry gives the *OpenAI* group. Its
  `onchange` looks the pick up in the same list by `key()`, `openai/gpt-6-luna`.
- **The startup fallback leaves Luna alone.** `main.rs` resets the pick to the first
  offered model when the saved one isn't offered any more, as when a Zen model is unticked.
  Luna is always offered, so a saved Luna pick survives that check.
- **The table never needed a change.** It stores `provider.slug()` and `model.id` as text,
  and reads back through `Provider::from_slug`. Step 1 made `openai` a known slug, so the
  round trip already worked. The test pins it so a later change can't drop it.
- **The error goes through the normal failure path.** `Conversation::settle(Err(e))` turns
  any `ChatError` into `Status::Failed(e.to_string())`, which the drawer shows as a red
  alert. The new variant needs no UI code. The question stays in the history, as it does
  after a real network error.
- **`and_then` back to `map`.** Step 1 used `and_then` so one arm could say "a key, but no
  client". Now every arm builds one, and clippy's `bind_instead_of_map` flagged it.

### Forks taken

- **A clear error over a canned reply.** A canned reply would look like Luna answered,
  and it would sit in the history as an assistant message. The error says what's true,
  and it goes through the existing failure path with no UI work. The cost is a variant on
  `AnyProvider` and one on `ChatError` that exist only until Step 3, which deletes both.
- **The error only shows on send, not on pick.** Showing it on pick would need a third
  drawer state between "no key" and "ready". Showing it on send keeps the drawer's two
  states and costs one keystroke.
- **The id lives in `ChatModel::luna()`, not a `LunaModel` enum or a `const`.** Gemini
  has an enum because it has two models and a default. Luna is one id with nothing to
  choose between, so a constructor next to `ChatModel::gemini` is enough. Adding Sol
  later is the point at which an `OpenAiModel { Luna, Sol }` with an `ALL` earns its place.
- **`OpenAiNotWired` is a unit variant.** It doesn't keep the key or the id. Step 3 needs
  both, but it builds them into the real client, not into this placeholder.

### Scope note

- No request reaches OpenAI. That's Step 3, which also removes `OpenAiNotWired` and
  `ChatError::NotWired`.
- The saved key is read and then dropped in the OpenAI arm. Step 3 passes it to the client.

### Review notes (from the `simplify` pass)

- **Applied:** `and_then` → `map` in `provider_for` (clippy's suggestion, see above).
- **Skipped:** `std::iter::once(ChatModel::luna())` over the one-element array. Both read
  the same, and the array is shorter.
- **Skipped:** naming the error after the model ("GPT-6 Luna isn't wired up yet"). The
  gap is the OpenAI client, not Luna, and Step 3 closes it for every OpenAI id at once.

## Step 3 — The real Luna stream

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `463d0a3` (237 tests green, 5 ignored; clippy clean; the live Luna test passed). The drawer stream is still to be confirmed by hand under `dx serve`.

**What it is.** Picking *gpt-6-luna* with an OpenAI key saved now streams a real reply
from `api.openai.com` into the drawer. The red *OpenAI chat isn't wired up yet* from Step 2
is gone. Zen streams exactly as before.

### Runnable check first

`src/ai/opencode.rs` — the Zen request test now names Zen's URL as a literal instead of
the deleted const, and a new test pins OpenAI's host with the same body:

```rust
#[test]
fn an_openai_request_posts_the_same_body_to_openai() {
    let messages = [Message::user("Which city?")];
    let zen = Completions::zen("zen-key".to_owned(), "gpt-6-luna");
    let openai = Completions::openai("openai-key".to_owned(), "gpt-6-luna");

    let zen_request = zen.request(&messages).build().unwrap();
    let request = openai.request(&messages).build().unwrap();

    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(request.headers()["authorization"], "Bearer openai-key");
    assert_eq!(
        request.body().unwrap().as_bytes(),
        zen_request.body().unwrap().as_bytes()
    );
}
```

`src/ai_client/mod.rs` — replaces Step 2's `an_openai_key_builds_a_client_that_says_it_is_not_wired_yet`.
It goes through `provider_for`, so it pins the routing as well as the URL:

```rust
#[test]
fn zen_and_luna_post_to_their_own_hosts() {
    // …save a Zen key and an OpenAI key…
    let url = |model: &ChatModel| match provider_for(&store, model).expect("read the key") {
        Some(AnyProvider::Completions(client)) => client
            .request(&[Message::user("Hi")])
            .build()
            .expect("build the request")
            .url()
            .to_string(),
        _ => panic!("{} builds no chat/completions client", model.key()),
    };

    assert_eq!(url(&zen), "https://opencode.ai/zen/v1/chat/completions");
    assert_eq!(
        url(&ChatModel::luna()),
        "https://api.openai.com/v1/chat/completions"
    );
}
```

And `a_real_luna_streams_through_the_trait`, `#[ignore = "needs OPENAI_API_KEY and the
network"]`, the same as Zen's live test with `Completions::openai(key, "gpt-6-luna")`.

Red against a `todo!()` body for `Completions::openai`: 2 failed, 235 passed, 5 ignored.

```
test ai::opencode::test::an_openai_request_posts_the_same_body_to_openai ... FAILED
test ai_client::test::zen_and_luna_post_to_their_own_hosts ... FAILED
```

Green after: 237 passed, 5 ignored, and clippy was clean.

**The live run.** `cargo test a_real_luna_streams_through_the_trait -- --ignored` passed
in 2.6s. A probe with the same prompt outside the test showed what the stream carries:

| | Luna, "Count from one to twenty in words" |
|---|---|
| chunks before `[DONE]` | 42 (40 with text) |
| time to first text | ~1.6s |
| whole reply | ~1.4–2.0s |
| fields in `delta` | `role`, `content`, `refusal` |

- **No reasoning field.** OpenAI's `chat/completions` doesn't stream the model's reasoning,
  so there's no `reasoning_content` and the drawer never shows *Thinking…* for Luna. The
  wait before the first word is silent, which is what Step 4 measures.
- **Two fields Zen doesn't send:** `refusal` in the delta and `obfuscation` on the chunk.
  `serde` ignores unknown fields, so the parser needed no change.

**Eyeball under `dx serve`:** pick *gpt-6-luna* with an OpenAI key saved and send a
message. The reply streams in word by word, and Stop works mid-reply. Then pick a Zen
model and check that it still answers.

### Minimal implementation

- **`src/ai/opencode.rs`** — `OpenCode` becomes `Completions`, with a `base_url: &'static
  str` field. `COMPLETIONS_URL` is replaced by `ZEN_BASE_URL` and `OPENAI_BASE_URL`. A
  private `new(base_url, key, model)` is called by two public constructors, `zen` and
  `openai`. `request` builds `{base_url}/chat/completions` and is now `pub(crate)`, so the
  routing test can inspect it.
- **`src/ai/mod.rs`** — `AnyProvider::OpenCode(OpenCode)` becomes
  `AnyProvider::Completions(Completions)`. `OpenAiNotWired` and `ChatError::NotWired` are
  deleted.
- **`src/ai_client/mod.rs`** — the Zen arm calls `Completions::zen`, and the OpenAI arm
  calls `Completions::openai` with the saved key and the model id.

### Why it works

- **The host was the only difference.** The body, the Bearer header, the SSE framing and
  `[DONE]` are the same on both hosts, so moving the URL into a field is the whole change.
  The body test compares the two requests' bytes directly, so a later per-host field
  shows up there first. Step 4 adds one on purpose.
- **Named constructors keep the URLs private.** `provider_for` says *which host*, not
  *what URL*. The two URLs live next to the code that appends `/chat/completions` to them.
- **`&'static str` for the base URL.** Both hosts are compile-time constants, so the
  client borrows them and `Clone` stays cheap. A user-configurable host would need a
  `String`, and nothing asks for one.
- **Inspect the request, don't send it.** `RequestBuilder::build()` gives a
  `reqwest::Request` with its URL, headers and body, so the URL tests run offline in
  microseconds. Only the `#[ignore]` test touches the network.
- **The compiler cleaned up Step 2.** Deleting `OpenAiNotWired` broke `AnyProvider::stream`
  and `provider_for`. Deleting `NotWired` broke Step 2's test, which this step replaces.

### Forks taken

- **One `Completions` variant, not an OpenAI arm.** Both hosts are the same type with a
  different URL, so a second arm would hold the same type and forward the same call. If
  Step 4's `reasoning_effort` makes them behave differently, that's a field on the
  client, not a new type. The cost is that `matches!(…, AnyProvider::Completions(_))`
  can't tell Zen from OpenAI, which is why the routing test reads the URL instead.
- **Renamed the type, kept the file.** `OpenCode` would be the wrong name for a client
  that talks to OpenAI, so the type became `Completions`. The file stays `opencode.rs`,
  because it also holds Zen's `/models` catalog, which OpenAI doesn't use. Splitting it
  into `completions.rs` and a small `opencode.rs` is a candidate for the Step 5 review.
- **`MODELS_URL` stays a full literal.** It could be built from `ZEN_BASE_URL`, but a
  `const` can't use `format!`, and a `LazyLock` for one URL is more code than it saves.
- **`request` is `pub(crate)`, not test-only.** A `#[cfg(test)]` accessor would hide the
  real method behind a copy. Making the method visible inside the crate costs nothing.

### Scope note

- No `reasoning_effort` yet. That's Step 4, which starts from the ~1.6s to first text
  above.
- The file split is left for Step 5.

### Review notes (from the `simplify` pass)

- **Applied:** nothing. The diff is a rename, one field, two constructors and two deletions.
- **Skipped:** dropping the private `new` and writing the struct literal in both
  constructors. Three lines saved, and the constructors would no longer read as "the same
  client, a different host".

## Step 4 — Luna starts sooner

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `8cf7672` (238 tests green, 5 ignored; clippy clean; the live Luna test passed with `reasoning_effort: "none"`). The quicker drawer reply is still to be confirmed by hand under `dx serve`.

**What it is.** Every OpenAI request asks for `reasoning_effort: "none"`, so a Luna reply
to a quoted passage starts in under a second instead of after about seven. Zen's requests
are byte-for-byte what they were.

### The measurement first

A throwaway Python probe (`http.client`, not `urllib`, with its own User-Agent) streamed
`gpt-6-luna` over `chat/completions` five times per config, on 2026-10-18. *Short* is Step
3's "Count from one to twenty in words". *Book* is a `prompt::draft`-shaped message: the
Ukrainian "translate and explain" line, the three opening lines of *The Colour of Magic*
quoted, and the question "why is the turtle swimming through the interstellar gulf?".
Medians of five, first text / whole reply:

| `reasoning_effort` | Short | Book |
|---|---|---|
| absent (the default) | 1.40s / 1.72s | **7.16s** / 11.71s |
| `"none"` | 0.91s / 1.26s | **0.74s** / 4.91s |
| `"low"` | 0.84s / 1.16s | 3.26s / 7.16s |
| `"medium"` | 1.00s / 1.37s | 7.16s / 11.46s |

- **The book prompt is where it shows.** The short prompt needs no thought at any level,
  so every row is about a second. The book prompt makes Luna think for 4–12s before the
  first word at the default, and that wait is silent, because OpenAI doesn't stream
  reasoning over `chat/completions` (Step 3). `"none"` cuts it to under a second and the
  whole reply to less than half.
- **Absent behaves like `"medium"`** on the book prompt, which fits `medium` being the
  default.
- **Quality held.** All three answers translated the passage, explained Discworld's turtle
  and elephants, and caught the *secondhand dimensions* joke. `low` and `medium` also
  caught the *plane* / aeroplane pun, and `none` missed it. That's the trade: a slightly
  shallower explanation for a reply that starts ten times sooner.
- **`"low"` isn't a middle ground** worth picking. It still waits about three seconds on
  the book prompt, which is the silent pause this step is about.

So the plan held, unlike Phase 23 Step 10: Luna can turn reasoning off, and turning it off
is the difference between a second and seven.

### Runnable check first

`src/ai/opencode.rs` — Step 3's `an_openai_request_posts_the_same_body_to_openai` was the
tripwire for this step: it compared the two hosts' body bytes, and "the same body" is no
longer true. It becomes:

```rust
#[test]
fn an_openai_request_posts_the_zen_body_plus_no_reasoning_to_openai() {
    // …zen and openai clients for "gpt-6-luna", as before…
    let mut expected = body(&zen_request);
    expected["reasoning_effort"] = json!("none");

    // …method, URL and Bearer asserts as before…
    assert_eq!(body(&request), expected);
}

#[test]
fn a_zen_request_carries_no_reasoning_effort() {
    let zen = Completions::zen("zen-key".to_owned(), "glm-5.3-flash");

    let request = zen
        .request(&[Message::user("Which city?")])
        .build()
        .unwrap();

    assert_eq!(body(&request).get("reasoning_effort"), None);
}
```

`body` is a small test helper that parses a built request's JSON. The Zen request test
uses it too.

Red with the field plumbed through and `Completions::openai` still passing `None`: 1
failed, 237 passed, 5 ignored. The OpenAI body was missing `"reasoning_effort": "none"`.

```
test ai::opencode::test::an_openai_request_posts_the_zen_body_plus_no_reasoning_to_openai ... FAILED
```

The Zen guard was green from the start, because `skip_serializing_if` came in with the
plumbing. To see it live, the attribute was removed for one run. Three tests went red:
the Zen guard (`left: Some(Null)`), the OpenAI test, and the whole-body
`the_conversation_becomes_chat_completions_messages`, which found `"reasoning_effort":
null`. Then it was put back.

Green after: 238 passed, 5 ignored, and clippy was clean. The live
`a_real_luna_streams_through_the_trait` passed again with the flag on, in 1.2s.

**Eyeball under `dx serve`:** select a paragraph, send it to the drawer with Luna picked,
and the reply should start in about a second rather than after a long `...`.

### Minimal implementation

- **`src/ai/opencode.rs`** — `CompletionRequest` gains `reasoning_effort:
  Option<&'static str>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
  `request_body` takes it as a third argument. `Completions` keeps it as a field next to
  `base_url`, the private `new` takes it, `zen` passes `None`, and `openai` passes
  `Some("none")`.

### Why it works

- **`skip_serializing_if` drops the key, not just the value.** Without it, `None`
  serializes as `"reasoning_effort": null`. GLM on Zen rejects the flag with a `400`
  (Phase 23 Step 10), and a `null` is still the flag being present. With it, Zen's JSON is
  exactly what it was before this step.
- **The effort is per host, so it lives with the host.** `base_url` and
  `reasoning_effort` are both decided by which constructor ran. `provider_for` still says
  only *which host*, and nothing outside `opencode.rs` knows the flag exists.
- **`get` over indexing in the Zen test.** `body["reasoning_effort"]` gives `Null` both
  for a missing key and for an explicit `null`, so it can't tell them apart. `get` returns
  `None` only when the key is absent, which is the property Zen needs.
- **The tripwire did its job.** Step 3 compared whole bodies so that a per-host field
  would break a test. The new version still compares whole bodies, with the one expected
  difference spelled out, so a second per-host field would break it again.

### Forks taken

- **`"none"`, not `"low"`.** The table above. `"low"` still waits about three seconds on
  a real passage.
- **A field on `Completions`, not a parameter to `request`.** The effort is fixed per
  host, like the URL. A parameter would push the choice into `AnyProvider` and
  `provider_for`, which only know about hosts.
- **`Option<&'static str>`, not a `ReasoningEffort` enum.** One value is ever sent. An enum
  earns its place if the effort becomes a setting ("think harder" in Settings → AI), which
  Phase 23 Step 9 also left for later.
- **No *Thinking…* for Luna.** With effort `"none"` there's nothing to wait for. If a
  later step raises the effort, the silent wait comes back, because `chat/completions`
  doesn't stream OpenAI's reasoning.

### Scope note

- The effort isn't a setting. Same reasoning as Gemini's `"minimal"` in Phase 23 Step 9.
- Sol, or any other OpenAI model, would get `"none"` too, since it's set per host. If one
  rejects it, the effort moves from the host to the model.
- The file split from Step 3's notes is still left for Step 5.

### Review notes (from the `simplify` pass)

The pass was run inline over the one-file diff.

- **Applied:** the Zen request test reuses the new `body` helper instead of parsing the
  request body inline.
- **Skipped:** a `const NO_REASONING: &str = "none"`. The literal appears once in the code
  and once in the test, which is the contract.
- **Skipped:** putting the effort in `request_body` behind a host check. The body builder
  would then need to know about hosts, and it's tested on its own without one.
