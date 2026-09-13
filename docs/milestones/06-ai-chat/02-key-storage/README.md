# Feature — Key storage

[← Milestone 6](../README.md)

The part of AI Chat that makes the provider usable outside a test: the Gemini key lives in
the OS keychain, never in SQLite; the settings panel gets a row to enter it and a line that
says only *key set* / *not set*; the model is picked from a short list. Everything but the
real keychain is `cargo test`-able; the keychain itself sits behind an `#[ignore]` test, the
same shape Phase 17 used for the network.

| # | Phase | Status |
|---|---|---|
| 18 | [API key storage + settings row](phase-18-key-storage.md) | 🚧 in progress |
