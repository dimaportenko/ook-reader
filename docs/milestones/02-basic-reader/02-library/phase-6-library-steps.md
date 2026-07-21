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
7. **Import into managed storage** — copy into `data_dir()/books/`, store source + managed
   paths, and prove open survives deletion of the source. `#[test]` + eyeball. *(done)*
8. **Re-import replaces the managed copy** — keep the row id while refreshing the stored
   bytes and metadata; repair a missing copy without leaking files. `#[test]`. *(done)*
9. **Remove the managed copy** — delete the row first, then the owned file; tolerate an
   already-missing managed file. `#[test]` + eyeball. *(done)*
10. **Cover image in `BookMeta`** — extract the manifest cover (bytes + media type) into
    `BookMeta.cover: Option<CoverImage>`, best-effort so metadata stays infallible.
    `#[test]` against the bundled book. *(done)*
11. **Persist & show covers (file beside the managed copy)** — 11a: cover-file lifecycle
    (import / re-import / remove / cleanup) with a nullable `cover_path` column *(done)*;
    11b: an app-level `covers` asset route + thumbnails in the list *(done)*.
12. **Review & refactor** — tidy module boundaries and errors, then delete the single-book
    scaffolding. *(suggested — punch-list below)*

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
- **Dedicated module from the start.** `Book`/`Library` live in `src/library.rs`; Step 10 can
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
- **Errors are displayed as strings for now.** R3 / Phase 6 Step 10 introduces a matchable
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
  `if let Ok(list)` after import are acceptable for this eyeball step; R3 / Step 10 can make
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
     to empty." A visible error status for delete can wait for the managed-remove step, with
     the final typed-error cleanup in R3 / Step 10.
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
- **Quiet on store errors.** Same honesty level as Step 4's list load; Step 9 can make
  remove failures visible, while R3 / Step 10 makes store failures matchable.
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
- Open failures are still display strings. Step 10 / R3 replaces boxed/opaque errors with a
  matchable `thiserror` type and reviews the remaining startup `expect`s.
- Re-clicking the already-open row is not a concern under exclusive screens; the reader is
  only reachable after a successful open.
- This step intentionally uses exclusive library/reader screens rather than an always-visible
  list with side-by-side selection. Richer routing can come later.

### Leftovers for Step 10

- The fixture constant is still named `BOOK`, not renamed to `TEST_BOOK`, though it is
  correctly gated with `#[cfg(test)]`.
- `load_spine(...).expect("bundled epub should load")` still uses bundled-book wording even
  though the spine now comes from a library path.
- Remove does not clear `open_book` by id. Under exclusive screens that path is unreachable
  (the list only mounts when `open_book` is already `None`), but a later always-visible list
  or multi-pane shell would need the clear.

> **Status:** done — committed in `907b0a6` (19 tests green; exclusive library/reader UX with
> Close, open-failure status, and leftovers recorded for the review step).

---

## Step 7 — import into managed storage

Steps 1–6 store the path returned by the picker. That path belongs to the user: moving or
renaming the source leaves a dangling row. This step changes only the import side of the
ownership model. A successful import creates an owned copy under `data_dir()/books/`, and
`Book.path` points at that copy. Removal remains row-only until Step 9.

### The crux

Keep two paths with different jobs:

- `source_path` is the canonical absolute path selected by the user. It is a private dedup
  key and is never used to open the reader.
- `path` is the managed copy under `books_dir`. It is the path returned in `Book`, opened by
  the reader, and later deleted by `Library::remove`.

`Library` therefore owns both its SQLite `Connection` and its `books_dir: PathBuf`. Import
copies first and reads metadata from the **copy**, so the metadata is guaranteed to describe
the bytes the library retained. A UUID filename prevents collisions between unrelated books
with the same source filename.

### Runnable check first

Add one store test using a temporary database, managed directory, and disposable source copy:

```rust
#[test]
fn import_opens_from_managed_copy_after_source_is_deleted() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("library.sqlite3");
    let books_dir = dir.path().join("books");
    std::fs::create_dir_all(&books_dir).expect("books dir");

    let source = dir.path().join("holmes-source.epub");
    std::fs::copy(crate::BOOK, &source).expect("fixture source");

    let library = Library::open(&db_path, &books_dir).expect("library opens");
    let added = library.add_from_path(&source).expect("import succeeds");

    assert!(std::path::Path::new(&added.path).starts_with(&books_dir));
    assert_ne!(std::path::Path::new(&added.path), source.as_path());

    std::fs::remove_file(&source).expect("delete source");
    let epub = rbook::Epub::open(&added.path).expect("managed copy opens");
    let meta = crate::epub::read_metadata(&epub).expect("managed metadata");

    assert!(meta.title.contains("Sherlock Holmes"));
    assert_eq!(added.title, meta.title);
}
```

Run `cargo test import_opens_from_managed_copy`. It should initially fail because
`Library::open` still takes one path and `add_from_path` does not exist. After the store
change, run the complete suite and `cargo clippy`.

Desktop eyeball:

1. Stop the app and remove the pre-Step-7 **development** database and managed-books
   directory. `CREATE TABLE IF NOT EXISTS` cannot add `source_path` to the old schema; this
   explicit reset is the intentionally migration-free pre-release policy.
2. Start `dx serve --platform desktop` and import an EPUB from outside app storage.
3. Confirm a UUID-named `.epub` appears under
   `~/Library/Application Support/com.dimaportenko.ook-reader/books/` on macOS.
4. Move the original and click **Open**. The reader must still render the book.

### Minimal implementation

1. Add the collision-safe filename dependency explicitly:

   ```toml
   uuid = { version = "1", features = ["v4"] }
   ```

   Do not synthesize uniqueness from timestamps and process IDs.

2. Extend the schema and store:

   ```sql
   CREATE TABLE IF NOT EXISTS books (
       id          INTEGER PRIMARY KEY,
       path        TEXT NOT NULL UNIQUE,
       source_path TEXT NOT NULL UNIQUE,
       title       TEXT NOT NULL,
       author      TEXT
   )
   ```

   ```text
   Library { conn, books_dir }
   Library::open(db_path, books_dir)
   Library::open_in_memory(books_dir) // tests only
   ```

   Keep directory creation at the boundary for now: `open_library` and tests call
   `create_dir_all`, while `Library::open` opens the paths it is given. This avoids lying
   about an I/O failure by converting it to `rusqlite::Error::InvalidPath`. Step 10 will
   introduce the combined typed error and can move that responsibility back into `Library`.

3. Implement `add_from_path(&Path)` in this order:

   ```text
   canonicalize source_path
   query by source_path; if already present, return the existing Book for now
   choose books_dir/<uuid>.epub
   copy source -> managed destination
   open the managed destination and read its metadata
   insert source_path + managed path + metadata
   return Book whose path is the managed destination
   ```

   Production must only import through `add_from_path`; do not preserve a production
   `add(path, meta)` route that can create rows without owned files. A private test helper is
   acceptable only for narrow SQL mapping tests. Import `rusqlite::OptionalExtension` for
   the optional source lookup.

   Once the copy exists, every later failure must attempt to delete it. Treat `NotFound` as
   already clean; report other cleanup failures (temporarily with `eprintln!`) rather than
   silently discarding them. This is best-effort cross-store consistency, not an atomic
   filesystem/SQLite transaction.

4. Collapse `main.rs` import glue to `library.add_from_path(path)`. `LibraryBooks` already
   opens `book.path`, so no reader or Dioxus state change is needed.

### Why it works

- The managed path is controlled by the application, so source moves no longer invalidate
  the row.
- Canonicalizing once makes source identity stable for relative paths and `..` components.
- Reading metadata from the copied file validates the exact retained bytes rather than a
  source that could differ by the time copying finishes.
- Returning an existing source row keeps re-import idempotent without introducing the
  replacement algorithm into this first increment. Step 8 deliberately upgrades that
  behavior to refresh the bytes.

### Scope note

- Re-import returns the existing managed copy in this step; replacing it is Step 8.
- Remove still deletes only the row until Step 9. Avoid using Remove in the Step 7 eyeball.
- The pre-release database reset is intentional; no schema migrator yet.
- Paths remain lossy UTF-8 strings in SQLite, matching the existing desktop MVP decision.
- Filesystem work remains synchronous on the UI thread; async progress is deferred.

> **Status:** done — committed in `f9edefd` (20 tests green; desktop import, source move,
> managed-copy presence, and reader open confirmed).

---

## Step 8 — re-import replaces the managed copy without changing row identity

Step 7 makes first import durable, but its early return means choosing the same source again
does not refresh changed bytes. Re-import must retain `books.id` while replacing `Book.path`
and metadata together. Preserving the **row id**, not the filename, is what keeps future
positions and highlights attached to the same logical book.

### Runnable check first

Two focused tests in `library.rs`, next to the Step 7 import test. Both fail against the
current code: `add_from_path` early-returns the existing row, so `second.path ==
first.path` and the repaired path doesn't exist.

```rust
#[test]
fn reimport_replaces_the_managed_copy_without_leaking_the_old_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("library.sqlite3");
    let books_dir = dir.path().join("books");
    std::fs::create_dir_all(&books_dir).expect("books dir");

    let source = dir.path().join("holmes-source.epub");
    std::fs::copy(crate::BOOK, &source).expect("fixture source");

    let library = Library::open(&db_path, &books_dir).expect("library opens");
    let first = library.add_from_path(&source).expect("first import");
    let second = library.add_from_path(&source).expect("reimport");

    // Same logical book, fresh bytes: id stable, managed path replaced.
    assert_eq!(second.id, first.id);
    assert_ne!(second.path, first.path);
    assert!(!Path::new(&first.path).exists());
    assert!(Path::new(&second.path).exists());

    // Exactly one managed file and one row — nothing leaked, nothing duplicated.
    let files = std::fs::read_dir(&books_dir).expect("read books dir").count();
    assert_eq!(files, 1);
    assert_eq!(library.list().expect("list succeeds"), vec![second]);
}

#[test]
fn reimport_repairs_a_missing_managed_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("library.sqlite3");
    let books_dir = dir.path().join("books");
    std::fs::create_dir_all(&books_dir).expect("books dir");

    let source = dir.path().join("holmes-source.epub");
    std::fs::copy(crate::BOOK, &source).expect("fixture source");

    let library = Library::open(&db_path, &books_dir).expect("library opens");
    let first = library.add_from_path(&source).expect("first import");

    // Simulate a hand-deleted managed file: the row now points at nothing.
    std::fs::remove_file(&first.path).expect("delete managed copy");

    let repaired = library.add_from_path(&source).expect("reimport repairs");

    assert_eq!(repaired.id, first.id);
    rbook::Epub::open(&repaired.path).expect("repaired copy opens");
}
```

Run `cargo test reimport` first (both should go green), then the full `cargo test`
(22 tests) and `cargo clippy`.

### Minimal implementation

Remove Step 7's early return and use this order:

```text
canonicalize source_path
look up the previous managed path for that source, if any
copy source to a fresh UUID destination
open that destination and read its metadata
upsert the row, replacing path/title/author but preserving SQLite row id
on SQL failure: clean up the fresh destination
on success: clean up the previous managed path
return the row produced by the upsert
```

Full replacement for `add_from_path`:

```rust
pub(crate) fn add_from_path(
    &self,
    source_path: &Path,
) -> Result<Book, Box<dyn std::error::Error>> {
    let source_path = source_path.canonicalize()?;
    let source_path_text = source_path.to_string_lossy().into_owned();

    // The managed file this source currently owns, if any. Kept alive until
    // the new copy is committed, then deleted — new-first ordering means a
    // crash in between leaves an orphan file, never a row pointing at nothing.
    let previous_path: Option<String> = self
        .conn
        .query_row(
            "SELECT path FROM books WHERE source_path = ?1",
            params![&source_path_text],
            |row| row.get(0),
        )
        .optional()?;

    let managed_path = self.books_dir.join(format!("{}.epub", Uuid::new_v4()));

    if let Err(error) = fs::copy(&source_path, &managed_path) {
        cleanup_managed_file(&managed_path);
        return Err(Box::new(error));
    }

    let result = (|| -> Result<Book, Box<dyn std::error::Error>> {
        let epub = Epub::open(&managed_path)?;
        let meta = epub::read_metadata(&epub)?;
        let managed_path_text = managed_path.to_string_lossy().into_owned();

        let book = self.conn.query_row(
            "INSERT INTO books (path, source_path, title, author)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(source_path) DO UPDATE SET
                path = excluded.path,
                title = excluded.title,
                author = excluded.author
            RETURNING id, path, title, author",
            params![
                &managed_path_text,
                &source_path_text,
                &meta.title,
                meta.author.as_deref(),
            ],
            Self::read_book,
        )?;

        Ok(book)
    })();

    match &result {
        Err(_) => cleanup_managed_file(&managed_path),
        Ok(_) => {
            if let Some(previous) = previous_path {
                cleanup_managed_file(Path::new(&previous));
            }
        }
    }

    result
}
```

Rename `cleanup_failed_import` to `cleanup_managed_file`, since it now also cleans up
replaced copies, not just failed imports:

```rust
/// Best-effort delete of a library-owned file. NotFound means the desired
/// state already holds; other failures are reported, not returned — the DB
/// has already committed by the time this runs.
fn cleanup_managed_file(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!("failed to clean up managed copy {}: {error}", path.display());
        }
    }
}
```

SQLite updates the existing row on conflict, so its `id` remains stable. The newly
generated path is stored atomically with the refreshed metadata. Only after that succeeds
is the old file removed. If the process stops between those last two operations, the row
still points at a valid new copy and the old file is merely an orphan; that is safer than
a readable row pointing at a deleted file.

For cleanup, treat `NotFound` as success and report other failures. Until Step 10 gives
cleanup a typed outcome, logging an old-file cleanup failure is preferable to returning a
plain `Err` after the database has already committed: the current import handler otherwise
would say “failed” and skip refreshing the list even though the import succeeded.

### Why it works

- **`ON CONFLICT(source_path) DO UPDATE`** hits the existing row instead of inserting, so
  SQLite keeps its `id` — that's the whole "row identity survives re-import" guarantee,
  and `RETURNING` hands back the refreshed row in one round trip.
- A new destination repairs a hand-deleted managed file and ensures changed source bytes
  are actually adopted: the fresh copy is always made from the source, and the upsert
  overwrites the dangling `path` regardless of what it pointed at.
- Updating `path` removes the stale-content bug where metadata came from the new source but
  the reader opened the old copy.
- New-first ordering prioritizes a valid readable row. The unavoidable cross-store failure
  mode is a benign orphan, which can be swept later.
- No `previous != new` comparison is needed before deleting the old file: the new path is a
  fresh UUID, so it can never equal the previous one.

### Scope note

- Identity is still canonical source path, not content hash or EPUB identifier.
- Cleanup is best-effort because SQLite and the filesystem cannot share one transaction.
- An orphan sweep and richer “import succeeded, cleanup warned” outcome wait for Step 10.
- No Dioxus change is required beyond the Step 7 import call.
- The two tests share near-identical setup (tempdir + books dir + fixture source + open)
  with the Step 7 test; extracting a small test helper is a Step 10 tidy, not a blocker.

> **Status:** done — committed in `a0cd057` (24 tests green, including both focused
> re-import tests).

---

## Step 9 — remove the row and its managed copy

The library now owns every path stored in `Book.path`, so Remove can finally
mean “forget the book and delete the application-owned bytes.” The user's
original source remains untouched.

### Runnable check first

Keep removal behaviors separate. Two new `#[test]`s in `library.rs`, next to
the existing import tests (same tempdir + `crate::BOOK` fixture setup):

1. `remove_deletes_the_row_and_managed_copy` imports a disposable EPUB,
   removes its id, and asserts `Ok(true)`, an empty list, and an absent
   managed path:

   ```rust
   #[test]
   fn remove_deletes_the_row_and_managed_copy() {
       let dir = tempfile::tempdir().expect("temp dir");
       let db_path = dir.path().join("library.sqlite3");
       let books_dir = dir.path().join("books");
       std::fs::create_dir_all(&books_dir).expect("books dir");

       let source = dir.path().join("holmes-source.epub");
       std::fs::copy(crate::BOOK, &source).expect("fixture source");

       let library = Library::open(&db_path, &books_dir).expect("library opens");
       let added = library.add_from_path(&source).expect("import succeeds");

       let removed = library.remove(added.id).expect("remove succeeds");

       assert!(removed, "expected an existing row to report true");
       assert!(library.list().expect("list succeeds").is_empty());
       assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
       assert!(source.exists(), "the user's original source is untouched");
   }
   ```

2. `remove_succeeds_when_the_managed_copy_is_already_missing` imports,
   manually deletes the managed path, removes the id, and asserts `Ok(true)`
   plus an empty list:

   ```rust
   #[test]
   fn remove_succeeds_when_the_managed_copy_is_already_missing() {
       let dir = tempfile::tempdir().expect("temp dir");
       let db_path = dir.path().join("library.sqlite3");
       let books_dir = dir.path().join("books");
       std::fs::create_dir_all(&books_dir).expect("books dir");

       let source = dir.path().join("holmes-source.epub");
       std::fs::copy(crate::BOOK, &source).expect("fixture source");

       let library = Library::open(&db_path, &books_dir).expect("library opens");
       let added = library.add_from_path(&source).expect("import succeeds");

       // Simulate a hand-deleted managed file: the row now points at nothing.
       std::fs::remove_file(&added.path).expect("delete managed copy");

       let removed = library.remove(added.id).expect("missing file is not an error");

       assert!(removed, "a stale row is still removable");
       assert!(library.list().expect("list succeeds").is_empty());
   }
   ```

3. Keep the existing unknown-id check
   (`remove_drops_the_row_and_is_a_noop_for_unknown_ids`): it returns
   `Ok(false)` without changing the store, and compiles unchanged.

Run `cargo test remove`. The two new tests should fail against a row-only
`remove` (it leaves the managed file behind), then pass once the
implementation lands.

Then run the desktop check:

1. Import a book and record both the original and managed paths.
2. Click **Remove**. The row disappears immediately and stays gone after
   restart.
3. The managed file is gone; the original source still exists.
4. Manually delete another managed file before clicking Remove. The stale row
   should still be removable without an error.

Finish with `cargo test` and `cargo clippy`.

### Minimal implementation

In `src/library.rs`, one statement does both the lookup and the delete.
SQLite's `RETURNING` clause (3.35+, guaranteed by rusqlite's `bundled`
feature) hands back the `path` of the row it just deleted, so there is no gap
between "read the managed path" and "remove the row" — the row either existed
and is now gone, or it never existed:

```rust
pub(crate) fn remove(&self, id: i64) -> rusqlite::Result<bool> {
    let removed_path: Option<String> = self
        .conn
        .query_row(
            "DELETE FROM books WHERE id = ?1 RETURNING path",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(removed_path) = removed_path else {
        return Ok(false);
    };

    cleanup_managed_file(Path::new(&removed_path));

    Ok(true)
}
```

Notes on the shape:

- `query_row`, not `execute`: a statement with `RETURNING` produces rows, and
  rusqlite requires the row-reading API for it. `.optional()` maps "deleted
  nothing" to `None`, preserving the idempotent `Ok(false)` contract.
- File deletion reuses `cleanup_managed_file`, the same helper import-failure
  cleanup uses: success and `NotFound` are both fine, and any other I/O error
  is logged to stderr rather than returned. The method therefore keeps its
  `rusqlite::Result<bool>` signature instead of widening to `Box<dyn Error>`.
- Row-first ordering still holds — the SQL runs before the file removal — so
  a failed file deletion can leave a silent orphan on disk, never a broken
  visible row. Because no new error can surface, the existing Remove handler
  in `LibraryBooks` needs no change.
- Optional polish: `cleanup_managed_file`'s log message says "imported copy",
  which reads oddly from `remove`; "managed copy" fits both call sites.

### Why it works

- `Book.path` is now an application-owned capability, so deleting it cannot
  remove the user's original file.
- `DELETE … RETURNING` makes lookup and delete one atomic statement: there is
  no window where the row can change between reading its path and removing
  it, and no `removed == 0` re-check is needed.
- `OptionalExtension` maps “no row” to `None`, preserving idempotent
  `Ok(false)` behavior.
- `cleanup_managed_file` treats `NotFound` as success — the desired
  filesystem state already holds — so a stale row is still removable.
- Logging (rather than returning) other I/O errors is a deliberate trade-off:
  the worst case is an invisible orphan file, not an error message the user
  cannot act on.

### Scope note

- There is no confirmation dialog, undo, trash, or orphan sweep yet — a
  failed managed-file deletion is visible only in stderr.
- `Box<dyn Error>` remains temporary on `add_from_path`; `remove` stays on
  `rusqlite::Result` because cleanup failures are logged, not returned.
- Step 12 (review & refactor; numbered 10 when this was written) will introduce
  the matchable error/outcome type, remove startup `expect`s, rename `BOOK` to
  `TEST_BOOK`, and review the `Library`/EPUB boundary.

> **Status:** done — committed in `a0cd057` (24 tests green; desktop removal,
> restart persistence, original-source preservation, and stale-row removal confirmed).

---

## Step 10 — cover image in `BookMeta`

*(Refined in: this step was inserted before the review-and-refactor closer, which moved
from Step 10 to Step 12. It deliberately pulls forward the cover thread deferred at
Step 4 — domain first, storage and UI in Step 11.)*

### Runnable check first

A `#[test]` in `epub.rs`'s test module. The existing `reads_cover_image_bytes` test
already proves the `rbook` API hands back real image bytes for the bundled book; this new
test pins that `read_metadata` now *carries* them:

```rust
#[test]
fn read_metadata_extracts_the_cover_image() {
    let epub = Epub::open(crate::BOOK).expect("open fixture book");
    let meta = read_metadata(&epub).expect("bundled epub metadata should read");

    let cover = meta.cover.expect("the bundled book declares a cover image");
    assert!(cover.media_type.starts_with("image/"));
    // Real image bytes, not a stray placeholder: JPEG → FF D8 FF, PNG → 89 50 4E 47.
    let is_jpeg = cover.bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
    let is_png = cover.bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    assert!(
        is_jpeg || is_png,
        "expected JPEG or PNG bytes, got {} bytes",
        cover.bytes.len()
    );
}
```

Watch it fail first — it won't even compile while `BookMeta` has no `cover` field, which
is the borrow checker acting as the red phase — then pass.

### Minimal implementation

All in `epub.rs`. A small struct, one new field, one lookup:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoverImage {
    pub(crate) media_type: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookMeta {
    pub(crate) title: String,
    pub(crate) author: Option<String>,
    pub(crate) cover: Option<CoverImage>,
}
```

In `read_metadata`, before the final `Ok(...)` (reusing the same manifest lookup the
`reads_cover_image_bytes` test exercises):

```rust
let cover = epub.manifest().cover_image().and_then(|entry| {
    let bytes = entry.read_bytes().ok()?;
    Some(CoverImage {
        media_type: entry.media_type().to_string(),
        bytes,
    })
});
```

*(Check `media_type()`'s exact return type against the pinned rbook 0.7 — the existing
test calls `.starts_with` on it, so it's string-like; `.to_string()` may need to be
`.as_str().to_string()` or similar.)*

**Ripple:** the `BookMeta { title, author }` literals in `library.rs`'s tests stop
compiling — add `cover: None` to each. `add_from_path` needs no change; it reads only
`title`/`author` and simply drops the cover for now.

### Why it works

- **`manifest().cover_image()` is the whole lookup.** EPUB declares covers two different
  ways (EPUB 3's `properties="cover-image"` manifest flag, EPUB 2's `<meta name="cover">`
  workaround), and rbook resolves both behind this one call — the same reason the project
  chose a parsing crate over hand-rolling OPF traversal.
- **Best-effort by construction.** The `and_then` + `.ok()?` chain means *any* missing
  piece — no cover declared, unreadable bytes — collapses to `None` instead of an error.
  `?` on an `Option` inside a closure returning `Option` short-circuits exactly like it
  does on `Result`, and it's what keeps `read_metadata` effectively infallible — the
  property Step 12's punch-list item b (drop the `Result`) depends on. A cover is
  decoration; its absence should never block an import.
- **`media_type` rides along now** because the eventual `<img>` render needs a content
  type for its data URL, and capturing it here avoids re-opening the EPUB later just to
  ask what kind of image we already had in hand.

### Scope note

Nothing persists or renders yet: `Book`, the SQLite schema, and the UI are untouched, and
the extracted bytes are dropped by `add_from_path`. Step 11 decides storage (a file
beside the managed copy vs a BLOB column) and puts thumbnails in the list — that's where
the bytes stop being dropped. There's no coverless fixture, so the `None` path stays
untested; it's guaranteed by the combinator chain rather than pinned by a test.

> **Status:** done — committed in `5202cff` (25 tests green; metadata extraction test
> confirmed real JPEG or PNG bytes and an `image/*` media type).

---

## Step 11 — persist & show covers (option B: file beside the managed copy)

### The decision, recorded

Three storage shapes were on the table: **(A)** a `cover BLOB` column rendered as base64
data URLs, **(B)** a cover *file* beside the managed `.epub`, served through a second
asset-handler route, **(C)** no storage — re-extract from the zip on every render. **B
chosen**: it keeps image bytes out of the DOM (no base64-inflated strings held by the
list) and out of the DB, and it generalizes the asset-handler pattern the reader already
uses for chapter resources. The price is a second managed file per book — but its
lifecycle (import writes, re-import replaces, remove deletes, failed import cleans up) is
exactly the machinery Steps 7–9 built and tested, now applied twice. A's transactional
"cover dies with the row for free" is real; here that guarantee is earned by tests
instead. C loses on reopening N zips per render for static data.

Split in two: **11a** is pure domain (file + column + lifecycle, all `#[test]`-able),
**11b** is delivery (asset route + `<img>`, eyeball). Data first, UI last.

---

### Step 11a — the cover file's lifecycle

#### Runnable check first

Three new tests in `library.rs`, mirroring the Steps 7–9 lifecycle tests — plus one
existing test that goes red on purpose:

```rust
#[test]
fn import_writes_a_cover_file_next_to_the_managed_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source) = library_with_source(&dir); // Step 12(a) proposes this helper — pulling it forward here is fair game
    let added = library.add_from_path(&source).expect("import succeeds");

    let cover_path = added.cover_path.expect("bundled book has a cover");
    assert!(Path::new(&cover_path).starts_with(dir.path().join("books")));
    assert!(Path::new(&cover_path).exists());
    // The stored extension round-trips through the serve-time content-type lookup.
    assert!(crate::epub::content_type_for(&cover_path).starts_with("image/"));
}

#[test]
fn reimport_replaces_the_cover_without_leaking_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source) = library_with_source(&dir);

    let first = library.add_from_path(&source).expect("first import");
    let second = library.add_from_path(&source).expect("reimport");

    let first_cover = first.cover_path.expect("first import has a cover");
    let second_cover = second.cover_path.expect("reimport has a cover");

    // Same logical book, fresh files: the old cover is gone, the new one exists.
    assert_ne!(second_cover, first_cover);
    assert!(!Path::new(&first_cover).exists());
    assert!(Path::new(&second_cover).exists());

    // Exactly one .epub + one cover — nothing leaked, nothing duplicated.
    // (This is the assertion that goes red in the *old* reimport test: its
    // `files == 1` becomes `files == 2` once covers land next to the copies.)
    let files = std::fs::read_dir(dir.path().join("books"))
        .expect("read books dir")
        .count();
    assert_eq!(files, 2);
}

#[test]
fn remove_deletes_the_cover_file_too() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source) = library_with_source(&dir);

    let added = library.add_from_path(&source).expect("import succeeds");
    let cover_path = added.cover_path.clone().expect("import has a cover");

    let removed = library.remove(added.id).expect("remove succeeds");

    assert!(removed, "expected an existing row to report true");
    assert!(library.list().expect("list succeeds").is_empty());
    assert!(!Path::new(&added.path).exists(), "managed copy is deleted");
    assert!(!Path::new(&cover_path).exists(), "cover file is deleted");
    assert!(source.exists(), "the user's original source is untouched");
}
```

**Expected red:** `reimport_replaces_the_managed_copy_without_leaking_the_old_file`
asserts `files == 1` in `books_dir` — with a cover written that becomes 2. Updating that
assertion is part of this step, not a regression.

#### Minimal implementation

1. **Schema + row type.** Add a nullable column and thread it through:

   ```sql
   cover_path TEXT
   ```

   `Book` gains `cover_path: Option<String>`; `read_book` gains `row.get(4)?`; `list`'s
   SELECT and both `RETURNING` clauses name the new column; the `ON CONFLICT(source_path)
   DO UPDATE` sets `cover_path = excluded.cover_path`.

   *Migration caveat:* `CREATE TABLE IF NOT EXISTS` won't add a column to the dev database
   you already have. Cheapest: delete it
   (`~/Library/Application Support/com.dimaportenko.ook-reader/` on macOS) and re-import —
   it's dev data, and re-import rebuilds everything. A real `PRAGMA user_version`
   migration is recorded as deferred (worth doing when the schema next changes *after*
   real data exists).

2. **A pure inverse of `content_type_for`,** in `epub.rs` beside it — media type → file
   extension, `None` for anything we wouldn't know how to serve back:

   ```rust
   pub(crate) fn extension_for(media_type: &str) -> Option<&'static str> {
       match media_type {
           "image/jpeg" => Some("jpg"),
           "image/png" => Some("png"),
           "image/gif" => Some("gif"),
           "image/svg+xml" => Some("svg"),
           _ => None,
       }
   }
   ```

   One-line `#[test]`: for each mapped pair, `content_type_for` of a name ending in the
   extension gives back the media type — the round-trip *is* the spec.

3. **Write the file in `add_from_path`,** inside the existing fallible block, after
   `read_metadata`. Reuse the UUID stem so the pair sorts together on disk:

   ```rust
   // managed_path is books_dir/<uuid>.epub — derive <uuid>.cover.<ext> from the same stem.
   let cover_path = meta.cover.as_ref().and_then(|cover| {
       let ext = epub::extension_for(&cover.media_type)?;
       let path = managed_path.with_extension(format!("cover.{ext}"));
       fs::write(&path, &cover.bytes).ok()?;
       Some(path.to_string_lossy().into_owned())
   });
   ```

   Best-effort, like extraction in Step 10: an unwritable cover means a bookless cover,
   never a failed import.

4. **Extend all three cleanup sites** — the same pattern each time, applied to the cover
   path alongside the epub path. The exact changes, site by site:

   - *Failed import.* The cover is written inside the fallible closure, so a
     `let` in there is invisible to the `Err` arm. Hoist the binding: declare
     it mutable before the closure, *assign* inside. The closure is invoked
     immediately, so its mutable capture of `cover_path` ends before the
     `match` reads it — no borrow conflict:

     ```rust
     let mut cover_path: Option<String> = None;

     let result = (|| -> Result<Book, Box<dyn std::error::Error>> {
         // ...
         cover_path = meta.cover.as_ref().and_then(|cover| { /* unchanged */ });
         // ...
     })();

     match &result {
         Err(_) => {
             cleanup_managed_file(&managed_path);
             if let Some(cover) = &cover_path {
                 cleanup_managed_file(Path::new(cover));
             }
         }
         // ...
     ```

   - *Successful re-import.* The pre-query grows from one column to a `(path,
     cover_path)` tuple — rusqlite maps a two-column row through a closure
     that builds the pair:

     ```rust
     let previous: Option<(String, Option<String>)> = self
         .conn
         .query_row(
             "SELECT path, cover_path FROM books WHERE source_path = ?1",
             params![&source_path_text],
             |row| Ok((row.get(0)?, row.get(1)?)),
         )
         .optional()?;
     ```

     and the `Ok` arm destructures and cleans both:

     ```rust
     Ok(_) => {
         if let Some((previous_path, previous_cover)) = previous {
             cleanup_managed_file(Path::new(&previous_path));
             if let Some(cover) = previous_cover {
                 cleanup_managed_file(Path::new(&cover));
             }
         }
     }
     ```

   - *Remove.* Same tuple shape on the `DELETE`:

     ```rust
     let removed: Option<(String, Option<String>)> = self
         .conn
         .query_row(
             "DELETE FROM books WHERE id = ?1 RETURNING path, cover_path",
             params![id],
             |row| Ok((row.get(0)?, row.get(1)?)),
         )
         .optional()?;
     ```

     then clean the epub path and, `if let Some`, the cover.
     `cleanup_managed_file` already tolerates `NotFound`, which covers rows
     imported before this step (their `cover_path` is `NULL`, so the `if let`
     simply skips them).

   *Reimport gotcha the test will catch:* the upsert's `DO UPDATE SET` list
   must also include `cover_path = excluded.cover_path`. Without it the
   conflicting row keeps the *old* cover path, `RETURNING` hands it back, and
   the freshly written new cover file leaks —
   `reimport_replaces_the_cover_without_leaking_files` fails on both
   `assert_ne!` and the `files == 2` count.

#### Why it works

- **The lifecycle is symmetric by construction.** Every place that creates, replaces, or
  destroys the managed `.epub` now does the identical dance for the cover — no new
  concepts, the Steps 7–9 pattern applied to a second file. That symmetry is what makes
  option B's bookkeeping safe: you can audit it by checking that the two paths always
  travel together.
- **`Option` composes down the whole chain.** No cover extracted (Step 10), no known
  extension, or a failed write all collapse into `cover_path: NULL` via the same
  `and_then`/`?` shape — one code path handles every "no cover" cause, and SQLite's
  nullable column mirrors `Option<String>` exactly (rusqlite maps the two for free).
- **`with_extension` on the shared stem** means the pair is visibly related in the books
  dir (`<uuid>.epub` / `<uuid>.cover.jpg`) and re-import's fresh UUID automatically gives
  the fresh cover a fresh name — no overwrite hazard while the old row still points at
  the old file.

#### Scope note

No serving, no UI — `cover_path` is written and cleaned up but never read back except by
tests. Thumbnail *downscaling* (the `image` crate) is deliberately not here: covers are
stored at original size until list memory/latency is a felt problem (ADR-0002
discipline). 11b reads the column.

> **Status:** done — committed in `d1ed76b` (28 tests green, including the
> three cover-lifecycle tests; the pre-existing reimport leak test updated to
> expect the epub + cover pair).

---

### Step 11b — serve covers, render thumbnails

#### Runnable check first

Two layers, like every UI step:

- **Unit test** for the one pure, security-relevant piece — the file-name sanitizer that
  keeps the covers route from serving arbitrary disk paths:

  ```rust
  #[test]
  fn covers_route_only_serves_bare_file_names() {
      assert_eq!(sanitized_file_name("abc.cover.jpg"), Some("abc.cover.jpg".to_string()));
      assert_eq!(sanitized_file_name("../library.sqlite3"), None);
      assert_eq!(sanitized_file_name("a/b.jpg"), None);
      assert_eq!(sanitized_file_name(""), None);
  }
  ```

- **`dx serve` eyeball:** thumbnails appear next to title/author for freshly imported
  books *and* (after a re-import) for books imported before 11a ran; a book with no cover
  renders its row without an `<img>` and without a broken-image icon; remove takes the
  thumbnail away; restart keeps them. `cargo clippy` clean.

#### Minimal implementation sketch

1. **Expose the books dir:** a `books_dir(&self) -> &Path` getter on `Library` (the
   handler needs the directory, and `App` shouldn't recompute `ProjectDirs`).
2. **An app-level route,** registered once in `App` (not per-book like the reader's):

   ```rust
   pub(crate) fn use_register_covers_handler(books_dir: PathBuf) {
       use_asset_handler("covers", move |request, responder| {
           let name = request.uri().path().rsplit('/').next().unwrap_or_default();
           let Some(name) = sanitized_file_name(name) else { /* 404 */ };
           match std::fs::read(books_dir.join(&name)) {
               Ok(bytes) => { /* respond with content_type_for(&name) */ }
               Err(_) => { /* 404, same shape as the epub handler */ }
           }
       })
   }
   ```

   `sanitized_file_name`: accept only names whose `Path::file_name()` equals the whole
   input (that single check defeats `../`, nested paths, and empty names at once).
3. **The `<img>` in `LibraryBooks`:** derive the URL from the stored path's file name —

   ```rust
   if let Some(cover) = book.cover_path.as_deref() {
       if let Some(name) = Path::new(cover).file_name().and_then(|n| n.to_str()) {
           img { src: "/covers/{name}", width: "48px" }
       }
   }
   ```

   Confirm under `dx serve` that a root-relative `src` resolves through the custom
   protocol to the `covers` route (the reader's `EPUB_URL_PREFIX` shows the expected
   final shape: `dioxus://index.html/covers/<name>`); if the relative form doesn't route,
   use the absolute prefix exactly as the EPUB constants do.

#### Why it works

- **Asset handlers are the desktop's "serve me bytes" primitive.** The webview can't read
  arbitrary `file://` paths from the app origin — that sandbox is a feature. A
  `use_asset_handler` route is the sanctioned hole: request comes in on the app protocol,
  your closure answers from disk. The reader already proved the pattern for chapter
  resources; this reuses it at app scope, which is why it's registered in `App` (the
  library screen exists before any book is open).
- **Sanitize by structure, not by blocklist.** `file_name() == whole input` is a
  whitelist-shaped check (same lesson as R6's fragment whitelist): you can't forget an
  escape sequence you never accept. The route can only ever serve immediate children of
  `books_dir`.
- **The browser does the caching.** Serving over a URL (vs data URLs in the DOM) means
  the webview fetches each cover once and caches it like any image — list re-renders
  don't re-touch the bytes at all, which is the concrete payoff option B was chosen for.

#### Scope note

Thumbnails are full-size images scaled by the `width` attribute — real downscaling at
import stays deferred until it hurts. The route serves anything in `books_dir`, including
`.epub` files, to anyone who guesses a UUID name — harmless in a local desktop app, worth
an extension check the day this meets a real network. Styling beyond a bare `width` waits
for the theming phase.

> **Status:** done — committed in `1020159` (29 tests green, including the
> sanitizer test; desktop eyeball confirmed thumbnails, placeholder jacket for
> cover-less books, removal, and restart persistence). Went beyond the sketch:
> the styling deferred above landed here anyway — a responsive cover grid, a
> spine-crease mask effect, and a placeholder jacket (`placeholder-2.jpg`,
> title/author overlaid) instead of an `<img>`-less row. Note: the placeholder
> overlay's `position: absolute` resolves against `.book-cover` only because
> its `drop-shadow` filter creates a containing block — add
> `position: relative` if that filter ever goes away.

> **Follow-up (unplanned):** `7b74a76` made the cover itself the click target — the
> whole `.book-cover` became a `button` whose handler runs the same `Epub::open` →
> `open_status`/`open_book` flow the old Open control used, replacing the separate
> button. UI-only; verified by eyeball. Step 12's item **c** (move `load_spine` out of
> `Reader`) now applies to *this* handler.

---

## Step 12 — review & refactor (phase closer)

The feature steps got the library *working*; this step makes it *good*. Nothing here
changes behavior — which is exactly what makes it safe to do boldly.

### Runnable check — a safety net, not a target

Refactoring must not change behavior, so the existing suite **is** the spec:

- `cargo test` — 29 green before, 29+ green after (item **b** adds one test; item **a**
  may reshape a few, but every behavior they pinned stays pinned).
- `cargo clippy` — clean before and after (the `block v0.1.6` future-incompat note is
  transitive and stays).
- One `dx serve` eyeball at the end: import → list → open → page → chapter nav → TOC link
  → close → remove. Identical behavior, tidier insides.

Run the suite after **each** punch-list item, not once at the end — that's what makes a
refactor a sequence of safe moves instead of one big leap.

### Punch-list

*(suggested order a → b → c → d; e rides along wherever you're already editing)*

#### a. Delete the test-only `Library::add` and share the test setup

`add` (`library.rs:75`) is `#[cfg(test)]` scaffolding from Steps 1–6: a path-only API that
production no longer has, with an `ON CONFLICT(path)` clause that *diverges* from
production's `ON CONFLICT(source_path)`. The three oldest tests are therefore exercising
conflict behavior the app can't reach. Rework `add_then_list_round_trips_books`,
`file_backed_library_survives_reopen_and_reimport_is_idempotent`, and
`remove_drops_the_row_and_is_a_noop_for_unknown_ids` to seed through `add_from_path`
(copy the `crate::BOOK` fixture to two differently-named temp sources, as the newer tests
already do), then delete `add` outright.

While you're in there: the newer tests repeat the same five setup lines (tempdir →
books_dir → copy fixture → open library) five times. Pull a helper into `mod test`, e.g.

```rust
fn library_with_source(dir: &tempfile::TempDir) -> (Library, PathBuf) {
    let books_dir = dir.path().join("books");
    std::fs::create_dir_all(&books_dir).expect("books dir");
    let library =
        Library::open(dir.path().join("library.sqlite3"), &books_dir).expect("library opens");
    let source = dir.path().join("holmes-source.epub");
    std::fs::copy(crate::BOOK, &source).expect("fixture source");
    (library, source)
}
```

*(The tempdir stays owned by the test — if the helper created it, it would be dropped —
and deleted — when the helper returned.)*

**Why.** Tests are the safety net for items b–d, so strengthen the net first. A test-only
method with test-only SQL is worse than dead code: it's *live* code asserting the wrong
contract. And a setup helper isn't just shorter — it makes each test read as *only* the
lines that differ, which is what makes a failure diagnosable at a glance.

#### b. R3 — a real error type with `thiserror` (the "tidy error handling" half)

Two changes in opposite directions:

- **`read_metadata` is infallible — drop its `Result`.** Its body contains no `?` and no
  `Err`: a missing title falls back to `"Untitled"`, a missing author is `None`. The
  signature `-> Result<BookMeta, Box<dyn Error>>` promises failures that cannot happen,
  and every caller pays an `?`/`expect` tax for them. Make it `-> BookMeta` and delete the
  handling at the call sites.
- **`add_from_path` gets a matchable enum.** Add `thiserror = "2"` and, in `library.rs`:

  ```rust
  #[derive(Debug, thiserror::Error)]
  pub(crate) enum LibraryError {
      #[error("could not copy the book into the library: {0}")]
      Io(#[from] std::io::Error),
      #[error("library database error: {0}")]
      Db(#[from] rusqlite::Error),
      #[error("failed to read the EPUB: {0}")]
      Epub(#[from] /* rbook's error type — check docs.rs/rbook for the pinned 0.7 path */),
  }
  ```

  `add_from_path` returns `Result<Book, LibraryError>`; every `?` in its body keeps
  compiling because `#[from]` generates the conversions. `remove` stays on
  `rusqlite::Result` (cleanup failures are logged, not returned — recorded in Step 9).

New test — the check that the box became matchable:

```rust
#[test]
fn import_of_a_missing_source_is_a_matchable_io_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, _source) = library_with_source(&dir);
    let err = library
        .add_from_path(Path::new("/no/such/book.epub"))
        .unwrap_err();
    assert!(matches!(err, LibraryError::Io(_)), "got {err:?}");
    assert!(!err.to_string().is_empty());
}
```

**Why.** `Box<dyn Error>` can only be *displayed*; an enum can be *matched* — and matching
is what a UI needs to choose between "that file isn't an EPUB" and "the disk is full".
`thiserror` derives `Display` (the `#[error]` strings), `std::error::Error` (with
`source()` wired), and the `From` impls `?` relies on — the machinery `anyhow` hides,
written out where you can see it. And shrinking `read_metadata`'s signature teaches the
inverse lesson: a `Result` that can't fail is as misleading as a panic that can.

#### c. Move the last fallible open out of `Reader`

`Reader` still holds a panic on the user path: `load_spine(&epub).expect("bundled epub
should load")` (`main.rs:154`) — a stale message from the `const BOOK` era, now reachable
by any imported file whose container opens but whose spine doesn't read. The Open handler
in `LibraryBooks` already has the right pattern: fallible work at the click site, failure
into `open_status`. Finish the job — load the spine there too, and let `OpenBook` carry it:

```rust
struct OpenBook {
    id: i64,
    title: String,
    epub: Rc<Epub>,
    docs: Rc<Vec<epub::SpineDoc>>,
}
```

In the handler, chain `Epub::open(&path)` → `epub::load_spine(&epub)`; either failure sets
`open_status`, success sets the fully-loaded `OpenBook`. `Reader`'s `use_hook` line
disappears — it just clones `book.docs`.

**Why.** This draws the boundary Step 12 is named for: *everything fallible happens at the
edge, where there's a status line to show; past the boundary, components are infallible.*
`Reader` becomes a pure function of an already-loaded book — no `expect`, nothing to go
wrong — which is the same "data first, UI last" shape the whole phase was built on.
(Verification is the end-to-end eyeball; "container opens but spine fails" has no easy
fixture, and the type change itself removes the panic.)

#### d. Shrink `main.rs` — split the UI modules, move the app-dir logic

`main.rs` is ~415 lines wearing four hats. Three moves, all mechanical:

1. **`open_library()` → `library.rs`** as `Library::open_default()`. The
   `ProjectDirs`/data-dir/books-dir logic is the library's own bootstrapping; moving it
   takes the `directories` import out of `main.rs` and puts the path policy next to the
   store it configures.
2. **`src/ui/reader.rs`** — `Reader`, `NavRow`, `use_bridge`, `BridgeMsg` + its test, and
   `BRIDGE_JS`. The bridge pieces are the reader's private plumbing: `pub(crate)` only
   what `App` renders (`Reader`), keep the rest module-private.
3. **`src/ui/library.rs`** — `LibraryBooks`, `ImportControl`, and `OpenBook` (it's the
   value the library screen produces and the reader consumes — either file defensibly owns
   it; put it where it's constructed).

`main.rs` keeps `main`, `App`, the asset `const`s, and the `#[cfg(test)] BOOK` fixture
path (already correctly test-gated — despite earlier notes, there's no dead scaffolding
left to delete, though renaming it `TEST_BOOK` would make the gating obvious at the call
sites). A `src/ui/mod.rs` (or `mod ui { … }` declarations) wires it up.

**Why.** Module boundaries in Rust are privacy boundaries: while everything lives in
`main.rs`, everything can touch everything, and `pub(crate)` is a formality. After the
split, the compiler enforces that only `Reader` is the reader's public surface — the
bridge protocol can change without any other file caring. That's the "module boundary
review" the phase promised, done with `mod` and `pub` instead of comments.

While moving the two mutation handlers, also collapse the duplicated refresh
(`if let Ok(list) = library.list() { books.set(list); }` appears in both Remove and
Import) into one small `refresh_books(library, books)` helper in `ui/library.rs`.

#### e. Naming & hygiene (ride-alongs)

- `injects_page_listener` (`epub.rs:168`) → `inject_page_listener` — it's named like its
  test, breaking the `inject_*` convention of its four siblings.
- Test-name typo: `"database reopnes"` (`library.rs:244`).
- Stale `expect` messages that still say "bundled epub" on paths that now serve
  user-supplied files (the Reader one dies in item c; grep for the rest).

### Scope note

- **R6 stays its own backlog item** (fragment sanitization, case-insensitive content
  types, "Page 1 of 0") — it changes behavior, so it doesn't belong in a refactor pass.
- The startup `expect`s in `Library::open_default` remain: no home directory / unopenable
  DB means the app genuinely can't run, and there's no UI yet to show it. Recorded, not
  hidden.
- Cover thumbnails, content-hash dedupe, and the web-target `read_bytes()` import path
  stay deferred per the phase doc.

> **Status:** in progress — item **a** implemented (uncommitted): `Library::add` and the
> now-unused `open_in_memory` + test-only `BookMeta` import deleted; the three oldest
> tests reseeded through `add_from_path` via the existing `library_with_source` helper,
> which every library test now uses. Because both seeded sources copy the same fixture,
> the round-trip test asserts list *contents* rather than `ORDER BY title` order (equal
> titles have unspecified relative order). The `"database reopnes"` typo from item **e**
> died with the rewrite. 29 tests green, clippy clean. Items **b**–**e** remain.
