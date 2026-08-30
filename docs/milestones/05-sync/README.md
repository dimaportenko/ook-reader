# Milestone 5 — Sync

[← Roadmap](../../roadmap.md)

**Goal:** read on the Mac, pick up the iPhone, and be on the same page — with the reading
position, the library and its deletions converging across devices, and **no server anywhere**.

**Status:** ⬜ planned — designed 2026-08-29 in a grilling session; decisions recorded in
[ADR-0005](../../adr/0005-backendless-sync-google-drive-per-device-shards.md) and
[ADR-0006](../../adr/0006-migrate-local-store-rusqlite-to-sqlx.md). No phase open yet.

The gate discussed while planning this — closing the iOS port first — is **already
satisfied**: [Phase 9](../04-multiplatform/01-mobile/phase-9-ios-simulator.md) closed on
`54e2fbd`. Nothing else is open in front of this milestone: Milestone 3 has no phase in
progress, so this one is free to start whenever it is picked up.

## The idea in one paragraph

The request was "a backend app where I can log in and sync between devices." The
investigation turned that inside out. Google Drive offers **no optimistic concurrency** —
`files.update` has no `If-Match`, no precondition — so two devices writing one shared state
file silently clobber each other, and arbitrating that is exactly what a server would be for.
But arbitration is only needed when two writers share a file. Give **every device its own
file** and conflicts become structurally impossible; the problem leaves the storage layer and
becomes a **merge function** — pure, local, and testable with no network at all. That is the
whole design, and it is why there is no backend. "Log in to an account" survives as
*connect your Google account*.

The learning here is distributed state — shards, logical clocks, tombstones, idempotent
replay — not HTTP handlers. Which is the better trade for this project, and conveniently
most of it is `#[test]`-able offline.

## Phases

| # | Phase | Outcome | Status |
|---|---|---|---|
| 11 | Store migration — `rusqlite` → `sqlx` | Versioned migrations exist for the first time; every query ported, tests green at each commit | ⬜ |
| 12 | Identity & schema | Content hashes backfilled, device identity minted, HLC stamps on every synced field | ⬜ |
| 13 | The merge engine | Shard model + merge + tombstones, against an in-memory `RemoteStore` — no network, no OAuth | ⬜ |
| 14 | Google auth | Desktop loopback + PKCE first; then the iOS flow over `ASWebAuthenticationSession` via objc2 | ⬜ |
| 15 | Drive `RemoteStore` + wiring | The real store behind the trait; cadence, conflict prompt, visible sync UI | ⬜ |
| 16 | Book file mirroring | EPUBs mirrored to `appDataFolder`; a new device downloads on first open | ⬜ |

> Phase files are written as each phase is picked up, per this repo's convention — the table
> is the plan, not a substitute for the steps.

**Why this order.** Phase 13 is the heart of the milestone and it needs *nothing* — no
account, no network, no Drive. Putting it before auth means the hard thinking happens against
unit tests rather than against an OAuth redirect that is failing for unrelated reasons. Phases
11 and 12 come first only because the merge engine has nothing to merge until books have a
device-independent identity, and identity is the first real schema change this project has
ever needed. Phase 16 is last because file transfer is a separate problem from state
convergence, and the reader is already useful once state syncs.

## The design, in short

Full reasoning lives in
[ADR-0005](../../adr/0005-backendless-sync-google-drive-per-device-shards.md); vocabulary in
[`glossary.md`](../../glossary.md) § Sync.

- **Store:** Drive `appDataFolder` — one Non-sensitive scope, no Google verification, hidden
  from the user.
- **State:** per-device **snapshot shards**, `state-<device-id>.json`; one writer each.
  Merge takes the highest **hybrid logical clock** per field.
- **Book identity:** SHA-256 of the EPUB bytes. `dc:identifier` recorded but not trusted.
- **Deletes:** tombstones — a stamped flag in the snapshot, so a local delete survives the
  next merge.
- **Payload:** reading positions + library membership. **Settings deliberately do not sync**
  — a font size that reads well on a 27" display is absurd on a phone.
- **Books:** local-primary, Drive-mirrored; downloaded on first open.
- **Cadence:** pull on launch/foreground, push debounced after a position changes.
- **Conflicts:** silent last-write-wins when close; prompt on a large divergence.
- **Auth:** desktop loopback + PKCE (`yup-oauth2`); iOS reversed-client-ID over
  `ASWebAuthenticationSession`. Refresh token in the OS keychain; `client_secret` committed,
  since Google states an installed app's secret is not confidential.
- **Boundary:** an owned four-method `RemoteStore` trait, for testability rather than for
  provider-swapping.

## Known risks and open questions

- **The consent screen must be published to Production.** In *Testing* with *External* user
  type, Google revokes refresh tokens after exactly **7 days** and sync dies with
  `invalid_grant`. `Internal` requires Google Workspace, which a `gmail.com` account lacks.
- **`appDataFolder` is destroyed by revoking access** — permanently, skipping Trash. This is
  an accepted risk (see ADR-0005), and it means a fresh device would bootstrap from nothing.
  Local copies survive.
- **The iOS auth path is officially discouraged.** Google's native-app page says custom URI
  schemes are "no longer supported" for iOS and points at a Swift SDK with no Rust bindings.
  The reversed-client-ID scheme works today and is what AppAuth does, but it could tighten.
- **`libsqlite3-sys` version conflict.** `rusqlite` 0.40 pins `^0.38`; `sqlx-sqlite` 0.9
  needs `<0.38`; Cargo allows one version of a `links` crate. Phase 11 downgrades `rusqlite`
  to **0.39** (`^0.37`) so both compile during the port, then deletes it.
- **`fallible_uint` needs a new home.** The `Cargo.toml` feature exists to give
  `Locator::spine_index` a checked `usize` conversion; `sqlx` maps types differently.
- **No Dioxus lifecycle hook.** Dioxus 0.7 documents no foreground/resume event, so "sync on
  foreground" on iOS likely means observing `UIApplicationDidBecomeActiveNotification` via
  objc2. Fallback: sync on launch and on opening a book.
- **Unconfirmed:** whether `appDataFolder` counts against the user's 15 GB quota.

## Deliberately out of scope

The **web client** (wasm runs no local database at all — it would be a thin server-calling
client, a second architecture; Milestone 4's problem), `changes.watch` push notifications,
settings sync, and real multi-user accounts.
