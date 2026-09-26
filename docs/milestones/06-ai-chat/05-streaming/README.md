# Feature — Streaming replies

[← Milestone 6](../README.md)

The answer appears as Gemini writes it instead of all at once after a pause, and a *Stop*
button ends it midway and keeps what has arrived so far. This is the first time the app
reads an HTTP response *while it is still arriving*. It uses Server-Sent Events from
Gemini's `:streamGenerateContent?alt=sse`.

| # | Phase | Status |
|---|---|---|
| 22 | [Streaming replies](phase-22-streaming.md) | 🚧 in progress — opened 2026-09-25 |
