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

- **The library owns a copy of each book.** Import copies the chosen `.epub` into
  `data_dir()/books/`; the SQLite row stores that *managed* path. Open always reads the
  managed copy, so moving or deleting the user's original no longer breaks the library.
  Remove deletes the managed file *and* the row. Steps 1–6 first landed a path-only MVP;
  Steps 7–9 add managed import, correct re-import, and managed removal once the path-only
  trade-off became the real annoyance (the original was moved).
- **Identity for re-import = the absolute *source* path.** A `UNIQUE` constraint on
  `source_path` keeps re-importing the same picker path idempotent: the row id stays stable
  while its managed copy and metadata are refreshed. The managed `path` is what open/remove
  use. (A content hash or the EPUB's `dc:identifier` would dedupe the same bytes imported
  from different source paths — deferred.)
- **Covers are files beside the managed copy, served over a `covers` asset route.**
  Import writes the extracted cover to `data_dir()/books/<uuid>.cover.<ext>` and stores
  that path in a nullable `cover_path` column; the library list renders thumbnails through
  an app-level `use_asset_handler("covers", …)`. Chosen over a SQLite BLOB + data URLs to
  keep image bytes out of the DOM and the DB, at the price of a second managed file whose
  lifecycle reuses the Steps 7–9 machinery (import / re-import / remove / failed-import
  cleanup). Full trade-off discussion in the build log's Step 11 entry.
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
- [x] **Step 4 — Render the library list.** A Dioxus view listing `library.list()` rows
      (title + author). Eyeball. *(Cover thumbnails deferred — see scope note.)*
- [x] **Step 5 — Delete a book from the library.** `Library::remove(id)` drops the row (not
      the `.epub` on disk); a Remove control on each list row refreshes the shared books
      signal. `#[test]` + eyeball.
- [x] **Step 6 — Open a book → reader renders it.** Selecting a row opens *that* path; the
      reader drops `const BOOK` and keys the spine + asset handler off the choice. Exclusive
      library/reader screens with a Close control. End-to-end eyeball: import → list → open
      → page → close.
- [x] **Step 7 — Import into managed storage.** Copy the chosen file into
      `data_dir()/books/`; store source + managed paths. `#[test]` proves open still works
      after the source is deleted.
- [x] **Step 8 — Re-import replaces the managed copy.** Preserve the row id while replacing
      the owned bytes and refreshing metadata; repair a missing copy without leaking files.
- [x] **Step 9 — Remove the managed copy.** Delete the row first, then the owned file;
      tolerate an already-missing managed file. `#[test]` + eyeball.
- [x] **Step 10 — Cover image in `BookMeta`.** Extend `read_metadata` to pull the manifest
      cover (bytes + media type) into `BookMeta.cover: Option<CoverImage>`, best-effort so
      metadata stays infallible. `#[test]` against the bundled book. Domain only — no
      storage, no UI. *(Pulls forward the cover thread deferred at Step 4.)*
- [ ] **Step 11a — Persist the cover beside the managed copy.** Nullable `cover_path`
      column; import writes `<uuid>.cover.<ext>`, re-import replaces it, remove deletes
      it, a failed import cleans it up. `#[test]`s mirror the Steps 7–9 lifecycle tests.
- [ ] **Step 11b — Serve covers and show thumbnails.** App-level `covers` asset route
      with file-name sanitization; `<img>` thumbnails in the library list. `#[test]` for
      the sanitizer + `dx serve` eyeball.
- [ ] **Step 12 — Review & refactor** (mandatory phase-closer): review the library module
      boundary, tidy error handling, and delete the dead single-book `BOOK` scaffolding.

> **Related:** a July 2026 codebase review produced a parallel refactor backlog
> ([`review-2026-07-steps.md`](../review-2026-07-steps.md)). Two items interact with this
> phase: **R2** (pass `&Epub`, not paths) is best landed *before* Step 6 (open a book), and
> **R3** (a `thiserror` error type) pairs with Step 10's "tidy error handling."

## Known constraints

- **`rusqlite` bundled feature** — add `rusqlite = { version = "…", features = ["bundled"] }`
  so it compiles its own SQLite; no system dependency, works the same on every dev machine.
- **`FileData` differs by renderer** — on desktop `FileData::path()` is the selected real
  path used as the *source* for the copy into app storage. A browser cannot reveal an
  absolute path; the future web target will use the same file-input event but consume
  `FileData::read_bytes()` and write those bytes straight into the managed books dir.
- **Managed paths can still fail at open** — disk full mid-copy, a hand-deleted file under
  `books/`, etc. Those stay runtime errors at `Epub::open` (Step 6's open-status path).
  What the managed-storage steps remove is the common case: the user moved the *original*
  after import.
