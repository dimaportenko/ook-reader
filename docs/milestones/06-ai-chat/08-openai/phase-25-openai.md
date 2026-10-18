# Phase 25 — Third provider: OpenAI

[← Feature: Third provider: OpenAI](README.md) · **Status:** 🚧 in progress — captured 2026-10-18,
opened the same day while [Phase 23](../07-opencode/phase-23-opencode-zen.md)'s review step
was still unwritten ·
build log: [`phase-25-openai-steps.md`](phase-25-openai-steps.md)

## Goal

In **Settings → AI**, an *OpenAI* block sits below Gemini and OpenCode Zen, with a key row.
The drawer's model picker always offers **GPT-6 Luna**, and choosing it streams the reply
from `api.openai.com`. The phase closes when a real Luna reply streams into the drawer and
starts about as soon as a Gemini reply does.

> **Captured 2026-10-18 through `lbb:refine`,** while Phase 23's last step (11, review and
> refactor) was still unwritten. The learner wants an OpenAI provider, mainly for the
> newest Luna model. It was split into its own phase rather than folded into Phase 23,
> because Phase 23's scope is Zen and its review step should close that scope. This phase
> has its own review at the end.

## The crux

**A second host for a wire format the app already speaks.** `OpenCode` is really a
`chat/completions` client with Zen's URL baked in. OpenAI's endpoint takes the same body,
the same Bearer auth, and returns the same SSE chunks ending in `[DONE]`. So the new
provider shouldn't be a copy of `opencode.rs`. It should be the existing client given a
**base URL**. That raises the lessons this phase is about:

1. **Extract after the second caller, not before.** Phase 23 kept Zen's URL as a `const`,
   which was right while there was one host. The second host is what justifies the
   parameter. The refactor lands *with* its second caller, in the step that needs it.
2. **`Provider` stays closed, and the client becomes shared.** `Provider { Gemini,
   OpenCodeZen, OpenAi }` grows a variant, and the compiler lists every `match` that has
   to answer for it. `AnyProvider` doesn't need a third arm if both hosts are the same
   client type with different URLs. Whether to give it one anyway is a fork to decide in
   Step 3.
3. **What's shared and what's per-host.** The body, the chunks and `[DONE]` are shared.
   The URL, the key and, from Step 4, `reasoning_effort` are per host. Zen's GLM rejects
   that flag, so it must never leak into Zen's requests.

## Design decisions (recorded up front)

- **Luna only, as a fixed id.** OpenAI's `/v1/models` lists hundreds of ids, including
  embeddings, TTS and image models. A tick-list over it would need filtering the API can't
  supply. Luna is hard-coded the way Gemini's models are, and adding Sol later is one line.
- **Always offered in the picker,** like Gemini. Without a key, the drawer's existing
  "no key for this provider" state explains it.
- **`chat/completions`, not `responses`.** It reuses the parser, SSE buffer and tests that
  already exist. `responses` would only pay off for tools or streamed reasoning summaries,
  and neither is in scope.
- **Its own keychain entry,** as with every provider. Saving or forgetting the OpenAI key
  touches no other key.

## Planned steps

- [x] 1. The OpenAI block and its key — `Provider::OpenAi`, a third keychain entry, a key row in Settings → AI
- [x] 2. Luna in the picker — `chat_models` always offers `openai/gpt-6-luna`, and the chosen-model table round-trips it
- [x] 3. The real Luna stream — the `chat/completions` client takes a base URL and serves both Zen and OpenAI, checked end to end with a real key
- [ ] 4. Luna starts sooner — measure time to first text per `reasoning_effort`, then send the winner to OpenAI only
- [ ] 5. Review and refactor
