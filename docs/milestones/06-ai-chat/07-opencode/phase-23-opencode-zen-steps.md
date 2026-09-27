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

1. **The Zen block and its key.** `Provider`, a second key row in Settings → AI, and key status read from the store.
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
