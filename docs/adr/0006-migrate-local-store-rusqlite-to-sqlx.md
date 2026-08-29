# ADR-0006 — Migrate the local store from `rusqlite` to `sqlx`

**Status:** accepted · 2026-08-29 · supersedes the store half of
[ADR-0004](0004-local-store-rusqlite-with-libsql-sync-path.md)

## Context

[ADR-0005](0005-backendless-sync-google-drive-per-device-shards.md) commits the project to a
sync engine that talks HTTP to Google Drive and reads and writes the local database from
inside that work. That raises a concrete runtime question, because `rusqlite`'s `Connection`
is blocking and not `Sync`, and it reopens ADR-0004's rejection of `sqlx`.

Two findings framed the choice.

**ADR-0004's stated reason for rejecting `sqlx` no longer holds.** It reasoned that
"`sqlx`/SeaORM would drag an async runtime in just for this," and that "the current
`Cargo.toml` pulls in zero async machinery." Both were true then; neither is true now.
`cargo tree -e normal -i tokio` shows **tokio 1.52.3 already a normal dependency**, pulled in
by `dioxus-desktop` and `dioxus-asset-resolver`. The runtime is already linked into the
binary. The objection got weaker on its own.

**`sqlx` does not make SQLite async — it cannot.** SQLite is an in-process blocking C
library with no async API to expose, so
[`sqlx`'s SQLite driver runs each connection on a dedicated worker thread](https://deepwiki.com/launchbadge/sqlx/7.3-sqlite),
talking to it over a flume channel with a oneshot per response. That is precisely the
owning-thread actor one would otherwise hand-write in about forty lines. So the real choice
was never "async vs blocking database access" — it was "write that actor and understand it,
or take it from a crate that also brings other things."

The other things turn out to matter. This project has **no migration machinery at all**:
`src/db/mod.rs` migrates with bare `CREATE TABLE IF NOT EXISTS` and never sets
`user_version`. ADR-0005 requires the first real schema change the project has ever needed
(content hashes, device identity, tombstones, clock stamps), so migration machinery must
exist either way.

The iOS worry proved unfounded. `sqlx-sqlite` uses the **same `libsqlite3-sys` bundled C**
that `rusqlite` already cross-compiles to iOS on the first try; the cross-compilation failure
reports in the wild are Linux→Apple hosts, which is not this build.

## Decision

**Move the local store from `rusqlite` to `sqlx`, gradually, as a step inside Milestone 5 and
ahead of the sync work proper.**

What it buys:

- **A real migration system** — `sqlx::migrate!` with versioned `.sql` files — which the
  project needs *now* and has never had.
- **Compile-time-checked queries** (`query!`). A genuine Rust superpower, at the cost of a
  live `DATABASE_URL` or a checked-in `.sqlx` offline cache that has to be kept in sync.
- The owning-thread actor, maintained by someone else.

### The version conflict, and how the port is sequenced

`libsqlite3-sys` is a `links = "sqlite3"` crate, so Cargo permits **exactly one version** in
the dependency tree. The lock currently holds **0.38.1** (via `rusqlite` 0.40), and
`sqlx-sqlite` 0.9.0 requires **`>=0.30.1, <0.38.0`**. At current versions the two crates
**cannot coexist**: `cargo add sqlx` simply fails to resolve, and there is no side-by-side
port.

The way through is a deliberate, temporary downgrade:

| crate | version | `libsqlite3-sys` |
|---|---|---|
| `rusqlite` | 0.40 (today) | `^0.38` — conflicts |
| `rusqlite` | **0.39 (transition)** | `^0.37` — inside `sqlx`'s range |
| `sqlx-sqlite` | 0.9.0 | `>=0.30.1, <0.38.0` |

So: **downgrade `rusqlite` to 0.39**, add `sqlx`, port module by module (`books`,
`positions`, `settings`) with the tests green at every commit, then delete `rusqlite`. The
downgrade is undone by deletion rather than by another upgrade. The alternative — a big-bang
swap in one phase — was rejected because the tree would not compile mid-phase, leaving no
green checkpoint, which is against this repo's small-test-first-steps rule.

## Consequences

- **Good:** migration machinery arrives as a dependency feature rather than as hand-written
  code competing for attention with the sync engine, and it arrives exactly when the first
  schema change needs it.
- **Good:** every commit of the port is green and small, because both crates compile together
  during the transition.
- **Cost:** every existing query is rewritten. This is real work that buys no user-visible
  behaviour, and it sits in front of the milestone's actual content.
- **Cost:** the `.sqlx` offline query cache becomes a build artifact to maintain and keep in
  git, and a thing that can go stale — friction in NeoVim and in any cross-compile.
- **Cost / to relocate:** the `fallible_uint` feature comment in `Cargo.toml` exists to give
  `Locator::spine_index` a checked `usize` conversion instead of an `as usize` cast at the
  restore site. `sqlx` maps types differently, so that conversion needs a new home and the
  comment becomes obsolete. This is a named step, not a detail to discover mid-port.
- **Tension retained from ADR-0004:** its argument that the SQLite *file format* is
  decades-stable and that SQL is transferable knowledge is untouched — this changes the
  driver, not the store.

## References

- [ADR-0004](0004-local-store-rusqlite-with-libsql-sync-path.md) — the decision this supersedes.
- [ADR-0005](0005-backendless-sync-google-drive-per-device-shards.md) — the sync design that motivates it.
- [`sqlx` SQLite driver architecture](https://deepwiki.com/launchbadge/sqlx/7.3-sqlite) — worker thread + flume channel.
