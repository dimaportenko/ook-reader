# ADR-0004 — Local store on `rusqlite` now, with a libSQL/Turso sync path later

**Status:** accepted · 2026-07-05

## Context

Phase 6 (Library & Import) introduces the project's **first persistence**: a durable place
to remember imported books (path, title, author) that grows into positions, bookmarks, and
highlights over the milestone. `RESEARCH.md §4` already recorded `rusqlite` (bundled SQLite)
as the choice; this ADR captures the reasoning behind that pick against the alternatives, and
folds in two questions raised in review that the research note did not fully answer:

1. **A future backend + cross-device sync.** The author wants, eventually, a backend service
   that syncs a user's library and reading data across devices — not just a local file. Does
   that change the local-store choice today?
2. **The WASM/tokio constraint.** The Dioxus fullstack docs warn that `tokio` (and native
   deps generally) cannot compile to `wasm32-unknown-unknown`. Since the async sync clients
   (`libsql`) pull in tokio, does that constraint affect us?

The store is small in every dimension that matters: tens–hundreds of books, one user, one
process, effectively no concurrency. The deciding axes are **sync vs async**, **C dep vs pure
Rust**, **SQL vs KV vs flat file**, **how it survives the eventual WASM/web jump**, and
**learning value** (this repo optimizes for understanding).

## Decision

**Use `rusqlite` with the `bundled` feature as the local store**, built behind a testable
`Library` boundary (a plain Rust type round-tripped in a `#[test]` against a temp DB), and
treat **libSQL/Turso as the documented upgrade path** when sync becomes a real milestone.

Rationale, by axis:

- **Synchronous, no runtime.** The current `Cargo.toml` pulls in zero async machinery.
  `rusqlite` is blocking and drops into Dioxus's mostly-sync desktop backend with no tokio
  tax. `sqlx`/SeaORM would drag an async runtime in just for this.
- **`bundled` statically compiles SQLite in** — no "is SQLite installed?" problem, the
  documented way to avoid system-lib grief on desktop and, later, iOS/Android.
- **It grows with the milestone.** Positions/bookmarks/highlights are just more tables with
  foreign keys; FTS5 gives full-text search over book contents later if wanted. A flat file
  can't do that without reinventing indexing.
- **Highest learning value + battle-tested.** SQL is transferable knowledge; the SQLite file
  format is decades-stable.

**The sync goal reinforces this choice rather than changing it:** the whole Turso ecosystem
is SQLite-shaped, so staying in the SQLite family keeps libSQL a near-drop-in later. It is the
reason we do **not** jump to a pure-Rust KV store (redb/native_db) — that would be a genuine
rewrite to gain sync, whereas rusqlite → libSQL is mostly a connection/API swap behind the
`Library` boundary.

### The WASM/tokio constraint does not affect Step 2 — and resolves in our favor

The key distinction: "tokio doesn't support WASM" means the **`wasm32-unknown-unknown`
browser *client*** specifically. Everything else is a native target where tokio and native
SQLite compile fine:

| Target | WASM? | tokio | native SQLite |
|---|---|---|---|
| Desktop (current default) | no — `target=host` | ✅ | ✅ |
| Mobile iOS/Android | no — native | ✅ | ✅ |
| Server binary | no — native | ✅ | ✅ |
| Web client | **yes — `wasm32`** | ❌ | ❌ (no C, no filesystem) |

Two consequences:

- **Desktop feels web-ish but isn't WASM.** Dioxus desktop uses a webview only to *render*;
  the Rust logic is a native binary. So `rusqlite` (and even an async `libsql`) compile fine
  on desktop. The doc's warning targets the wasm *client* bundle, which this project does not
  build today. **Step 2 is unaffected.**
- **The browser limit was never tokio-specific.** `rusqlite` can't run in the browser either
  (C + filesystem). No native DB of any kind runs in `wasm32`. tokio's incompatibility is one
  more symptom of the same rule, not a new problem.

The Dioxus fullstack server/client split *is* the resolution, and it happens to be the sync
architecture we want:

- **Server binary** (native → tokio + libSQL/rusqlite fine): source-of-truth DB, gated behind
  the `server` Cargo feature (`optional = true` deps) so it never enters the wasm bundle.
- **Web client** (wasm): **no local DB, no tokio** — calls `#[server]` functions over the
  network. Browsers are online by nature, so "sync" for web is just "talk to the server."
- **Desktop / mobile clients** (native): keep a **local** store — `rusqlite` now, or a libSQL
  embedded replica later — that works offline and syncs to that same server.

So async (tokio/libsql) only ever appears on **native** targets where it's fine; the one place
it would be impossible (the browser) is exactly the place that runs no DB. The
`#[cfg(feature = "server")]` / `optional = true` discipline from the fullstack docs is the
concrete tool that keeps tokio, libsql, and rusqlite out of the wasm build.

## Consequences

- **Good:** Step 2 stays the simplest possible first-persistence lesson — sync SQL, no async,
  a `#[test]` round-trip — with no premature sync/WASM complexity.
- **Good:** the `Library` boundary makes later moves bounded: rusqlite → libSQL (embedded
  replica) for native sync, and a "call `#[server]` functions" impl for the web client, with
  no changes to the reader.
- **Good:** the sync requirement is what rules out redb/native_db, so we avoid a future
  rewrite by staying SQLite-shaped now.
- **Cost / deferred:** true offline-write, bidirectional sync (libSQL embedded replicas, and
  the maturity of self-hosting `sqld` vs Turso Cloud) is **unverified** and moves quickly. A
  focused research pass is owed *before* the sync milestone — not now. The Rust rewrite of
  SQLite ("Turso Database" / limbo) is too early to build on.
- **Tension to watch:** the moment the project first adds `dioxus/fullstack` or a `server`
  feature, every native-only dep (tokio, rusqlite/libsql, filesystem) must be made
  `optional` and feature-gated, or the wasm client build breaks with cryptic errors. Capture
  that discipline in the web milestone's plan.
- **Escape hatch retained:** a `serde_json` index file remains the fallback if even rusqlite
  proves heavier than the MVP needs — rejected here only because bookmarks/positions are
  coming this milestone and want real tables.

## References

- `RESEARCH.md §4` (persistence) — the original recorded choice.
- [`../milestones/02-basic-reader/02-library/phase-6-library.md`](../milestones/02-basic-reader/02-library/phase-6-library.md)
  — the phase this decision serves.
- Dioxus 0.7 fullstack docs, *Project Setup* → "Adding Server Only Dependencies" — the
  `optional`/`#[cfg(feature = "server")]` pattern for keeping tokio out of the wasm bundle.
