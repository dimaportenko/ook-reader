# Phase 6 — Library & Import — build log

[← Phase doc](phase-6-library.md)

Per-step test → minimal code → why, appended newest-last. The
[phase doc](phase-6-library.md)'s "Planned steps" checklist is the high-level index; this
file is the detail and the build log.

## The crux

The reader is wired to one book by a **compile-time `const BOOK`**. A library breaks two
assumptions: books live at **runtime paths the user chose** (→ a file dialog + a durable
store), and the reader must be **told which book** to open (→ the `const` comes out and
ripples through `App`/`Reader`/the asset handler). The move that keeps it small: build the
library **domain** as plain, testable Rust first — a `BookMeta` you extract with a pure
function, a `Library` store you round-trip in a `#[test]` — then wrap it in a dialog and a
list. **Data first, UI last**, exactly like the EPUB layer.

## Step plan

1. **Extract a book's metadata** — pure `epub::read_metadata(path) -> BookMeta { title,
   author }` via `rbook`; `#[test]` against the bundled book. *(done)*
2. **A persistent library store** — `rusqlite` `Library` with `add`/`list` and a `Book` row
   type; round-trip `#[test]` against a temp/in-memory DB. *(done)*
3. **Import via Dioxus file input** — `<input type="file" accept=".epub">` → desktop
   `FileData::path()` → `read_metadata` → `library.add`; real DB path via
   `directories::ProjectDirs`. Eyeball. *(done)*
4. **Render the library list** — Dioxus view over `library.list()` (title + author). Eyeball.
   *(done)*
5. **Delete a book from the library** — `Library::remove(id)` + a Remove control per row that
   refreshes the shared books signal. `#[test]` + eyeball. *(done)*
6. **Open a book → reader renders it** — the row selection drives the reader; `const BOOK`
   comes out. End-to-end eyeball. *(done)*
7. **Review & refactor** — tidy module boundaries and errors, then delete the single-book
   scaffolding. *(pending)*

---

## Step 1 — extract a book's metadata from its path

The smallest possible start: no store, no dialog, no UI — just "given an `.epub` path, what's
its title and author?" It's a pure `rbook` read, it's the field the list will show first, and
it's `#[test]`-able against the book already bundled for the Phase 3–5 tests.

**Runnable check.** A `#[test]` in `epub.rs` (the module that already owns every `rbook`
operation), opening `crate::BOOK` and asserting on the extracted fields — mirrors
`loads_spine_in_reading_order`:

```rust
#[test]
fn reads_title_and_author_from_metadata() {
    let meta = read_metadata(crate::BOOK).expect("bundled epub metadata should read");

    // The bundled book is "The Adventures of Sherlock Holmes" by Arthur Conan Doyle.
    assert!(
        meta.title.contains("Sherlock Holmes"),
        "expected the book's title, got {:?}",
        meta.title,
    );
    assert!(
        meta.author.as_deref().unwrap_or("").contains("Doyle"),
        "expected Conan Doyle as the author, got {:?}",
        meta.author,
    );
}
```

Assert on the *values*, not just `Ok` — a metadata read that returns an empty title is a bug
the `.is_ok()` check would wave through, the same reason `reads_cover_image_bytes` asserts on
the magic bytes rather than just that the read succeeded.

**Minimal implementation.** A small record plus one pure function, in `src/epub.rs` next to
`load_spine`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookMeta {
    pub(crate) title: String,
    pub(crate) author: Option<String>, // not every EPUB declares a creator
}

pub(crate) fn read_metadata(path: &str) -> Result<BookMeta, Box<dyn std::error::Error>> {
    let epub = Epub::open(path)?;
    let metadata = epub.metadata();

    let title = metadata
        .title()
        .map(|t| t.value().to_string())
        .unwrap_or_else(|| "Untitled".to_string());

    let author = metadata
        .creators()
        .next()
        .map(|c| c.value().to_string());

    Ok(BookMeta { title, author })
}
```

**Why it works.** `Epub::metadata()` returns an `EpubMetadata` whose `.title()` is
`Option<EpubTitle>` — `Option` because the Dublin Core `dc:title` element is *technically*
optional in the spec, so `rbook` refuses to invent one. `.map(|t| t.value().to_string())`
turns "a title if present" into a `String`, and `.unwrap_or_else(|| "Untitled".into())` gives
the list something to show for a malformed book instead of blowing up — a total function over
"maybe there's a title." Author is `.creators().next()`: a book can list several creators
(author, illustrator, translator…), and the first is the one to show; keeping it
`Option<String>` rather than defaulting is honest — an absent author should read as absent,
not as `"Unknown"` masquerading as data. `?` on `Epub::open` propagates a bad path or corrupt
zip to the caller as the boxed error, so the happy path stays the three lines you read top to
bottom. `.title()`, `.creators()`, and `.value()` are all **inherent** methods on the
concrete `rbook` types, so the existing `use rbook::Epub` is the only import — no trait to
bring into scope.

**Scope note.**
- **No `path` field on `BookMeta` yet.** `read_metadata` is handed the path by the caller; the
  path only needs to be *stored* alongside the metadata in Step 2, where the `Book` row type
  (`id`, `path`, `title`, `author`) is defined. Keeping `BookMeta` path-free here means the
  extractor doesn't care where the bytes came from.
- **No cover here.** Cover *bytes* already have a proven read (`reads_cover_image_bytes` in
  Phase 3); surfacing a thumbnail is a Step 4 concern (image serving), not metadata.
- **Re-opens the file just to read metadata.** `read_metadata` opens the EPUB, and Step 6 will
  open it *again* to read the spine. Redundant but cheap, and it keeps import (which only needs
  title/author) decoupled from rendering (which needs the spine). Fold them only if it ever
  shows up as slow.

> **Status:** done — committed in `623a997` (15 tests green). Landed in `src/epub.rs`
> (not a separate file), keeping every `rbook` read in one module as the crux describes.

---

## Step 2 — a persistent library store (rusqlite, `add` + `list`)

Step 1 gave us a `BookMeta` extracted from a path. Step 2 gives that metadata somewhere to
**live across launches**: the first persistence in the project. Still no dialog, no UI — just
"put a book in, get the list back out," round-tripped in a `#[test]`. This is the step that
introduces `rusqlite`, a schema, and a `Book` row type. (Store choice + the sync/WASM
reasoning: [ADR-0004](../../../adr/0004-local-store-rusqlite-with-libsql-sync-path.md).)

**The crux.** A `Connection` is a *handle to a database that outlives any single call* — so
the store is a struct that **owns** the connection, and every `add`/`list` borrows it. Two
ideas make the step small: (1) **schema-as-init** — a `CREATE TABLE IF NOT EXISTS` run once
when the store opens means every later call can assume the table exists; (2) **`Option` ↔ SQL
`NULL`** — rusqlite maps a `TEXT` column that may be NULL straight to `Option<String>` on
`row.get`, so the "author may be absent" honesty from Step 1 survives the round-trip for free.
The test to aim at is a *round-trip*: what you `add` is exactly what `list` returns.

**Runnable check.** A `#[test]` (run with `cargo test`) against an **in-memory** DB — no temp
files, no filesystem, just `Connection::open_in_memory()`. It adds two books (one with an
author, one without) and asserts they come back intact, including the NULL-author case:

```rust
#[test]
fn add_then_list_round_trips_books() {
    let library = Library::open_in_memory().expect("in-memory db opens");

    let holmes = BookMeta {
        title: "The Adventures of Sherlock Holmes".to_string(),
        author: Some("Arthur Conan Doyle".to_string()),
    };
    let added = library
        .add("/books/holmes.epub", &holmes)
        .expect("add succeeds");

    // The row comes back with a DB-assigned id and the fields we put in.
    assert_eq!(added.id, 1);
    assert_eq!(added.path, "/books/holmes.epub");

    // A book with no declared author must round-trip as NULL ↔ None.
    let beowulf = BookMeta { title: "Beowulf".to_string(), author: None };
    library.add("/books/beowulf.epub", &beowulf).expect("add anon");

    let books = library.list().expect("list succeeds");
    assert_eq!(books.len(), 2);

    // ORDER BY title: "Beowulf" before "The Adventures…".
    assert_eq!(books[0].title, "Beowulf");
    assert_eq!(books[0].author, None);
    assert_eq!(books[1], added);
    assert_eq!(books[1].author.as_deref(), Some("Arthur Conan Doyle"));
}
```

Assert on the *values and the id*, not just that `add`/`list` returned `Ok` — a store that
silently drops the author or hands back the wrong row is exactly the bug a `.is_ok()` check
waves through (the same reasoning as Step 1's value assertions).

**Minimal implementation.** First the dependency — `bundled` statically compiles SQLite in
(ADR-0004), so there's no system-SQLite requirement:

```toml
# Cargo.toml — cargo add rusqlite --features bundled
rusqlite = { version = "0.40", features = ["bundled"] }
```

> Heads-up: the *first* build with `bundled` compiles SQLite from C source — expect a slow
> one-time `cargo build`, not a hang.

Then a row type and the store. They live in a dedicated `src/library.rs` module from the
start, keeping persistence separate from Dioxus wiring:

```rust
use rusqlite::{params, Connection};

use crate::epub::BookMeta; // Step 1's type is the input to `add`

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Book {
    pub(crate) id: i64,           // DB-assigned rowid
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) author: Option<String>,
}

pub(crate) struct Library {
    conn: Connection, // owned — the store IS the open connection
}

impl Library {
    pub(crate) fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    // Shared by open_in_memory (now) and a file-backed open() in Step 3.
    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                 id     INTEGER PRIMARY KEY,
                 path   TEXT NOT NULL UNIQUE,
                 title  TEXT NOT NULL,
                 author TEXT
             )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub(crate) fn add(&self, path: &str, meta: &BookMeta) -> rusqlite::Result<Book> {
        self.conn.execute(
            "INSERT INTO books (path, title, author) VALUES (?1, ?2, ?3)",
            params![path, meta.title, meta.author],
        )?;
        Ok(Book {
            id: self.conn.last_insert_rowid(),
            path: path.to_string(),
            title: meta.title.clone(),
            author: meta.author.clone(),
        })
    }

    pub(crate) fn list(&self) -> rusqlite::Result<Vec<Book>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, title, author FROM books ORDER BY title")?;
        let rows = stmt.query_map([], |row| {
            Ok(Book {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
            })
        })?;
        rows.collect() // Iterator<Item = rusqlite::Result<Book>> → Result<Vec<Book>>
    }
}
```

**Why it works.**
- **`Library` owns the `Connection`.** A DB handle has to outlive individual calls, so it's a
  struct field, and `add`/`list` take `&self` — they *borrow* the open connection rather than
  reopening it each time. This is the persistence analogue of "state that outlives the
  function body."
- **`rusqlite::Result<T>` is `Result<T, rusqlite::Error>`.** Using the crate's own error type
  (rather than Step 1's `Box<dyn Error>`) is more precise here — a caller can `match` on a
  real SQLite error — and `?` threads any failure (bad SQL, constraint violation) straight
  out. `open_in_memory`, `init`, `add`, and `list` all just `?`-propagate.
- **`params![...]` binds values to `?1 ?2 ?3`.** Parameter binding — never string
  interpolation — is what makes this injection-safe and lets rusqlite convert Rust types to
  SQLite types. Crucially, `Option<String>` binds to `NULL` when `None`; on the way back,
  `row.get(3)` reads that `NULL` into `Option<String>` as `None`. That symmetry is why the
  Beowulf assertion passes without any special-casing.
- **`last_insert_rowid()`** returns the `INTEGER PRIMARY KEY` SQLite just assigned, so `add`
  can hand back a fully-formed `Book` (with its id) instead of making the caller re-query.
- **`query_map(...).collect()`** turns rows into an iterator of `Result<Book>`; collecting
  into `rusqlite::Result<Vec<Book>>` short-circuits on the first row that fails to decode —
  one clean line instead of a manual `while let` loop pushing into a `Vec`.
- **`schema-as-init`** with `IF NOT EXISTS` makes opening idempotent: every `add`/`list`
  afterward can assume the table is there, so there's no "is it set up yet?" branch anywhere.

**Scope note.**
- **In-memory only.** `open_in_memory()` is the *test* seam; **Step 3** adds a file-backed
  `open(path)` (sharing `init`) and locates the real DB via `directories::ProjectDirs`.
- **No idempotent re-import yet.** The `path` column is `UNIQUE`, but `add` is a plain
  `INSERT` — adding the *same* path twice will error. Making re-import a no-op (`INSERT …
  ON CONFLICT(path) DO NOTHING/UPDATE`) is a **Step 3** import concern, where the design
  decision "identity = the absolute path" is enforced.
- **No positions/bookmarks/highlights.** One `books` table now; the related tables arrive
  later in the milestone as features need them.
- **Dedicated module from the start.** `Book`/`Library` live in `src/library.rs`; Step 7 can
  still review the boundary once the import and UI callers reveal what should remain
  `pub(crate)`.

> **Status:** done — committed in `a1d6822` (17 tests green, including
> `library::test::add_then_list_round_trips_books`; `cargo clippy` completed with the
> expected pre-UI dead-code warnings).

---

## Step 3 — import through Dioxus's file input

Steps 1–2 built the two tested halves: an open `Epub` can yield `BookMeta`, and a `Library`
can persist that metadata. This step joins them at the desktop boundary: choose a path,
open the EPUB once, extract metadata, and add it to a **file-backed** library. There is
still no library list and choosing a book does not change the reader; a short status line is
the visible proof that import completed.

**The crux.** Dioxus already abstracts file selection through
`<input type="file">`: Desktop opens a native dialog internally and delivers a `FileData`
in the form event. On desktop, `FileData::path()` is the selected runtime `PathBuf`; SQLite
still needs a separate stable location that survives app restarts. Keep those
responsibilities at the boundary: the Dioxus event supplies the book path,
`directories::ProjectDirs` supplies the OS-appropriate app-data directory, and `Library`
only knows how to open the database path it is given. The `Rc<Library>` provided through
Dioxus context keeps one connection alive and cheaply shareable with Step 4's list.

### Runnable check first

This step has one automated persistence check and one desktop eyeball check.

1. Add a `library.rs` test using a `tempfile::tempdir()` database. Open the file-backed
   library, add a book, drop the connection, reopen the same file, and add the same path
   again. Assert that there is still exactly one row and that its id is unchanged:

   ```rust
   #[test]
   fn file_backed_library_survives_reopen_and_reimport_is_idempotent() {
       let dir = tempfile::tempdir().expect("temp dir");
       let db_path = dir.path().join("library.sqlite3");
       let meta = BookMeta {
           title: "The Adventures of Sherlock Holmes".to_string(),
           author: Some("Arthur Conan Doyle".to_string()),
       };

       let library = Library::open(&db_path).expect("file database opens");
       let first = library.add("/books/holmes.epub", &meta).expect("first add");
       drop(library);

       let library = Library::open(&db_path).expect("database reopens");
       let second = library.add("/books/holmes.epub", &meta).expect("reimport");
       let books = library.list().expect("list succeeds");

       assert_eq!(second.id, first.id);
       assert_eq!(books, vec![second]);
   }
   ```

   Add `tempfile = "3"` under `[dev-dependencies]`, then run `cargo test`. The new test
   should fail first because `Library::open` does not exist; after adding it, it should
   expose the second failure: the current plain `INSERT` rejects a duplicate path.

2. Run `dx serve --platform desktop` and verify this exact flow:
   - an **Import EPUB** file input opens the native picker filtered to `.epub` files;
   - cancelling produces no `onchange` work and leaves the app unchanged;
   - selecting the bundled Sherlock Holmes EPUB shows `Imported: The Adventures of
     Sherlock Holmes` (or equivalent) without crashing;
   - selecting it again still succeeds rather than reporting a UNIQUE-constraint error;
   - stop and restart `dx serve`, import it once more, and confirm it still succeeds — this
     proves the app is using the on-disk DB rather than the test-only in-memory DB.

   Finish with `cargo clippy`. Step 4 will make the persisted row itself visible.

### Minimal implementation

1. Add only the storage-location and test dependencies:

   ```toml
   [dependencies]
   directories = "6"

   [dev-dependencies]
   tempfile = "3"
   ```

   Do **not** add `rfd` directly. Dioxus Desktop already uses it internally to implement
   file inputs; our code should depend on Dioxus's public form/file API rather than on that
   renderer implementation detail.

2. In `src/library.rs`, add the file-backed constructor next to `open_in_memory`; both
   continue to share the existing private `init`:

   ```rust
   pub(crate) fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
       Self::init(Connection::open(path)?)
   }
   ```

   Make `add` enforce the phase's “identity = absolute path” decision by changing the plain
   insert to an upsert and reading the row id returned by SQLite:

   ```sql
   INSERT INTO books (path, title, author)
   VALUES (?1, ?2, ?3)
   ON CONFLICT(path) DO UPDATE SET
       title = excluded.title,
       author = excluded.author
   RETURNING id
   ```

   Execute that statement with `query_row` and use the returned id when constructing
   `Book`. Reimport now updates changed metadata while retaining the row's identity, which
   makes the persistence test pass.

3. In `src/main.rs`, add a small startup helper that:
   - calls `ProjectDirs::from("com", "dimaportenko", "ook-reader")` — these three parts
     join into `com.dimaportenko.ook-reader`, matching the bundle `identifier` in
     `Dioxus.toml`; macOS convention is for the `Application Support` folder name to equal
     the bundle identifier, and keeping them identical means one app identity everywhere;
   - creates `project_dirs.data_dir()` with `std::fs::create_dir_all`;
   - opens `data_dir().join("library.sqlite3")` with `Library::open`.

   Initialize it once in `App`, wrap it in `Rc`, and provide a clone through context, just
   as `App` already does for `Rc<Epub>`:

   ```text
   use_hook → Rc<Library::open(real database path)>
       ↓
   use_context_provider(|| library.clone())
       ↓
   ImportControl reads use_context::<Rc<Library>>()
   ```

4. Add a tiny `ImportControl` component above `Reader`. It owns a
   `Signal<Option<String>>` status and renders a Dioxus file input.

   Start with a plain helper function, not a component. The import pipeline crosses three
   error types — `rbook`'s open error, `Box<dyn Error>` from `read_metadata`, and
   `rusqlite::Error` from `add` — and all three implement `std::error::Error`, so `?`
   coerces each into one `Box<dyn Error>` and the event handler stays a single `match`:

   ```rust
   /// Opens an EPUB from a picked file path, reads its metadata, and upserts
   /// it into the library. One Result so the caller shows one status line.
   fn import_epub(
       library: &Library,
       path: &std::path::Path,
   ) -> Result<library::Book, Box<dyn std::error::Error>> {
       let epub = Epub::open(path)?;
       let meta = epub::read_metadata(&epub)?;
       Ok(library.add(&path.to_string_lossy(), &meta)?)
   }
   ```

   The component wraps that helper in the file input's `onchange`:

   ```rust
   #[component]
   fn ImportControl() -> Element {
       let library = use_context::<Rc<Library>>();
       let mut status = use_signal(|| None::<String>);

       rsx! {
           div {
               style: "padding: 8px; display: flex; gap: 8px; align-items: center;",

               label {
                   "Import EPUB "
                   input {
                       r#type: "file",
                       accept: ".epub",
                       onchange: move |event| {
                           // Cancelling the picker yields no files; do nothing.
                           let Some(file) = event.files().into_iter().next() else {
                               return;
                           };
                           // FileData::path() is the real filesystem path on desktop.
                           match import_epub(&library, &file.path()) {
                               Ok(book) => status.set(Some(format!("Imported: {}", book.title))),
                               Err(error) => status.set(Some(format!("Import failed: {error}"))),
                           }
                       },
                   }
               }

               if let Some(message) = status() {
                   span { "{message}" }
               }
           }
       }
   }
   ```

   Mount it in `App`'s `rsx!` directly above the reader:

   ```rust
   ImportControl {}
   Reader {}
   ```

   Notes on the shape:
   - `event.files()` returns `Vec<FileData>` in Dioxus 0.7; the input does not set
     `multiple`, so `into_iter().next()` takes the only file, and `let … else { return }`
     makes cancel a no-op — no work, no status change.
   - the `move` closure captures the `Rc<Library>` from context: cheap to clone, and the
     single SQLite connection stays shared instead of being reopened per import;
   - errors become a status message instead of a panic — a bad pick must not crash the
     reader;
   - `if let Some(message) = status()` renders the status line only after an import
     attempt, matching the "only when present" check.

   Keep this component separate from `Reader`: import changes library data, not reading
   position, and Step 4 can extend this small library-facing component without rerendering
   or rewiring the iframe. Both `match` arms set only the status signal, and only
   `ImportControl` reads it, so the iframe never reloads on import.

### Why it works

- `ProjectDirs` selects the platform's durable application-data directory; creating that
  directory before `Connection::open` matters because SQLite creates the database file but
  not missing parent directories. The three arguments are a contract: they must stay
  stable across releases (changing them strands the existing database in the old
  directory) and, by macOS convention, should join into the same reverse-domain string as
  the bundle identifier in `Dioxus.toml`.
- `use_hook` opens the connection once for the component's lifetime. `Rc` makes the same
  single-threaded handle available through context without reopening SQLite or trying to
  clone `Connection` itself.
- Dioxus Desktop translates the file input into a native picker and returns `FileData`, so
  app code stays on Dioxus's renderer-neutral API instead of importing `rfd` directly.
- `FileData::path()` provides the real desktop path. `Epub::open(&path)` happens at that
  boundary and `read_metadata(&epub)` borrows the open value, preserving R2's “open once,
  borrow inward” design.
- `ON CONFLICT(path) DO UPDATE … RETURNING id` makes reimport idempotent while keeping the
  existing row id. It also refreshes title/author if metadata extraction improves later.
- The status signal is local UI state: reading it subscribes `ImportControl`; setting it in
  the change handler schedules only the needed rerender.

### Scope note

- **No list yet.** The status text proves import; Step 4 reads `library.list()` and renders
  all rows.
- **No reader switch yet.** The bundled `BOOK` still drives `Reader`; Step 6 replaces it
  with the selected library path.
- **Errors are displayed as strings for now.** R3 / Phase 6 Step 7 introduces a matchable
  `thiserror` type and cleans up the remaining startup `expect` path.
- **The persistence flow is desktop-only for now.** Browsers expose selected bytes, not an
  absolute path. A future web build can keep the same Dioxus input but use
  `FileData::read_bytes().await` and copy/store the EPUB instead.
- **Paths remain strings in the schema.** `to_string_lossy()` is acceptable for this
  desktop MVP; preserving arbitrary non-UTF-8 Unix paths is deferred.

> **Status:** done — committed in `d47ecb6` (18 tests green, including
> `file_backed_library_survives_reopen_and_reimport_is_idempotent`; desktop eyeball
> confirmed for import, cancel, reimport, and restart).

---

## Step 4 — render the library list

A Dioxus list over `library.list()` (title + optional author). The first cut loaded the
rows once with a local hook and never refreshed after import; the fix was to lift a shared
`Signal<Vec<Book>>` into `App` so import and list stay in sync.

**Runnable check.** Desktop eyeball with `dx serve --platform desktop`:

1. Startup shows existing library rows (title + author when present).
2. Import an EPUB → status updates **and** a new row appears without restart.
3. Reimport the same path → row count stays stable (upsert path).
4. Restart the app → rows still come from the file-backed DB.

**Minimal implementation.**

- `App` creates `books = use_signal(|| library.list().unwrap_or(vec![]))` and provides it
  via `use_context_provider` next to `Rc<Library>`.
- `LibraryBooks` reads `use_context::<Signal<Vec<library::Book>>>()` and renders with
  `for book in books.iter()`, stable `key: "{book.id}"`, structured title/author markup.
- `ImportControl` also takes that signal; on successful import it re-lists from SQLite and
  `books.set(list)` so the UI follows the store (including `ORDER BY title`).

### Why it works

- `Rc<Library>` is a durable resource (SQLite), not UI state — it does not notify anyone
  when rows change.
- A shared `Signal<Vec<Book>>` is the UI snapshot. Reading it subscribes `LibraryBooks`;
  `books.set(...)` after import schedules the rerender.
- Re-listing after add keeps sort order and upsert identity consistent with the DB, instead
  of hand-pushing the returned `Book` into a local vec.
- Row keys use `book.id` so later selection (Step 6) can reconcile correctly.

### Scope note

- **No reader switch yet.** The bundled `BOOK` still drives `Reader`; Step 6 wires row
  selection to the open path.
- **List load errors still collapse to empty.** `unwrap_or(vec![])` on startup and a quiet
  `if let Ok(list)` after import are acceptable for this eyeball step; R3 / Step 7 can make
  failures matchable and visible.
- **Cover thumbnails deferred** — title + author only, per the phase plan.
- **No delete yet.** Removing a row is Step 5 — store method first, then a control on each
  list row.

> **Status:** done — committed in `80c709e` (18 tests green; desktop eyeball confirmed for
> startup list and import-without-restart refresh).

---

## Step 5 — delete a book from the library

The list can grow but never shrink. Before wiring "open this book" (Step 6), close the
CRUD loop on the library domain: **remove a row by id**, then expose it as a control on
each list item that refreshes the same shared `Signal<Vec<Book>>` import already writes.
Still no reader switch — the bundled `BOOK` keeps driving the iframe.

**The crux.** Deleting is two layers, same shape as import:

1. **Store:** a pure `Library::remove(id)` that issues `DELETE … WHERE id = ?1` and returns
   whether a row was actually removed. Identity is the DB-assigned `id` (not the path) —
   the list already keys rows by `book.id`, and Step 6 will select by the same id.
2. **UI:** the button lives on the row (in `LibraryBooks`), not in `ImportControl`. On
   click it calls `remove`, then re-lists and `books.set(list)` — the same "store is source
   of truth, signal is the UI snapshot" pattern Step 4 established for import.

One design choice to lock in now: **delete the library row, not the `.epub` on disk.** The
phase stores *paths*, not copies (see phase design decisions). Removing a row is "forget
this book from the library"; the original file stays where the user put it. Deleting the
file itself is a later hardening concern, and not what a reader usually means by "remove
from library."

### Runnable check first

Two checks — one automated store test, one desktop eyeball.

1. A `#[test]` in `library.rs` next to the existing round-trip tests. Add two books, remove
   one by id, assert the remaining list is exactly the other book; then remove a missing id
   and assert that is a clean no-op (returns `false`, list unchanged):

   ```rust
   #[test]
   fn remove_drops_the_row_and_is_a_noop_for_unknown_ids() {
       let library = Library::open_in_memory().expect("in-memory db opens");

       let holmes = BookMeta {
           title: "The Adventures of Sherlock Holmes".to_string(),
           author: Some("Arthur Conan Doyle".to_string()),
       };
       let beowulf = BookMeta {
           title: "Beowulf".to_string(),
           author: None,
       };
       let added = library.add("/books/holmes.epub", &holmes).expect("add holmes");
       library.add("/books/beowulf.epub", &beowulf).expect("add beowulf");

       // Remove by the DB-assigned id, not by path.
       let removed = library.remove(added.id).expect("remove succeeds");
       assert!(removed, "expected an existing row to report true");

       let books = library.list().expect("list succeeds");
       assert_eq!(books.len(), 1);
       assert_eq!(books[0].title, "Beowulf");
       assert_ne!(books[0].id, added.id);

       // Unknown id: no error, no change, reports false.
       let removed_again = library.remove(added.id).expect("missing id is Ok(false)");
       assert!(!removed_again);
       assert_eq!(library.list().expect("list still one").len(), 1);
   }
   ```

   Run `cargo test remove_drops`. It should fail first with "no method named `remove`";
   after the store method lands, both assertions above should pass. Assert on *values*
   (remaining title, returned `bool`), not just `Ok` — a delete that silently no-ops on a
   real id is exactly the bug a bare `.is_ok()` would wave through.

2. Desktop eyeball with `dx serve --platform desktop`:
   - Import two distinct EPUBs (or re-use whatever is already in the DB) so the list has
     more than one row.
   - Click **Remove** on one row → that row disappears immediately; the other stays.
   - Restart the app → the removed book is still gone (file-backed DB, not just the signal).
   - The `.epub` file on disk is untouched (open it in Finder / `ls` the original path).

   Finish with `cargo clippy`.

### Minimal implementation

1. In `src/library.rs`, add `remove` next to `list`:

   ```rust
   /// Deletes the row with this id. Returns `true` if a row was removed,
   /// `false` if no row had that id (idempotent no-op).
   pub(crate) fn remove(&self, id: i64) -> rusqlite::Result<bool> {
       let n = self
           .conn
           .execute("DELETE FROM books WHERE id = ?1", params![id])?;
       Ok(n > 0)
   }
   ```

   `Connection::execute` returns the number of rows affected. Mapping that to `bool` is
   the whole API: callers that care (tests, a future "undo") get a clear answer; callers
   that don't can ignore it. Parameter binding (`?1` via `params![id]`) stays the same
   injection-safe pattern as `add`.

2. In `LibraryBooks` (`src/main.rs`), pull `Rc<Library>` from context next to the books
   signal, and put a Remove button on each row. On click: `library.remove(book.id)`, then
   re-list and `books.set(...)` — mirror the import refresh, don't hand-filter the signal:

   ```rust
   #[component]
   fn LibraryBooks() -> Element {
       let library = use_context::<Rc<Library>>();
       let mut books = use_context::<Signal<Vec<library::Book>>>();

       rsx! {
           ul {
               for book in books() {
                   li {
                       key: "{book.id}",
                       "{book.title}"
                       if let Some(author) = book.author.as_deref() {
                           span { " - {author} " }
                       }
                       button {
                           // Capture a Copy id (i64) into the move closure — not the whole Book.
                           onclick: move |_| {
                               let id = book.id;
                               if library.remove(id).is_ok() {
                                   if let Ok(list) = library.list() {
                                       books.set(list);
                                   }
                               }
                           },
                           "Remove"
                       }
                   }
               }
           }
       }
   }
   ```

   Notes on the shape:
   - `books` must be `mut` so `books.set` compiles — same as in `ImportControl`.
   - Prefer `for book in books()` (read the signal once into a `Vec` clone for this render)
     over `books.iter()` if the `onclick` needs to capture `book.id` by value into a `move`
     closure; `i64` is `Copy`, so `let id = book.id` is free. If `books.iter()` already
     works with your current Dioxus version for this pattern, keep it — the important bit is
     capturing the **id**, not a borrow of the row.
   - Failures stay quiet for now (`if … is_ok()`), matching Step 4's "list errors collapse
     to empty." A visible error status for delete can wait for R3 / Step 7.
   - No confirm dialog. One click removes the row. A "are you sure?" prompt is polish for
     later; the action is cheap to reverse (re-import the same path upserts it back).

### Why it works

- **`DELETE … WHERE id = ?1` is the dual of `INSERT`.** The schema already has `id` as
  `INTEGER PRIMARY KEY`; using it as the delete key means two books at different paths can
  never collide on remove, and the UI's `key: "{book.id}"` is the same value the store
  understands.
- **`execute` → row count → `bool`.** SQLite reports how many rows matched the `WHERE`.
  Turning that into `Ok(true)` / `Ok(false)` keeps "row missing" out of the error channel —
  a missing id is not a failure of the database, it's a no-op. Real failures (disk full,
  locked DB) still come back as `Err`.
- **Re-list after remove, don't filter the signal.** The DB is the source of truth
  (`ORDER BY title`, upsert identity). Hand-removing one element from the `Vec` would
  work today, but would drift the moment a future trigger (reimport, another window,
  a migration) changes the list out of band. Import already re-lists; delete should too.
- **Row-only, not file-system.** Because the library stores paths, not copies, `remove`
  never touches the `.epub`. That matches the phase's "store the path" decision and keeps
  this step a pure SQLite + signal change — no new I/O surface.

### Scope note

- **No reader switch yet.** `const BOOK` still drives `Reader`. If the user later opens a
  book and then deletes it from the list, "what happens to the open reader" is a Step 6
  concern (close / fall back), not this one.
- **No confirm dialog, no undo, no toast.** One click, row gone. Re-import restores it via
  the existing upsert path.
- **Does not delete the file on disk.** Intentional; see the crux.
- **Quiet on store errors.** Same honesty level as Step 4's list load; Step 7 / R3 can make
  failures matchable and visible.
- **No multi-select / bulk delete.** One row, one button, one idea.

> **Status:** done — committed in `db5b79c` (19 tests green; desktop eyeball confirmed for
> immediate removal, persistence across restart, and leaving the source EPUB untouched).


---

## Step 6 — open a library book in the reader

The library now owns the runtime paths, but the reader still ignores them and opens the
bundled `BOOK`. This step replaces that compile-time choice with one piece of app state:
an `Option<OpenBook>`. A row click opens the path once; mounting a keyed `Reader` gives
that EPUB to both spine loading and the asset handler, so changing books also resets all
per-book navigation hooks.

### Runnable check first

This is Dioxus wiring, so the primary check is a desktop eyeball rather than a unit test.
Before editing, run `dx serve --platform desktop` and note that the bundled Sherlock Holmes
book renders even when no library row was selected. After the implementation:

1. Start the app with at least two imported EPUBs. Before opening a row, no book iframe is
   shown — only the library list and import control.
2. Click **Open** on book A. Its first chapter renders, its title is shown above the reader,
   and chapter/page navigation still works. The library list is hidden while reading.
3. Advance book A away from its first page, click **Close**, then open book B. Book B
   renders — including its CSS/images — and navigation starts at chapter 1, page 1 rather
   than retaining book A's state.
4. Restart the app and open a persisted row without re-importing it. It still renders.
5. Import a temporary EPUB, remove or rename that file, then click **Open**. The app stays
   alive and shows an `Open failed: …` message under the list instead of panicking.
6. Close the reader, then **Remove** a row. The row disappears from the list and the source
   file remains untouched.

Finish with `cargo test` (the existing 19 tests remain green) and `cargo clippy`.

### Minimal implementation

1. In `src/main.rs`, introduce a small UI-state value near `BridgeMsg`:

   ```rust
   #[derive(Clone)]
   struct OpenBook {
       id: i64,
       title: String,
       epub: Rc<Epub>,
   }

   impl PartialEq for OpenBook {
       fn eq(&self, other: &Self) -> bool {
           self.id == other.id
       }
   }
   ```

   `OpenBook` is not a database row: it means “this row's path opened successfully.” Keep
   the `Rc<Epub>` here so the row click is the only `Epub::open` boundary and both consumers
   borrow/share the same parsed book. The manual `PartialEq` compares stable library
   identity because `Epub` itself is not a value type that needs structural comparison.

2. In `App`, replace the bundled-EPUB hook/context with selection state:

   ```rust
   let open_book = use_signal(|| None::<OpenBook>);
   use_context_provider(|| open_book);
   ```

   Remove the unconditional asset-handler registration. Treat library and reader as
   exclusive screens, and key the reader by row id:

   ```rust
   if let Some(book) = open_book() {
       Reader {
           key: "{book.id}",
           book,
       }
   } else {
       LibraryBooks {}
       ImportControl {}
   }
   ```

3. In `LibraryBooks`, consume `Signal<Option<OpenBook>>` and add a local
   `Signal<Option<String>>` for open errors. Add an **Open** button beside **Remove**. Capture
   owned `id`, `title`, and `path` values for its `move` closure, then use this shape:

   ```rust
   match Epub::open(&path) {
       Ok(epub) => {
           open_status.set(None);
           open_book.set(Some(OpenBook {
               id,
               title: title.clone(),
               epub: Rc::new(epub),
           }));
       }
       Err(error) => open_status.set(Some(format!("Open failed: {error}"))),
   }
   ```

   Render the status under the list so open failures are visible.

4. Change `Reader` to accept `book: OpenBook` instead of reading `Rc<Epub>` from context.
   Register the asset handler with `book.epub.clone()`, build the spine from the same EPUB,
   show `book.title` in the reader chrome, and add a **Close** control that sets
   `open_book` back to `None`. Keep the existing nav/bridge/iframe body otherwise unchanged.

   The component key is essential: when the selected id changes, Dioxus unmounts the old
   `Reader` and mounts a fresh one. That reruns `use_hook`/`use_reader_state`, registers the
   handler for the new EPUB, and resets chapter/page state. Merely changing a prop while
   keeping the same component instance would leave one-time hook initialization tied to the
   previous book. Exclusive screens make that switch go through **Close** first; the key
   still protects the mount lifetime when the open id changes.

5. Remove the production use of `BOOK`. Gate the fixture constant with `#[cfg(test)]` so the
   application no longer has a default book path. Keep the bundled file as a test fixture for
   the EPUB suite.

### Why it works

- **Open at the event boundary, share inward.** `Epub::open` is fallible user I/O, so the
  click handler can report failure without constructing broken reader state. On success,
  `Rc<Epub>` gives the spine loader and asset handler shared ownership without reopening the
  zip or cloning its contents.
- **`Option` models the UI honestly.** `None` means library mode; `Some(OpenBook)` means a
  valid EPUB is ready. There is no sentinel path and no `expect` for a user-controlled file.
- **Exclusive screens keep one mode honest.** While reading, the list is unmounted, so open
  and remove cannot race the active reader. **Close** is the deliberate return path to the
  library.
- **The key defines state identity.** Dioxus preserves hook slots while a component keeps
  its identity. `key: book.id` says navigation state belongs to one library row; opening a
  different id after Close deliberately creates a new state lifetime.
- **The handler follows the mounted reader.** `use_asset_handler` registers on mount and
  removes its route handler on cleanup. Putting it inside the keyed reader keeps resource
  requests and rendered spine documents on the same `Rc<Epub>`.

### Scope note

- No saved reading position yet; every open starts at chapter 1/page 1. Persistence is the
  next library-adjacent feature.
- Open failures are still display strings. Step 7 / R3 replaces boxed/opaque errors with a
  matchable `thiserror` type and reviews the remaining startup `expect`s.
- Re-clicking the already-open row is not a concern under exclusive screens; the reader is
  only reachable after a successful open.
- This step intentionally uses exclusive library/reader screens rather than an always-visible
  list with side-by-side selection. Richer routing can come later.

### Leftovers for Step 7

- The fixture constant is still named `BOOK`, not renamed to `TEST_BOOK`, though it is
  correctly gated with `#[cfg(test)]`.
- `load_spine(...).expect("bundled epub should load")` still uses bundled-book wording even
  though the spine now comes from a library path.
- Remove does not clear `open_book` by id. Under exclusive screens that path is unreachable
  (the list only mounts when `open_book` is already `None`), but a later always-visible list
  or multi-pane shell would need the clear.

> **Status:** done — committed in `907b0a6` (19 tests green; exclusive library/reader UX with
> Close, open-failure status, and leftovers recorded for Step 7).
