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
3. **Import via a native dialog** — `rfd` picker → path → `read_metadata` → `library.add`;
   real DB path via `directories::ProjectDirs`. Eyeball. *(pending)*
4. **Render the library list** — Dioxus view over `library.list()` (title + author). Eyeball.
   *(pending)*
5. **Open a book → reader renders it** — the row selection drives the reader; `const BOOK`
   comes out. End-to-end eyeball. *(pending)*
6. **Review & refactor** — tidy module boundaries and errors, then delete the single-book
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
- **Re-opens the file just to read metadata.** `read_metadata` opens the EPUB, and Step 5 will
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
- **Dedicated module from the start.** `Book`/`Library` live in `src/library.rs`; Step 6 can
  still review the boundary once the import and UI callers reveal what should remain
  `pub(crate)`.

> **Status:** done — committed in `a1d6822` (17 tests green, including
> `library::test::add_then_list_round_trips_books`; `cargo clippy` completed with the
> expected pre-UI dead-code warnings).
</content>
