# Milestone 4 — Multi-platform

[← Roadmap](../../roadmap.md)

**Goal:** run the same Rust/Dioxus codebase on **mobile** (iOS/Android) and the **web**
(WASM), beyond the desktop build from Milestones 1–3.

**Status:** 🚧 in progress — the mobile half is open.

> **The deferral expired rather than being jumped.** Its stated gate was "until the desktop
> reader works", and the desktop reader works: Milestone 2 shipped the reader, and Milestone 3
> closed Themes & Typography and ToC & Navigation. Web (WASM) stays ⏸ — it is the harder port
> and nothing is asking for it yet.

## Why it was deferred (not blocked)

Dioxus is cross-platform by design — one crate, platform chosen by Cargo feature +
`dx --platform`. Desktop comes first because it's the easiest path and has full
filesystem access. Mobile and web mostly need the platform-specific edges abstracted, not
a rewrite.

## Features

| # | Feature | Outcome | Status |
|---|---|---|---|
| 1 | [Mobile (iOS/Android)](01-mobile/README.md) | The reader running on a phone/tablet | 🚧 — [Phase 9](01-mobile/phase-9-ios-simulator.md) takes iOS/iPadOS on the simulator; Android is not in it |
| 2 | Web (WASM) | The reader running in the browser | ⏸ |

## What each platform needs

- **Mobile:** `rustup target add` the iOS/Android targets; **Xcode** (iOS) or **Android
  Studio + SDK/NDK/CMake** (Android). `dx serve --platform ios`, `dx bundle`.
  `rusqlite` bundled works on mobile — **confirmed**, not assumed: `libsqlite3-sys` compiles
  bundled SQLite against the iPhoneSimulator SDK without complaint. So does the rest of the
  dependency graph; the only iOS compile blocker in the whole crate was the five places that
  name `dioxus::desktop`, which does not exist under the `mobile` feature. See
  [Phase 9](01-mobile/phase-9-ios-simulator.md).
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

> **Half-answered by the iOS port: for mobile, none of it.** iOS keeps the custom protocol
> and keeps native SQLite, so `epub.rs`'s asset handler and the `db` layer crossed untouched.
> The question was really a *web* question all along — the browser is what has neither — and
> it stays open for that feature. What iOS does put pressure on is a layer the question never
> named: **file import**, which assumes a path the user can point at.
</content>
