# Phase 6 — Library & Import

[← Feature: Library & Import](README.md) · **Status:** 🚧 in progress ·
build log: [`phase-6-library-steps.md`](phase-6-library-steps.md)

## Goal

Stop hard-coding one book. Let the user **add `.epub` files**, **see them in a list**
(title + author, cover later), and **click one to open it** in the Phase 3–5 reader. This is
Feature 2 of Milestone 2 and clears two of the milestone's three exit criteria ("import an
`.epub` and see it in a library list", "open it and turn pages").

## The crux

Today the reader is wired to a single book by a **compile-time `const BOOK`** —
`Epub::open(BOOK)`, `load_spine(BOOK)`, `use_register_asset_handler` all thread that one
constant (`main.rs:30`, `62`, `107`). A library breaks two assumptions at once:

1. **Books live outside the binary, at runtime paths the user chose.** That means a native
   file input and a durable place to remember what was added — an on-disk **`rusqlite`**
   store located via **`directories::ProjectDirs::data_dir()`**. Dioxus's
   `<input type="file">` provides the picker abstraction; on desktop it opens the native
   dialog internally. This is the
   genuinely new territory: SQLite, app data dirs, first persistence in the project.
2. **The reader must be *told which* book to open** instead of reading a `const`. Opening an
   arbitrary path ripples through `App`, `Reader`, `use_register_asset_handler`, and
   `load_spine` — the single-book `const` has to come out.

The insight that keeps this small is the same one that shaped the EPUB layer: **build the
library *domain* as plain, testable Rust first** — a `BookMeta` you extract with a pure
function, a `Library` store you can round-trip in a `#[test]` against a temp database — and
only *then* wire it to a file dialog and a Dioxus list. **Data first, UI last.** The dialog
and the list are thin glue over a core that's already tested.

## Design decisions (recorded up front)

- **Store the book's *path*, not a copy of the file.** The library row references the
  `.epub` where it already sits on disk; we don't copy it into app storage. Simpler schema,
  no file I/O in import, and good enough for a desktop MVP. *Trade-off:* if the user moves or
  deletes the original, the row dangles — we detect that lazily at open time. Copying files
  into `data_dir()` (robust to moves) is a deliberate later hardening, not this phase.
- **Identity = the absolute path.** A `UNIQUE` constraint on `path` makes re-importing the
  same file idempotent. (A content hash or the EPUB's `dc:identifier` would dedupe copies at
  different paths — deferred; the path is the pragmatic key.)
- **`rusqlite`, per [`RESEARCH.md`](../../../../RESEARCH.md) §4** — already the recorded
  choice; bundled feature so there's no system SQLite dependency. Full reasoning vs the
  alternatives (redb, a JSON file), the libSQL/Turso sync path, and why the WASM/tokio
  constraint doesn't affect this step:
  [ADR-0004](../../../adr/0004-local-store-rusqlite-with-libsql-sync-path.md).

## Planned steps

*(smallest-first; the last step is the mandatory review-and-refactor pass)*

- [x] **Step 1 — Extract a book's metadata.** Pure `epub::read_metadata(path) -> BookMeta
      { title, author }` via `rbook`; `#[test]` against the bundled book. No store, no UI.
- [x] **Step 2 — A persistent library store.** `rusqlite`-backed `Library` with `add` and
      `list`; schema + row type (`Book`); round-trip `#[test]` against a temp/in-memory DB.
- [x] **Step 3 — Import via Dioxus file input.** `<input type="file" accept=".epub">` →
      desktop `FileData::path()` → `read_metadata` → `library.add`; app locates the real DB
      via `directories::ProjectDirs`. Eyeball.
- [ ] **Step 4 — Render the library list.** A Dioxus view listing `library.list()` rows
      (title + author). Eyeball. *(Cover thumbnails deferred — see scope note.)*
- [ ] **Step 5 — Open a book → reader renders it.** Selecting a row opens *that* path; the
      reader drops `const BOOK` and keys the spine + asset handler off the choice. End-to-end
      eyeball: import → list → open → page.
- [ ] **Step 6 — Review & refactor** (mandatory phase-closer): review the library module
      boundary, tidy error handling, and delete the dead single-book `BOOK` scaffolding.

> **Related:** a July 2026 codebase review produced a parallel refactor backlog
> ([`review-2026-07-steps.md`](../review-2026-07-steps.md)). Two items interact with this
> phase: **R2** (pass `&Epub`, not paths) is best landed *before* Step 5, and **R3** (a
> `thiserror` error type) pairs with Step 6's "tidy error handling."

## Known constraints

- **`rusqlite` bundled feature** — add `rusqlite = { version = "…", features = ["bundled"] }`
  so it compiles its own SQLite; no system dependency, works the same on every dev machine.
- **`FileData` differs by renderer** — on desktop `FileData::path()` is the selected real
  path, which fits this phase's path-based library. A browser cannot reveal an absolute
  path; the future web target will use the same file-input event but consume
  `FileData::read_bytes()` and store/copy those bytes instead.
- **Dangling paths are a runtime error, not a schema one** — a stored path whose file has
  since moved fails at `Epub::open` time. Handle it where the book is opened (Step 5), not in
  the store.
