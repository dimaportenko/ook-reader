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

1. **The OpenAI block and its key.** `Provider::OpenAi` (slug `openai`), a third keychain
   entry, and an *OpenAI* block in Settings → AI with only a key row. Nothing uses the key
   yet. The `Provider::ALL` length and `from_slug` round trip pin the new variant.
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
