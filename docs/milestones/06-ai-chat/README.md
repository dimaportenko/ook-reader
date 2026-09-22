# Milestone 6 — AI Chat

[← Roadmap](../../roadmap.md)

**Goal:** select a passage in the reader, pick *Ask AI* from the selection menu, and land in
a chat whose first message is already filled in — book title, author, the selected text —
so the only thing left to type is the question. First provider: **Gemini Flash-Lite**.
The provider and model must be swappable later without touching the reader.

**Status:** 🚧 in progress — captured 2026-09-11; [Phase 17](01-provider/phase-17-provider-boundary.md) closed 2026-09-13; [Phase 18](02-key-storage/phase-18-key-storage.md) closed 2026-09-20; [Phase 19](03-chat-panel/phase-19-chat-panel.md) closed 2026-09-22; [Phase 20](04-selection/phase-20-selection-chat.md) opened 2026-09-22. No ADR yet. Sits behind
nothing: Milestones 3 and 5 both have no phase in progress.

## The idea in one paragraph

The reader already knows everything the prompt needs: the book (`title`, `author` from the
`books` table), the chapter (Phase 8's "which chapter am I in?"), and — once the selection
is read out of the WebView — the passage. The chat feature is therefore mostly **plumbing
between three things that already exist and one that does not**: a selection entry point,
a small provider abstraction over an HTTP chat API, and a chat panel. The provider
abstraction is the part worth designing carefully; the rest is UI. Streaming is the one
piece of genuine new ground — the app has never held an open HTTP response — and it is
deliberately pushed to its own phase so the first end-to-end slice is request/response.

## Phases

| # | Phase | Outcome | Status |
|---|---|---|---|
| 17 | [Provider boundary + Gemini](01-provider/phase-17-provider-boundary.md) | A `ChatProvider` trait with one method, a Gemini implementation behind it, a `#[test]` against a fake — no UI, the key read from an env var | ✅ |
| 18 | [API key storage + settings row](02-key-storage/phase-18-key-storage.md) | Key entered in the settings panel, stored in the OS keychain (not SQLite), read back on launch; provider/model chosen from a list | ✅ |
| 19 | [Chat panel](03-chat-panel/phase-19-chat-panel.md) | A drawer showing a message list and an input; sends through the provider; conversation held in memory | ✅ |
| 20 | [Selection → prefilled chat (desktop)](04-selection/phase-20-selection-chat.md) | Read the selection out of the WebView via the existing eval bridge; a menu item opens the chat with the template filled in | 🚧 |
| 21 | Selection → prefilled chat (iOS) | *Ask AI* in the native edit menu via `buildMenuWithBuilder:` (objc2), reusing Phase 20's template | ⬜ |
| 22 | Streaming replies | Tokens appear as they arrive; cancel mid-reply | ⬜ |

> Phase files are written as each phase is picked up, per this repo's convention — the table
> is the plan, not a substitute for the steps.

**Why this order.** Phase 17 is first for the same reason Milestone 5 puts the merge engine
before auth: it needs no UI and no key management to be *correct*, and the trait it defines
is the one decision every later phase leans on. Phase 18 comes before any UI because a chat
panel with nowhere to put the key is untestable by hand. Phases 20 and 21 are split by
platform on purpose — the desktop path is pure Dioxus plus one `eval`, while the iOS path is
FFI into UIKit's responder chain and deserves its own scope and its own risks. Streaming is
last because a working non-streaming chat is already the feature; streaming is polish.

## The design, in short

- **Provider boundary.** One trait, roughly `async fn complete(&self, messages: &[Message])
  -> Result<Reply, ChatError>`; a `Message` is `{role, text}`. Implementations: `Gemini`
  now, a `Fake` for tests. Model name is a field on the provider, not a trait parameter, so
  "Flash-Lite → Flash" is a settings change. Provider-specific wire formats stay inside the
  implementation; the reader never sees a Gemini type.
- **Prompt template.** Built in Rust from `Book { title, author }`, the current chapter
  label, and the selection: a system-ish preamble ("You are helping a reader of *Title* by
  Author…") plus the quoted passage as the first user turn, with the cursor left after it
  for the question. Templates are plain functions and `#[test]`-able.
- **Selection readout.** `window.getSelection().toString()` through the existing
  `document::eval` bridge in `src/ui/reader.rs`, which already tracks selection for the
  swipe-vs-select gesture rule. Nothing DOM-positional is stored — this is *not*
  annotations, and it keeps the "resolve a position in the live DOM" problem out of scope.
- **Secrets.** The API key never goes into `settings` in SQLite (a plain-text file on disk).
  macOS/iOS Keychain via `objc2` (Security framework) or the `keyring` crate; the settings
  row shows only "key set / not set". Milestone 5 planned the same home for the OAuth refresh
  token, so this is the first tenant of a shared keychain helper.
- **HTTP.** The crate has no HTTP client, no async runtime and no `serde` yet. Dioxus desktop
  runs on tokio, so `reqwest` with `json` + `stream` features is the expected choice;
  `serde`/`serde_json` for the Gemini wire types. Phase 17 makes the call.
- **Conversation state.** In memory, per chat session, a `Signal<Vec<Message>>`. Persisting
  chats to SQLite is a later decision, not a Phase 19 requirement.
- **Entry point on desktop.** Either a right-click context menu on the reader (Dioxus
  `oncontextmenu`) or a small floating "Ask AI" button that appears while a selection is
  non-empty. The floating button also works on Android and web unchanged, so it is the
  default; the native iOS menu is the platform-specific upgrade.

## Known risks and open questions

- **iOS edit-menu customisation is fragile.** `buildMenuWithBuilder:` on a view controller
  above the private `WKContentView` works today, but Apple ignores removals of system items
  on some versions and Dioxus mobile does not expose the hosting `UIViewController`. Phase 21
  starts with a spike: find the window / root controller from `UIApplication` and prove one
  custom item shows up before designing anything on top of it.
- **Bundle size and build time.** `reqwest` pulls TLS and hyper. On iOS the default TLS
  backend needs checking (`rustls` vs native); a wrong choice shows up as a link error in
  `dx build --platform ios`, not on desktop.
- **Cost and rate limits.** Flash-Lite is cheap but not free; a long selected passage in every
  first turn adds up. Cap the selection length in the template (e.g. a few thousand
  characters) and say so in the UI when truncated.
- **Copyright of the passage.** Sending book text to a third-party API is the user's call
  with their own key; the feature ships nothing without a key the user entered. Worth one
  sentence in the settings row.
- **Web target.** WASM cannot reach the keychain and would expose the key in the browser.
  Out of scope, consistent with Milestone 5's web stance.
- **Multiple providers later.** OpenAI-compatible and Anthropic APIs differ in message shape
  (system prompt placement, streaming framing). The trait stays minimal so a second
  implementation is a new file, not a trait change — but do not add a second provider until
  a real one is wanted.

## Deliberately out of scope

Chat history persistence, tool use / function calling, RAG over the whole book, images in
prompts, the web target, and any provider beyond Gemini in the first pass.
