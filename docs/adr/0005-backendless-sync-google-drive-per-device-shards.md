# ADR-0005 — Backend-less sync: Google Drive `appDataFolder` + per-device snapshot shards

**Status:** accepted · 2026-08-29 · supersedes the sync half of
[ADR-0004](0004-local-store-rusqlite-with-libsql-sync-path.md)

## Context

The goal, as originally stated, was "a backend app where I can log in to an account and sync
state between devices." [ADR-0004](0004-local-store-rusqlite-with-libsql-sync-path.md)
anticipated this and hedged toward libSQL/Turso, while explicitly owing "a focused research
pass **before** the sync milestone." This is that pass, and it changed the shape of the
answer: the requirement is *sync across my devices*, and a backend is one way to get it, not
the requirement itself.

Two facts, established by investigation rather than assumption, do most of the work.

**Drive has no optimistic concurrency.** The
[`files.update`](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/update)
reference documents no `If-Match`, no precondition parameter, no version guard. Two devices
writing one shared state file therefore silently clobber each other — the loser's reading
position vanishes with no error. This kills the obvious "one `state.json` in the cloud"
design, and it is the reason a naive file-store sync needs a server to arbitrate.

**But it only needs arbitration if two writers share a file.** If every device writes *only
its own* file and reads all of them, no file ever has two writers, and storage-layer
conflicts become structurally impossible. There is nothing left for a server to arbitrate.
The problem moves out of the storage layer and into a merge function — which is pure, local,
and unit-testable without a network.

That reframing is what removes the backend. What remains of "log in to an account" is
federated: connect a Google account.

## Decision

**Sync rides on the user's own Google Drive. There is no server.**

### Storage layout

Everything lives in Drive's **`appDataFolder`** — a per-app hidden folder. Both it and
`drive.file` are classified **Non-sensitive**, so neither triggers Google's app verification
(the restricted scopes `drive`, `drive.readonly`, `drive.metadata` do, and that is an annual
paid security assessment). One scope, no picker, nothing user-visible to break.

EPUB files are **local-primary, Drive-mirrored**: the book on disk is the source of truth,
Drive is the copy a new device pulls from. A fresh device shows the merged library and
downloads a book **on first open** — nothing is fetched that is never read, which matters on
a phone.

### The merge engine

- **Per-device snapshot shards.** `state-<device-id>.json`. Each device writes only its own
  shard and reads all of them. Not an operation log: a shard holds the device's *current*
  value for every book and position, each field clock-stamped. Size is bounded by library
  size rather than by history; a corrupt shard heals on the next write; and tombstones become
  an ordinary stamped field, which quietly disposes of tombstone garbage collection.
- **Merge = max clock per field**, across all shards.
- **Hybrid logical clock** for that ordering: wall-clock seconds for human meaning, plus a
  Lamport counter (`max(seen) + 1`) as the real comparison and tiebreak. Wall clock alone
  lets a device with a wrong clock stamp the future and pin a stale position permanently.
- **Book identity** is the **SHA-256 of the EPUB bytes**. `books.id` is a local
  autoincrement and `books.path` / `books.source_path` are local filesystem paths, so none of
  today's keys mean anything on another device. The OPF `dc:identifier` is recorded alongside
  but not trusted — it is frequently missing, frequently duplicated across unrelated books,
  and regenerated per build by some toolchains. It is stored so a future "same book,
  different file" merge has something to work with.
- **Device identity** is a uuid v4 generated on first run and stored locally, plus a
  human-readable name and a *forget this device* action that deletes its shard. A reinstall
  simply becomes a new device; the orphan shard is inert data the merge ignores until removed.
- **Deletes are tombstones** — a stamped `deleted` flag inside the snapshot. Without them a
  local delete is undone by the next merge, because the other device's shard still lists the
  book.
- **Payload:** reading positions and library membership. **Settings do not sync** — a font
  size and margin that read well on a 27" display are absurd on a phone, so the whole
  question is deferred rather than answered badly.

### Behaviour

Pull on launch and on foreground; push the local shard on a debounce after a position
changes. `changes.list` with a stored page token is the efficient "what changed since last
sync" feed; `changes.watch` push is deferred, since it only signals *that* something changed
and you poll anyway.

Conflicts resolve **silently by last-write-wins when the two positions are close, and prompt
when they diverge far** ("You were at Chapter 12 on iPhone — jump there?"). The annoying
failure is losing an evening's reading, not losing a paragraph.

Sync is **fully visible in the UI while the milestone is being built** — status, per-book
state, manual trigger, device list — and quiets to a "last synced" line plus surfaced errors
once it is trusted. Silent failure is the real enemy here, because an expired grant looks
exactly like nothing happening.

### Auth

- **Desktop:** loopback `127.0.0.1:port` + PKCE, Google's recommended installed-app flow.
  `yup-oauth2` ships it.
- **iOS:** loopback is deprecated for the iOS client type, and Google's native-app page says
  custom URI schemes are "no longer supported" for iOS, pointing instead at a Swift SDK with
  no Rust bindings. In practice the reversed-client-ID scheme still works — AppAuth and every
  non-Google-SDK client depends on it, and the
  [2023 custom-scheme restrictions](https://developers.googleblog.com/improving-user-safety-in-oauth-flows-through-new-oauth-custom-uri-scheme-restrictions/)
  restricted only new *Chrome extension* and *Android* clients, never iOS. We build it over
  `ASWebAuthenticationSession` via objc2, the same house style as the existing
  `UIDocumentPickerViewController` binding.
- **Refresh token in the OS keychain**, not the database — it is a long-lived credential that
  would otherwise sit in plaintext in a file that lands in every backup.
- **`client_secret` committed to the repo.** Google's own position is that an installed app's
  secret is not confidential; it ships inside every copy of the binary regardless. PKCE is
  what actually protects the flow. Committing keeps the build reproducible with no
  out-of-band setup.
- **The consent screen is published to Production.** While it sits in *Testing* with
  *External* user type, Google revokes refresh tokens after exactly **7 days** — sync dies
  weekly with `invalid_grant`. `Internal` needs Google Workspace, which a `gmail.com` account
  does not have. With non-sensitive scopes only, publishing needs basic verification rather
  than the paid assessment.

### Boundary

A small owned **`RemoteStore` trait** (list / get / put / delete) with Drive as the only
implementation. The point is *not* provider-swapping, which nothing is asking for. It is that
the merge engine gets tested against an in-memory fake with no network and no OAuth — which
is most of this milestone's test suite. `opendal` was considered and rejected: it would buy
gdrive/dropbox/webdav/s3 portability we do not need, at the price of a large abstraction we
do not control, and it does not solve OAuth anyway.

## Consequences

- **Good:** no hosting, no deploy, no server DB, no accounts table, no password storage, no
  uptime obligation, and no user data held on anyone's behalf. The entire operational surface
  of "a backend app" disappears while the actual requirement is met.
- **Good:** the interesting work is the part worth learning. Shard-and-merge, logical clocks,
  tombstones and idempotent replay are real distributed-state problems; writing HTTP handlers
  is not. And because merging is a pure function over shards, nearly all of it is testable
  with `#[test]` and no network — which is exactly the repo's small-test-first-steps rule.
- **Good:** `tokio` is already a normal dependency (via `dioxus-desktop` and
  `dioxus-asset-resolver`), and `serde_json` already too, so the shard format and the HTTP
  client are less of a dependency step than expected. `reqwest` is genuinely new.
- **Cost, accepted knowingly:** putting the book library in `appDataFolder` means that
  [revoking the app's access](https://developers.google.com/workspace/drive/api/guides/appdata)
  in Google Account settings — or *Delete hidden app data* in Drive settings — **permanently
  destroys the cloud copy**. Files there skip Trash entirely: no 30-day grace, no undo. These
  are deliberate actions, but they are the sort of thing people do while tidying an account
  security list, with nothing warning them a library is inside. Local copies survive; a fresh
  device would bootstrap from nothing. The alternative — books under `drive.file` in a visible
  folder that survives revocation — was offered and declined in favour of one scope and no
  picker.
- **Cost:** the iOS auth path is officially discouraged by Google. It works today and is what
  every non-SDK client does, but it could tighten, and the fallback (their Swift SDK) has no
  Rust bindings. This is a standing risk on the milestone, not a solved problem.
- **Cost:** no server-side merge, no cross-user features, and no push to a device that is
  currently offline. All acceptable; none were wanted.
- **Deferred:** the web client (no local DB in wasm at all — it would be a thin
  server-calling client, a second architecture beside this one, and Milestone 4's problem);
  `changes.watch` push; settings sync; real multi-user accounts.
- **Unconfirmed:** whether `appDataFolder` contents count against the user's 15 GB quota.
  Google's appdata guide does not say. Worth establishing before mirroring a large library.
- **Unresolved:** Dioxus 0.7 documents no app-lifecycle/foreground hook, so "sync on
  foreground" on iOS likely means observing `UIApplicationDidBecomeActiveNotification`
  through objc2. There is a trivial fallback — sync on launch and on opening a book — so this
  is a phase-level detail, not a design risk.

## What this overturns in ADR-0004

ADR-0004 named libSQL/Turso "the documented upgrade path when sync becomes a real milestone,"
and reasoned that staying SQLite-shaped kept it "mostly a connection/API swap." That path is
**dropped**. It was a hedge written before the requirement was understood: it is the fastest
route to working sync and the worst for learning, because it hides change tracking, conflict
resolution and offline reconciliation — precisely the content of this milestone. It also fits
a shared multi-tenant schema awkwardly, and it would have required a server-side account
system that this design does not need at all.

ADR-0004's local-store choice is separately revisited in
[ADR-0006](0006-migrate-local-store-rusqlite-to-sqlx.md).

## References

- [ADR-0004](0004-local-store-rusqlite-with-libsql-sync-path.md) — the decision this supersedes.
- [`../glossary.md`](../glossary.md) § Sync — shard, hybrid logical clock, tombstone,
  content hash, `RemoteStore`.
- [Drive `files.update` reference](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/update) — no conditional-request support.
- [Drive application data folder](https://developers.google.com/workspace/drive/api/guides/appdata) — deleted on disconnect; no Trash.
- [Choose Drive API scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth) — `drive.file` and `drive.appdata` are Non-sensitive.
- [OAuth 2.0 for iOS & Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app) — loopback for desktop; iOS caveats.
- [Manage changes](https://developers.google.com/workspace/drive/api/guides/manage-changes) — `changes.list` page tokens, `changes.watch`.
