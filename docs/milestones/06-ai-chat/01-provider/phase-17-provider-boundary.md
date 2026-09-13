# Phase 17 — Provider boundary + Gemini

[← Feature: Provider boundary](README.md) · **Status:** 🚧 in progress — opened 2026-09-11 ·
build log: [`phase-17-provider-boundary-steps.md`](phase-17-provider-boundary-steps.md)

## Goal

A `ChatProvider` trait with one method, a `Gemini` implementation behind it, and a prompt
template — with the reader still knowing nothing about any of it. The phase closes when an
`#[ignore]`d test, run with a real key in an env var, gets a real answer back from
Gemini Flash-Lite through the trait, and every other test in the module runs without a
network.

## The crux

**The provider is a boundary, and the thing on the app's side of it must be boring.**
Gemini's wire format has `contents`, `parts`, `role: "model"`, and safety blocks; Anthropic's
has a top-level `system` and `role: "assistant"`; OpenAI's has `choices[0].message`. If any
of those names reach the reader, swapping providers later means editing the reader. So the
app-side vocabulary is fixed first — `Message { role, text }`, `Reply`, `ChatError` — and
each provider is a translation from that vocabulary to its own, hidden in its own file.

The second insight is about **what a test can reach**. The network is the one thing
`cargo test` should not touch, so the Gemini code is split at the HTTP call: *building* the
request body and *reading* a response body are pure functions over `serde` types and get
real tests against captured JSON; the `reqwest` call that joins them is a few lines with an
`#[ignore]` test. That split is not test ceremony — it is where the wire format lives, and
it is the part that changes when Google changes the API.

## Design decisions (recorded up front)

- **A new module, `src/ai/`**, with `mod.rs` (trait + app-side types), `prompt.rs` (the
  template), and `gemini.rs` (the one implementation). The reader will import from `ai`, never
  from `ai::gemini`.
- **`async fn` in the trait, no `async_trait` crate.** Rust 1.96 supports it natively. The
  cost is that the trait is not `dyn`-compatible, so a runtime-chosen provider needs generics
  or an enum. That is fine: this phase has one real provider and one test fake, and the fake
  is only ever named in tests. Revisit when a second real provider exists.
- **`pollster` as a dev-dependency** to `block_on` async tests — one function, no runtime.
  Dioxus already runs tokio at runtime, so `reqwest` works in the app without adding tokio as
  a direct dependency.
- **The key comes from `GEMINI_API_KEY` in this phase.** Keychain storage is Phase 18's whole
  job; here the constructor takes the key as a `String` and the ignored test reads the env.
- **Truncate the selection in the template**, not in the UI. A cap in one pure function is
  one test; a cap at every entry point is a bug farm.

## Planned steps

Detail for each lives in
[`phase-17-provider-boundary-steps.md`](phase-17-provider-boundary-steps.md).

- [x] **1. The vocabulary and the trait** — `src/ai/mod.rs`: `Role`, `Message`, `Reply`,
      `ChatError`, `trait ChatProvider { async fn complete }`, and a `Fake` in the test
      module that proves the trait can be implemented and called. `#[test]` via `pollster`.
      Committed in `aa29912`, **137 tests green**.
- [x] **2. The prompt template** — `src/ai/prompt.rs`: `draft(&Passage) -> String`, the
      prefilled input text, with the passage cap. `#[test]`. Committed in `e74b4ed`,
      **141 tests green**.
- [x] **3. Gemini request body** — `src/ai/gemini.rs`: `serde` types for `generateContent`
      and a pure `request_body(&[Message]) -> GenerateRequest`; assert the JSON shape.
      `#[test]`. Committed in `49f7902`, **143 tests green**.
- [ ] **4. Gemini response body** — the response types and `reply_from(GenerateResponse)
      -> Result<Reply, ChatError>`, against a captured success and an empty-candidates
      body. `#[test]`.
- [ ] **5. The HTTP call** — `reqwest` joins 3 and 4 inside `impl ChatProvider for Gemini`;
      `#[ignore]` test with `GEMINI_API_KEY`. Also `dx build --platform ios` to catch TLS.
- [ ] **6. Review and refactor** — punch-list over `src/ai/`, suite green, clippy clean.
