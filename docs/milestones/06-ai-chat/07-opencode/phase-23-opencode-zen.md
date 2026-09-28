# Phase 23 — Second provider: OpenCode Zen

[← Feature: Second provider: OpenCode Zen](README.md) · **Status:** 🚧 in progress — opened 2026-09-27,
paused for [Phase 24 — Settings screen](../06-settings-screen/phase-24-settings-screen.md),
resumed 2026-09-28 ·
build log: [`phase-23-opencode-zen-steps.md`](phase-23-opencode-zen-steps.md)

## Goal

In **Settings → AI**, an *OpenCode Zen* block sits beside the Gemini one. It has a key row
and the **full list of Zen models, fetched live**, where you tick the ones you want offered
in chat. The chat drawer gets its own model picker listing the Gemini models plus the Zen
models you ticked, and the reply streams from whichever provider serves the chosen model.
The phase closes when a real Zen reply streams into the drawer on desktop and on the iOS
simulator.

> **Taken ahead of Phase 21** on 2026-09-27. **Replanned the same day, before Step 1 was
> written:** the first plan pinned two Zen models in code (`AiModel::DeepSeekFlash`,
> `AiModel::Kimi`) and kept the picker in the reader popover. The learner wants to choose
> from Zen's live list instead, and have the choice appear in the chat. That needs a
> settings screen with room for it, so [Phase 24](../06-settings-screen/phase-24-settings-screen.md)
> was inserted first, and this phase resumes once Phase 24 is done.

## The crux

**Models stop being a closed set.** Today `AiModel` is an enum. Every model is known at
compile time, it's `Copy`, and it lives in `Settings`. Zen's list is data fetched at runtime,
so a chosen model becomes a value: roughly `ChatModel { provider: Provider, id: String }`.
Three consequences follow, and each is a lesson:

1. **`String` isn't `Copy`, and `Settings` is.** Putting a `ChatModel` inside `Settings`
   would drop `Copy` from `Settings`, and every `settings()` read in the app would start
   cloning or fail to compile. The chosen model and the ticked list get **their own home**
   instead: a SQLite table and their own signal, next to `Settings` rather than inside it.
2. **The provider is still a closed set.** Models are open, but *who serves them* is not:
   `Provider { Gemini, OpenCodeZen }` stays an enum. The runtime pick between the two
   clients is `AnyProvider { Gemini(..), OpenCode(..) }`, an enum that implements
   `ChatProvider` by delegating, because a trait with an `async fn` can't be a `dyn` object.
3. **A catalog is a network call outside chat.** Fetching `GET /zen/v1/models` is the first
   request the app makes that isn't a reply. It needs its own loading and error state in the
   settings UI (Dioxus' `use_resource`), separate from the drawer's.

## Design decisions (recorded up front)

- **Show Zen's whole list, filter nothing.** `/zen/v1/models` returns bare ids with no hint
  of which endpoint serves them. Zen serves Claude over `messages`, GPT over `responses`,
  and Qwen/DeepSeek/GLM/Kimi over `chat/completions`, and the families don't map cleanly
  (`qwen3.8-flash` is on `messages`). This phase speaks **only `chat/completions`**, so
  ticking a Claude or GPT model gives an API error in the drawer. That's acceptable for a
  list the learner curates by hand. A per-model format table is a later refinement if it
  bites. Known-good picks checked 2026-09-27: `deepseek-v4-flash`, `glm-5.3-flash`,
  `kimi-k2.6`.
- **Gemini's models stay fixed** (Flash-Lite, Flash) and are always offered in chat. A
  Gemini catalog could reuse the same tick-list later; that isn't needed now.
- **The model choice moves into the chat drawer.** Phase 24 moves the Gemini model picker
  to Settings → AI → Gemini. This phase replaces it with a picker in the drawer, because
  that's where you'd switch.
- **Separate keychain entry per provider.** Switching models never forgets either key.
- **OpenCode Go is out of scope.** Its terms target coding-agent traffic.

## Planned steps

- [x] 1. The Zen block and its key — `Provider`, a second key row in Settings → AI, key status read from the store
- [x] 2. The Zen catalog — fetch `/zen/v1/models`, show the ids, parse under `#[test]`
- [x] 3. Choose models for chat — ticks on the catalog, saved in an `ai_models` table
- [ ] 4a. The model picker moves into the chat — Gemini + ticked Zen models, `ChatModel` kept out of `Settings`
- [ ] 4b. The chosen model persists — a `chat_model` table, and `ai_model` leaves `Settings`
- [ ] 5. The drawer talks to either provider — `AnyProvider`, an `OpenCode` that returns a canned reply
- [ ] 6. The `chat/completions` wire format — request body, chunk text, `[DONE]`, under `#[test]`
- [ ] 7. The real Zen stream — POST, Bearer auth, end to end with a real key
- [ ] 8. Review and refactor
