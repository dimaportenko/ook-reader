# Milestone 4 — Multi-platform

[← Roadmap](../../roadmap.md)

**Goal:** run the same Rust/Dioxus codebase on **mobile** (iOS/Android) and the **web**
(WASM), beyond the desktop build from Milestones 1–3.

**Status:** ⏸ deferred (until the desktop reader works)

## Why deferred (not blocked)

Dioxus is cross-platform by design — one crate, platform chosen by Cargo feature +
`dx --platform`. Desktop comes first because it's the easiest path and has full
filesystem access. Mobile and web mostly need the platform-specific edges abstracted, not
a rewrite.

## Features

| # | Feature | Outcome | Status |
|---|---|---|---|
| 1 | Mobile (iOS/Android) | The reader running on a phone/tablet | ⏸ |
| 2 | Web (WASM) | The reader running in the browser | ⏸ |

## What each platform needs

- **Mobile:** `rustup target add` the iOS/Android targets; **Xcode** (iOS) or **Android
  Studio + SDK/NDK/CMake** (Android). `dx serve --platform mobile`, `dx bundle`.
  `rusqlite` bundled works on mobile.
- **Web:** `wasm32-unknown-unknown` target. The browser is sandboxed, so two things must
  be abstracted behind traits:
  - **Resource serving** — no custom protocol; serve EPUB resources as **blob URLs**
    instead of `use_asset_handler`.
  - **Persistence** — no native SQLite/filesystem; use **IndexedDB/localStorage** (or a
    server) instead of `rusqlite`.
  - File **import** uses a sandboxed file input rather than a native dialog.

## Design implication for earlier milestones

Keep **persistence** and **resource-serving** behind traits from the start (Milestone 2)
so the web backend is an add-on, not a refactor. See
[`RESEARCH.md`](../../../RESEARCH.md) §3.2, §4.

## Open question

How much of the persistence/asset layer must be trait-abstracted before this milestone,
vs. retrofitted? Decide during Milestone 2. See
[`RESEARCH.md`](../../../RESEARCH.md) open questions.
</content>
