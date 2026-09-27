# Feature — Second provider: OpenCode Zen

[← Milestone 6](../README.md)

The chat drawer can answer with an **OpenCode Zen** model as well as Gemini. In
**Settings → AI** (built in [Phase 24](../06-settings-screen/phase-24-settings-screen.md)),
the reader pastes a Zen API key next to the Gemini one, loads Zen's live model list, and
ticks the models they want offered in the chat. The drawer's model picker lists those
alongside Gemini's, and the reply streams from whichever provider serves the chosen model.
This is the first time `ChatProvider` has **two real implementations**, which is the test
the Phase 17 boundary was designed to pass: a new provider should be a new file, not a
trait change.

| # | Phase | Status |
|---|---|---|
| 23 | [Second provider: OpenCode Zen](phase-23-opencode-zen.md) | ⏸ paused — waiting on Phase 24 |

## Why Zen, not Go or a ChatGPT subscription

Investigated 2026-09-27:

- **OpenCode Zen** is pay-as-you-go with an API key from opencode.ai/auth, base URL
  `https://opencode.ai/zen/v1/`, and is documented as usable from any app through standard
  SDKs. A reader app is fine here.
- **OpenCode Go** ($10/month) also issues a key, but its docs say it is "designed for OpenCode
  and other coding agents". They ask for a custom user agent and session headers and monitor
  traffic for abuse. A book chat isn't coding-agent traffic, so it's out.
- **A ChatGPT subscription** only reaches models through Codex's OAuth client and the
  undocumented `chatgpt.com/backend-api/codex` endpoint. That route is tolerated for coding
  tools, has a localhost-only redirect that doesn't work on iOS, and can be withdrawn at any
  time. Also out.

## Scope: one wire format

Zen serves different model families over different endpoints: `responses` for GPT,
`messages` for Claude and a few others, and `chat/completions` for DeepSeek, GLM, Kimi and
most of Qwen. `GET /zen/v1/models` lists bare ids and doesn't say which endpoint each one
uses. This phase speaks **only `chat/completions`**, the OpenAI-compatible shape:

- Request: `POST /zen/v1/chat/completions` with `Authorization: Bearer <key>` and
  `{ model, stream: true, messages: [{ role: "user" | "assistant", content }] }`.
- Reply: SSE `data:` lines, each a JSON chunk whose text is in `choices[0].delta.content`.
  The stream ends with the literal `data: [DONE]`, which isn't JSON and must be recognized
  before parsing.

The existing `src/ai/sse.rs` framing carries over unchanged. Only the JSON types and the
`[DONE]` marker are new.

## Risks

- **Ticked models on another wire format.** Choosing a Claude or GPT id in the chat gives
  an API error, because the catalog can't tell which formats work. That's acceptable for a
  hand-curated list. Revisit with a per-model format table if it keeps happening.
- **Model churn.** A ticked id can disappear from Zen's list. The drawer should still show
  it, and let the API error explain, rather than silently dropping it.
- **Privacy differs per model.** Zen says its `chat/completions` providers keep no data and
  don't train on it, while OpenAI and Anthropic models keep requests for 30 days. Worth a
  sentence in the Zen block.
- **Error bodies differ.** `ChatError::Api { status, body }` already carries the raw body.
