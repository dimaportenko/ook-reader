# Feature — Third provider: OpenAI

[← Milestone 6](../README.md)

> **Note 2026-10-18:** the OpenCode Zen provider has since been removed (see
> [Phase 23](../07-opencode/phase-23-opencode-zen.md)). The shared `chat/completions` client
> now serves OpenAI only, and the picker offers Gemini's models then Luna. Mentions of Zen
> below describe the app as it was when this phase closed.

The chat drawer can answer with **GPT-6 Luna** (`gpt-6-luna`), served directly by the
OpenAI API. **Settings → AI** gets an *OpenAI* block with a key row, and the drawer's picker
always offers Luna beside Gemini's models and the ticked Zen models. This is the third
`ChatProvider`, and the first one that speaks a wire format the app **already knows**: OpenAI's
`chat/completions` is the format Zen copied. So the lesson here is reuse. The Zen client
becomes one `chat/completions` client that serves two hosts.

| # | Phase | Status |
|---|---|---|
| 25 | [Third provider: OpenAI](phase-25-openai.md) | ✅ done — closed 2026-10-18 (`0b417cb`, 239 tests) |

## Why the API, and why Luna

Checked on 2026-10-18 against OpenAI's [GPT-6 Luna model page](https://developers.openai.com/api/docs/models/gpt-6-luna):

- **The model.** `gpt-6-luna` is the cheap tier of the GPT-6 family. It was announced
  2026-09-22 alongside GPT-6 Sol and is available in the API.
- **Endpoints.** It works over both `v1/chat/completions` and `v1/responses`, with
  streaming supported. Chat Completions limits function calling to `reasoning_effort: "none"`,
  and the app doesn't use tools, so that limit doesn't matter here.
- **Reasoning.** `reasoning_effort` takes `none`, `low`, `medium`, `high`, `xhigh` or
  `max`. The default is **`medium`**, which matters for how soon a reply starts (see Risks).
- **A key, not a subscription.** As with Zen, this goes through a pay-as-you-go API key from
  platform.openai.com. The ChatGPT-subscription route was ruled out in
  [Phase 23's README](../07-opencode/README.md#why-zen-not-go-or-a-chatgpt-subscription).

## Risks

- **Silent thinking.** Chat Completions doesn't stream reasoning text the way Zen's
  `reasoning_content` does. At the default `medium` effort, the bubble may sit on `...` and
  Step 10's *Thinking…* label will never show. Measure this first, as Phase 23 Step 10 did,
  before choosing an effort.
- **One flag doesn't fit every host.** Phase 23's measurement found that GLM-5.3 on Zen
  rejects every reasoning flag with a `400`. A `reasoning_effort` added to the shared request
  body has to be per-host, never sent to Zen.
- **Retention.** OpenAI keeps API requests for 30 days by default, and the book passage
  goes with them. Worth a sentence in the OpenAI block, as with Zen's privacy note.
- **Model churn.** `gpt-6-luna` is a fixed id in code. When a successor ships, it's a
  one-line change, the same as Gemini's `GeminiModel`.
