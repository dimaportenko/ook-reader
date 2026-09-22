# Feature — Chat panel

[← Milestone 6](../README.md)

The first UI of AI Chat: a drawer over the reader with a message list and an input, sending
through the `ChatProvider` from Phase 17 with the key Phase 18 put in the keychain. The
conversation lives in memory for the life of the open book. Everything that decides *what
the conversation is* is a plain struct under `cargo test`; only the drawer itself needs
`dx serve`.

| # | Phase | Status |
|---|---|---|
| 19 | [Chat panel](phase-19-chat-panel.md) | ✅ done — closed 2026-09-22 |
