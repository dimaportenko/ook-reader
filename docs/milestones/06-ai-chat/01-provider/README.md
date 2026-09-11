# Feature — Provider boundary

[← Milestone 6](../README.md)

The part of AI Chat that has no UI: a `ChatProvider` trait the reader talks to, a Gemini
implementation behind it, and the prompt template that turns *book + chapter + selection*
into a first message. Everything here is `cargo test`-able offline; the one network call is
behind an `#[ignore]` test that needs a key.

| # | Phase | Status |
|---|---|---|
| 17 | [Provider boundary + Gemini](phase-17-provider-boundary.md) | 🚧 in progress |
