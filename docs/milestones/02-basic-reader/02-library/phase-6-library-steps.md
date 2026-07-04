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
   author }` via `rbook`; `#[test]` against the bundled book. *(pending)*
2. **A persistent library store** — `rusqlite` `Library` with `add`/`list` and a `Book` row
   type; round-trip `#[test]` against a temp/in-memory DB. *(pending)*
3. **Import via a native dialog** — `rfd` picker → path → `read_metadata` → `library.add`;
   real DB path via `directories::ProjectDirs`. Eyeball. *(pending)*
4. **Render the library list** — Dioxus view over `library.list()` (title + author). Eyeball.
   *(pending)*
5. **Open a book → reader renders it** — the row selection drives the reader; `const BOOK`
   comes out. End-to-end eyeball. *(pending)*
6. **Review & refactor** — lift library types into `src/library.rs`, tidy errors, delete the
   single-book scaffolding. *(pending)*

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

> **Status:** pending.
</content>
