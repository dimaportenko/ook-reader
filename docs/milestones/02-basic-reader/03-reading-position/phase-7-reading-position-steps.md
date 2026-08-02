# Phase 7 — Reading Position — build log

[← Phase doc](phase-7-reading-position.md)

Per-step test → minimal code → why, appended newest-last. The
[phase doc](phase-7-reading-position.md)'s "Planned steps" checklist is the high-level
index; this file is the detail and the build log.

## The crux

**The thing you see is not a thing you can store.** The page index is derived from
`window.innerWidth` at render time (`page-count.js`, `pagination.css`), so persisting it
restores the wrong spot at any other window size. The durable locator is
`{spine_index, element selector}`, and the page is recomputed on restore.

What makes that cheap: the reader **already** converts an element into a page.
`fragment-scroll.js` computes `Math.round(el.offsetLeft / window.innerWidth)` for TOC
links, reports it as `ook-scroll`, and rides a state machine that hides the iframe until
the jump settles. **Capture** is that conversion backwards (given a page, which element is
first on it?); **restore** is that conversion unchanged, keyed on a selector instead of an
`id` — delivered through the same iframe URL hash, which is present *before the document
parses* and so needs no load-ordering handshake.

Then it's Phase 6's rhythm again: **data first, UI last.** Columns, `Locator`, and store
round-trips get `#[test]`s; the DOM halves are eyeballs, each provable in devtools before
any database is wired to them.

## Step plan

**Step 0 — Import several EPUBs at once** *(prerequisite, folded in mid-phase)*. Not
reading-position work at all — a Feature 2 leftover (`TODO.md`: "multiple selection for book
addition") pulled forward because Step 1's column change wipes the dev database and makes a
whole-shelf re-import unavoidable. `multiple: true` on the file input, a loop that survives
one bad file, one refresh at the end. Eyeball. Numbered `0` so the arc below keeps its
numbering.

Then the reading-position arc proper:

1. **Stamp `added_at`, and keep it stable across re-import** — two schema columns, a `now`
   parameter, and an `ON CONFLICT` clause that refreshes everything except the day the book
   joined the library. `#[test]`.
2. **Recency: `touch_opened` + the sort** — `last_opened_at` on open;
   `ORDER BY COALESCE(last_opened_at, added_at) DESC, title`. `#[test]` + eyeball.
3. **A `Locator` and somewhere to put it** — the `positions` table, `save_position` /
   `position`, cascade on remove. Round-trip `#[test]`.
4. **Report the first element on the current page (JS)** — an `nth-child` chain for the
   first element whose `offsetLeft` lands on the current page. Asset `#[test]` + devtools
   round-trip eyeball.
5. **Bridge the selector into reader state** — `ook-position` → `BridgeMsg::Position` →
   `ReaderData.locator`. Parse `#[test]` + eyeball.
6. **Persist it** — save `{chapter, selector}` per book on every position message. Eyeball.
7. **Resolve a selector back to a page (JS)** — the `ook-sel:` hash prefix +
   `querySelector`. Asset `#[test]` + a hand-set-the-hash devtools eyeball.
8. **Restore on open** — seed chapter + pending target from the stored locator; iframe
   hidden until `ook-scroll` lands. Nav `#[test]`s + the end-to-end eyeball. **Ticks the
   milestone exit criterion.**
9. **Review & refactor** — the pending-state shape, the split page↔element conversion, the
   `Library` API surface, the save path's error handling.

---

## Step 0 — import several EPUBs at once

> **Status:** done — committed in `afa4cb0` (TODO tick in `d451eac`); eyeball-verified, no
> new tests (the behavior under test is a native file panel).

A prerequisite, not a reading-position step. Step 1 deletes the dev database by hand for
the two new `books` columns, then re-imports the whole shelf through a picker that takes
**one file per click**. One wipe × N books is already enough tax to make this worth twenty
minutes. The original plan also counted Step 3, but that was mistaken: a missing
`positions` table can be added with `CREATE TABLE IF NOT EXISTS` against the existing
database, unlike columns on an already-existing table.

It's also honest about where it came from: this is Feature 2's unchecked
`TODO.md` line, "multiple selection for book addition." Phase 6 declined it on purpose
(*"the input does not set `multiple`, so `into_iter().next()` takes the only file"* —
[phase 6 steps](../02-library/phase-6-library-steps.md)). The deferral is being paid off
now because the schema-reset ritual turned it into the phase's worst real annoyance, which
is the condition ADR-0002 asks for before pulling an unlock forward.

**Runnable check — an eyeball, and it can't be anything else.** The behavior under test is
the OS file panel's multi-selection; `rfd` opens a real `NSOpenPanel`, and no `#[test]` can
click it. So the check is a specific script under `dx serve`, not a vague "try it":

1. Click **Import EPUB**. In the panel, ⌘-click **three** `.epub` files and confirm — the
   panel must *let* you select more than one. That alone proves the attribute reached the
   dialog; if it doesn't, nothing below matters.
2. All three appear in the grid, and the status line reads one aggregate message
   (`Imported 3 books`), not three messages racing to overwrite each other.
3. **The partial-failure case, which is the actual design content of this step.** Make a
   decoy: `cp README.md /tmp/not-a-book.epub`. Select it *together with* two real books.
   Expect the two real books imported and a status naming the failure —
   `Imported 2 books, 1 failed` — **not** an empty library and not a panic. A twenty-file
   re-import that aborts on file three is worse than no batch import at all.
4. Select the same three real books again. The grid still shows three rows, not six —
   `ON CONFLICT(source_path)` already makes re-import idempotent, so "re-upload the whole
   library" is a safe habit rather than a duplicate factory.
5. `cargo clippy --all-targets -- -D warnings` stays clean.

**Minimal implementation.** Two edits, both inside `ImportControl` in `src/ui/library.rs`:

1. **The attribute** — one line on the existing `input`:

   ```rust
   input {
       r#type: "file",
       accept: ".epub",
       multiple: true,
       onchange: move |event| { … },
   }
   ```

2. **The loop** — replace the `let Some(file) = … .next() else { return }` early-exit with a
   pass over every file that *counts* instead of *returning*:

   ```rust
   onchange: move |event| {
       let files = event.files();
       if files.is_empty() {
           return; // cancelled the panel — no work, no status change
       }

       let mut imported = 0usize;
       let mut failed = 0usize;

       for file in files {
           // Each file stands alone: one bad EPUB must not cost you the other nineteen.
           match library.add_from_path(&file.path()) {
               Ok(_) => imported += 1,
               Err(_) => failed += 1,
           }
       }

       status.set(Some(match failed {
           0 => format!("Imported {imported} books"),
           _ => format!("Imported {imported} books, {failed} failed"),
       }));

       // One refresh for the whole batch, after the loop — not one per file.
       refresh_books(&library, books);
   },
   ```

**Why it works.**

- **`event.files()` was always plural.** `FormData::files()` returns `Vec<FileData>` in
  Dioxus 0.7; today's `.into_iter().next()` is throwing away a list that was never
  restricted to one element. What actually restricts it is the DOM attribute:
  `dioxus-desktop`'s `FileDialogRequest` carries a `multiple: bool` and branches on it
  between `rfd`'s `pick_file()` and `pick_files()`. So `multiple: true` isn't a hint to the
  browser — on desktop it's the flag that picks which native dialog call is made.
- **Counting instead of `?` is the whole step.** The current handler's `match` has exactly
  one file to be right about, so "stop on error" and "report the error" are the same
  behavior. With N files they diverge, and the batch case wants the loop to *finish*. This
  is why it's its own step: the interesting decision is error policy, not the attribute.
- **Refresh once, after the loop.** `refresh_books` does a `SELECT` and writes a `Signal`,
  and every signal write re-renders the grid. Inside the loop that's N queries and N
  renders, with the intermediate ones showing a half-imported library. Outside, it's one of
  each — the batch lands as a single visible event, which is also what makes the aggregate
  status line truthful.
- **Discarding the returned `Book` is deliberate.** The single-file version put
  `book.title` in the status; for a batch, a count is the readable summary and `refresh_books`
  re-reads the rows anyway. `Ok(_)` says that out loud.

**Scope note.** The picker only — no drag-and-drop onto the window (a `wry::DragDropEvent`
path, a different mechanism), no directory picker (`FileDialogRequest` has a `directory`
flag, but "import a folder" is its own idea with its own recursion and filtering questions),
no progress UI, no per-file error detail beyond the count. Import stays synchronous on the
UI thread: twenty local file copies are fast enough to not need a spawned task, and adding
one would drag `Rc<Library>` across an await point — a real fight, and not this step's.

> **Hand-off to Step 1.** Step 1 changes the signature to
> `add_from_path(&file.path(), now)`. Call `library::now_secs()` **once, above the loop**,
> and pass the same value to every file in the batch — a shelf imported in one gesture
> joined the library at one moment. That also makes Step 2's
> `COALESCE(last_opened_at, added_at) DESC, title` fall back to title *within* a batch,
> which is the tie the phase doc already calls out under "Known constraints."

---

## Step 1 — stamp `added_at`, and keep it stable across re-import

> **Status:** done — committed in `142b84d` (41 tests green).

The smallest possible start, and the one that forces the schema decision while it's still
cheap: give every book the moment it **joined the library**, and prove that re-importing
the same file refreshes its bytes and metadata **without** resetting that moment. No
ordering yet, no UI change beyond one call site.

`last_opened_at` lands in the same schema edit even though nothing writes it until Step 2 —
one table shape, one dev-DB reset, and the row type is complete for the test to assert
against.

> **Before you start — do Step 0 first, then the dev DB reset.** The recorded decision for
> this phase is *no migrator*: `CREATE TABLE IF NOT EXISTS` is a **no-op against an existing
> table**, so an old `library.sqlite3` keeps its old shape and every query naming `added_at`
> fails at runtime with `no such column`. Delete the whole app data dir so the managed copies
> go with the rows rather than becoming orphans, then re-import your books — in one pick,
> which is what [Step 0](#step-0--import-several-epubs-at-once) is for:
>
> ```sh
> rm -rf ~/Library/Application\ Support/com.dimaportenko.ook-reader
> ```
>
> `cargo test` is unaffected — every test builds its own database under `tempfile::tempdir()`.

**Runnable check.** A `#[test]` in `library.rs`, alongside the existing re-import tests. The
injected timestamps are the point: with a real clock both imports land in the same second
and the assertion *cannot fail*, which would make it a decoration rather than a test.

```rust
#[test]
fn import_stamps_added_at_and_reimport_preserves_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source, _) = library_with_source(&dir);

    let first = library.add_from_path(&source, 1_000).expect("first import");

    assert_eq!(first.added_at, 1_000);
    assert_eq!(
        first.last_opened_at, None,
        "a freshly imported book has never been opened",
    );

    // Same source path → same row, fresh bytes and metadata …
    let second = library.add_from_path(&source, 2_000).expect("reimport");

    assert_eq!(second.id, first.id);
    assert_ne!(second.path, first.path);
    // … but the day it joined the library is not "fresh metadata".
    assert_eq!(second.added_at, 1_000);
}
```

**Minimal implementation.** Five small edits, all in `library.rs` except the last:

1. **Schema** (`Library::init`) — two nullable integer columns:

   ```rust
   "CREATE TABLE IF NOT EXISTS books (
       id INTEGER PRIMARY KEY,
       path TEXT NOT NULL UNIQUE,
       source_path TEXT NOT NULL UNIQUE,
       title TEXT NOT NULL,
       author TEXT,
       cover_path TEXT,
       added_at INTEGER NOT NULL,
       last_opened_at INTEGER
   )"
   ```

2. **The row type** — `Book` gains `pub(crate) added_at: i64` and
   `pub(crate) last_opened_at: Option<i64>`, and `read_book` reads indices 5 and 6.
   `read_book` is shared by `list()`'s `SELECT` and `add_from_path`'s `RETURNING`, so both
   column lists have to grow **in the same order** — a coupling worth noticing now and
   worth a shared `const BOOK_COLUMNS: &str` later (Step 9).

3. **The signature** — `add_from_path(&self, source_path: &Path, now: i64)`.

4. **The upsert** — `added_at` is inserted but *absent* from the `DO UPDATE SET` list:

   ```sql
   INSERT INTO books (path, source_path, title, author, cover_path, added_at)
   VALUES (?1, ?2, ?3, ?4, ?5, ?6)
   ON CONFLICT(source_path) DO UPDATE SET
       path = excluded.path,
       title = excluded.title,
       author = excluded.author,
       cover_path = excluded.cover_path
   RETURNING id, path, title, author, cover_path, added_at, last_opened_at
   ```

5. **The clock, at the edge** — a helper plus the one UI call site in
   `ui/library.rs`'s `ImportControl`:

   ```rust
   pub(crate) fn now_secs() -> i64 {
       std::time::SystemTime::now()
           .duration_since(std::time::UNIX_EPOCH)
           .map(|elapsed| elapsed.as_secs() as i64)
           .unwrap_or(0) // a clock set before 1970 is not our problem to solve
   }
   ```

   ```rust
   match library.add_from_path(&file.path(), library::now_secs()) { … }
   ```

The existing `library.rs` tests each gain a timestamp argument — pass whatever reads
clearly (`0`, or distinct values where the test is about ordering later).

**Why it works.** Three mechanisms, each worth holding onto:

- **`excluded` is the row you tried to insert.** In an upsert, SQLite exposes the rejected
  candidate as the pseudo-table `excluded`, and `DO UPDATE SET x = excluded.x` copies it
  over the existing row. Preservation therefore isn't an extra clause you write — it's a
  column you *don't mention*. `RETURNING` then hands back the row as it now stands, which is
  why the test can read the preserved `added_at` straight out of the returned `Book` without
  a second query.
- **Passing `now` in is what makes the assertion real.** The store becomes a pure function
  of its inputs: same arguments, same row, no hidden dependency on when the test ran.
  `SystemTime::now()` moves to the one place that genuinely has to know the time — the
  edge where a human clicked something. This is the same "take the general form, let the
  caller decide" inversion R2 applied to `&Epub`, aimed at a different kind of ambient
  dependency.
- **Unix seconds in an `INTEGER`.** SQLite has no date type; an integer sorts correctly,
  carries no timezone, and is what `COALESCE(last_opened_at, added_at) DESC` will compare in
  Step 2. `i64` rather than `u64` because that's what SQLite stores and what `rusqlite`
  maps without a cast at every boundary.

**Scope note.** Nothing writes `last_opened_at` yet and `list()` still sorts by title —
both are Step 2. Nothing reads position yet — that's Step 3 onward. The only user-visible
change from this step is that the library is empty until you re-import, which is the price
of the recorded no-migrator decision.

---

## Step 2 — recency: `touch_opened` + the sort

> **Status:** done — committed in `d4f2e7b` (42 tests green; shelf reordering
> eyeball-verified).

Step 1 gave every book a timestamp nothing reads. This step makes both timestamps *matter*:
write `last_opened_at` when a book is opened, and sort the library by "the last time this
book and I had anything to do with each other." It's the first step in the phase with a
visible payoff — the shelf reorders itself around what you're actually reading — and it's
still pure store work plus one call site, so it stays small.

The one real design decision is what a **never-opened** book sorts as. `ORDER BY
last_opened_at DESC` puts every fresh import at the *bottom*, under books you read months
ago — exactly backwards, since the book you just imported is the one you most likely want.
`COALESCE(last_opened_at, added_at)` says the right thing instead: fall back to the day it
joined the library, so an import is "recent activity" until the first time you open it, and
after that its own opens take over.

**Runnable check.** A `#[test]` in `library.rs`. Unlike the earlier tests this one asserts
**order**, so give the two books distinct timestamps — with a real clock they'd tie and
`ORDER BY title` (equal titles, same fixture) would decide, making the assertion a coin
flip. This is the same reason Step 1 injected `now`, now paying off a second time.

```rust
#[test]
fn opening_a_book_floats_it_to_the_top_of_the_list() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, first_source, _) = library_with_source(&dir);
    let second_source = dir.path().join("holmes-second-source.epub");
    std::fs::copy(crate::TEST_BOOK, &second_source).expect("second fixture source");

    let older = library
        .add_from_path(&first_source, 1_000)
        .expect("first import");
    let newer = library
        .add_from_path(&second_source, 2_000)
        .expect("second import");

    // Nothing opened yet: `added_at` stands in, so the newest import leads.
    let ids: Vec<i64> = library.list().expect("list").iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![newer.id, older.id]);

    // Open the *older* import, later than either import moment.
    let touched = library.touch_opened(older.id, 3_000).expect("touch succeeds");
    assert!(touched, "an existing row reports true");

    let books = library.list().expect("list");
    assert_eq!(books[0].id, older.id, "the book you just read leads");
    assert_eq!(books[0].last_opened_at, Some(3_000));
    assert_eq!(
        books[1].last_opened_at, None,
        "the book you didn't open is untouched",
    );

    // Unknown id: no error, no change — same contract as `remove`.
    let missing = library
        .touch_opened(-1, 4_000)
        .expect("missing id is Ok(false)");
    assert!(!missing);
}
```

Watch it fail first for the right reason: `touch_opened` doesn't exist, so this is a
compile error, not a wrong-order failure. Once it compiles, the *ordering* assertions are
what the step is actually about.

Then the eyeball under `dx serve`: open a book that isn't first in the grid, close it, and
it's now first. Plus `cargo clippy --all-targets -- -D warnings`.

**Minimal implementation.** Three edits:

1. **`touch_opened`** in `library.rs`, next to `remove` (whose `bool` convention it copies):

   ```rust
   pub(crate) fn touch_opened(&self, id: i64, now: i64) -> rusqlite::Result<bool> {
       let updated = self.conn.execute(
           "UPDATE books SET last_opened_at = ?2 WHERE id = ?1",
           params![id, now],
       )?;

       Ok(updated == 1)
   }
   ```

2. **The sort** in `list()` — the `ORDER BY` clause only, the `SELECT` is unchanged:

   ```sql
   ORDER BY COALESCE(last_opened_at, added_at) DESC, title
   ```

3. **The call site**, in `LibraryBooks`'s cover `onclick` (`src/ui/library.rs:50`), inside
   the `Ok(…)` arm — you record having read a book only *after* it actually opened. The
   closure needs its own handle, so clone the `Rc` in the same `{ … }` prelude that already
   captures `id`/`title`/`path`, the way the Remove button does:

   ```rust
   onclick: {
       let library = Rc::clone(&library);
       let id = book.id;
       // … title, path as today …

       move |_| {
           match open_epub(std::path::Path::new(&path)) {
               Ok((epub, docs)) => {
                   open_status.set(None);
                   // Best-effort: failing to record the visit must not block the read.
                   let _ = library.touch_opened(id, library::now_secs());
                   refresh_books(&library, books);
                   open_book.set(Some(OpenBook { … }));
               }
               Err(error) => open_status.set(Some(format!("Open failed: {error}"))),
           }
       }
   },
   ```

**Why it works.**

- **`COALESCE(a, b)` returns the first non-`NULL` argument**, evaluated per row inside the
  `ORDER BY`. So each row sorts on a single "last activity" number without you storing one:
  opened books sort by their open, never-opened books by their import. SQLite is happy to
  sort by an expression — no extra column, no view, nothing to keep in sync. The cost is
  that the expression isn't indexable as written; at library sizes (hundreds of rows, a
  local file) that is not a real cost, and noticing *why* it's fine is more useful than
  pre-optimizing it.
- **`, title` is the tiebreaker, and it's load-bearing.** Two books imported in the same
  gesture share a timestamp (Step 0's hand-off deliberately passes one `now_secs()` to the
  whole batch), so without a second key SQLite may return them in any order — and "any
  order" is allowed to *change* between runs, which reads as a flickering shelf.
- **`Connection::execute` returns the number of rows it changed**, which is what turns a
  bare `UPDATE` into a usable answer: `Ok(updated == 1)` distinguishes "recorded" from
  "there's no such book" without a preceding `SELECT`. `remove` already reports `bool` the
  same way, so the `Library` API stays consistent — worth keeping in mind for Step 9's API
  review.
- **`refresh_books` *before* `open_book.set`** — both writes happen in one event handler,
  and Dioxus batches them into a single re-render, so the reordered grid is simply already
  correct behind the reader when you close it. (Writing signals from an event handler is
  fine; writing one during render is the infinite-loop trap.)
- **`let _ = …` on the touch is deliberate**, and matches the phase doc's constraint about
  the save path: the errors here are still `Box<dyn Error>`-era plumbing, and a book that
  opens fine should not refuse to open because a bookkeeping `UPDATE` failed. **R3**
  (`thiserror`) in the [review backlog](../review-2026-07-steps.md) is where that gets a
  real answer.

**Scope note.** Recency only — this step records *that* you opened a book, not *where* you
were in it; the `positions` table and the `Locator` are Step 3. No relative-date UI ("2 days
ago"), no user-chosen sort order, and no "currently reading" section. The existing tests
need no edits: they assert list *contents* (`contains`, or a one-element `vec![…]`), which
is exactly why they survive a sort change — a useful thing to notice about how they were
written.

---

## Step 3 — a `Locator` and somewhere to put it

> **Status:** done — committed in `d5dc134` (44 tests green). Two `dead_code` warnings are
> expected and accepted until Step 6 gives `Locator`, `save_position` and `position` their
> first caller — a knowing exception to this phase's "clippy clean" bar, because the step's
> whole point is that nothing consumes the store yet.

This step establishes the persistence boundary before any DOM code exists. A locator is
owned Rust data — a spine index plus a selector — and the database keeps exactly one of
them per book. Saving again replaces the old value; deleting the book deletes its locator.
Nothing in the reader uses it yet, which is deliberate: the store contract can be proved in
isolation before JavaScript starts producing selectors in Steps 4–5.

There is one SQLite trap worth making explicit: writing `REFERENCES books(id) ON DELETE
CASCADE` in the schema is not enough. SQLite foreign-key enforcement is disabled by
default **per connection**. `Library::init` must enable it, or the cascade test leaves an
orphan row while the schema looks perfectly correct.

> **Correction to the upfront plan.** Do **not** delete the dev database for this step.
> Step 1 needed a reset because `CREATE TABLE IF NOT EXISTS books` cannot add columns to an
> existing table. Step 3 adds a whole new table, so `CREATE TABLE IF NOT EXISTS positions`
> safely creates it the next time the app opens. Schema initialization is enough here; a
> migration is not.

**Runnable checks.** Add two `#[test]`s in `src/library.rs`. The first fixes the
save/read/latest-wins contract; the second proves the foreign key is really active rather
than merely written in the DDL.

```rust
#[test]
fn position_round_trips_and_latest_save_wins() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source, _) = library_with_source(&dir);
    let book = library.add_from_path(&source, 1_000).expect("import");

    assert_eq!(library.position(book.id).expect("empty position"), None);

    let first = Locator {
        spine_index: 2,
        selector: "body > p:nth-child(3)".to_string(),
    };
    library
        .save_position(book.id, &first, 2_000)
        .expect("first save");
    assert_eq!(library.position(book.id).expect("first read"), Some(first));

    let latest = Locator {
        spine_index: 4,
        selector: "body > div:nth-child(2) > p:nth-child(7)".to_string(),
    };
    library
        .save_position(book.id, &latest, 3_000)
        .expect("latest save");
    assert_eq!(
        library.position(book.id).expect("latest read"),
        Some(latest),
    );

    // `updated_at` is intentionally not part of `Locator`, but the injected clock is
    // still a storage contract worth proving inside this module-level test.
    let updated_at: i64 = library
        .conn
        .query_row(
            "SELECT updated_at FROM positions WHERE book_id = ?1",
            params![book.id],
            |row| row.get(0),
        )
        .expect("stored timestamp");
    assert_eq!(updated_at, 3_000);
}

#[test]
fn removing_a_book_cascades_to_its_position() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source, _) = library_with_source(&dir);
    let book = library.add_from_path(&source, 1_000).expect("import");
    let locator = Locator {
        spine_index: 2,
        selector: "body > p:nth-child(3)".to_string(),
    };

    library
        .save_position(book.id, &locator, 2_000)
        .expect("save position");
    assert!(library.remove(book.id).expect("remove book"));
    assert_eq!(library.position(book.id).expect("position lookup"), None);
}
```

Run the focused checks while iterating:

```sh
cargo test position_round_trips_and_latest_save_wins
cargo test removing_a_book_cascades_to_its_position
```

Watch the first one fail to compile because `Locator`, `save_position`, and `position` do
not exist. After the implementation, run the full safety net:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

**Minimal implementation.** All edits are in `src/library.rs`.

1. Add the owned value type beside `Book`:

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub(crate) struct Locator {
       pub(crate) spine_index: usize,
       pub(crate) selector: String,
   }
   ```

   `usize` matches the reader's spine indexing; `String` is owned because the value crosses
   the DOM → Rust → SQLite boundary and must outlive any message buffer.

   **`usize` needs a Cargo feature.** rusqlite gates the `ToSql`/`FromSql` impls for
   `usize` and `u64` behind `fallible_uint` (off by default) — SQLite's only integer type is
   `i64`, and unlike every narrower type those conversions can fail in both directions.
   Without the feature, the `params!` in item 3 is a compile error: *the trait bound `usize:
   rusqlite::ToSql` is not satisfied*. Add it to `Cargo.toml` before writing the store:

   ```toml
   rusqlite = { version = "0.40", features = ["bundled", "fallible_uint"] }
   ```

2. At the start of `Library::init`, enable foreign keys for this connection:

   ```rust
   conn.pragma_update(None, "foreign_keys", true)?;
   ```

   Then, after creating `books`, create the dependent table:

   ```sql
   CREATE TABLE IF NOT EXISTS positions (
       book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
       spine_index INTEGER NOT NULL,
       selector TEXT NOT NULL,
       updated_at INTEGER NOT NULL
   )
   ```

3. Add the latest-wins write:

   ```rust
   pub(crate) fn save_position(
       &self,
       book_id: i64,
       locator: &Locator,
       now: i64,
   ) -> rusqlite::Result<()> {
       self.conn.execute(
           "INSERT INTO positions (book_id, spine_index, selector, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(book_id) DO UPDATE SET
                spine_index = excluded.spine_index,
                selector = excluded.selector,
                updated_at = excluded.updated_at",
           params![book_id, locator.spine_index, locator.selector.as_str(), now],
       )?;

       Ok(())
   }
   ```

4. Add the optional read. `OptionalExtension` is already imported for `remove`, so no new
   import is needed:

   ```rust
   pub(crate) fn position(&self, book_id: i64) -> rusqlite::Result<Option<Locator>> {
       self.conn
           .query_row(
               "SELECT spine_index, selector FROM positions WHERE book_id = ?1",
               params![book_id],
               |row| {
                   Ok(Locator {
                       spine_index: row.get(0)?,
                       selector: row.get(1)?,
                   })
               },
           )
           .optional()
   }
   ```

**Why it works.**

- **`book_id PRIMARY KEY` expresses one position per book.** It is both the identity of the
  row and the conflict target. The upsert therefore has only two outcomes: insert the first
  locator or overwrite that book's existing locator. There is no history to accidentally
  accumulate and no preliminary `SELECT` race.
- **`excluded` is the attempted new row**, the same mechanism Step 1 used for re-import.
  Here every mutable position field is copied from it because "latest wins" is the whole
  contract. `updated_at` changes in the same statement, so locator and timestamp cannot
  disagree halfway through a save.
- **`OptionalExtension::optional()` changes only `QueryReturnedNoRows` into `None`.** Real
  database failures remain `Err`; "this book has no saved position yet" is ordinary state,
  not an error. That gives Step 8 a natural fallback to chapter 0/page 0.
- **The cascade is database-owned cleanup.** `remove` already deletes the `books` row;
  with foreign keys enabled, SQLite deletes the dependent `positions` row in the same
  operation. Putting this in the schema avoids a second manual delete path that could be
  forgotten or partially fail.
- **The clock stays at the edge.** `save_position` takes `now`, just like `add_from_path`
  and `touch_opened`, so Step 6 can call `now_secs()` while this test supplies exact values.
  The store remains deterministic.
- **`fallible_uint` keeps `usize` out of the call sites.** The alternatives were an
  `i64`/`u32` field with casts at every crossing, or `as` casts at the SQL boundary. Both
  lose, for the same reason: every existing spine index in the codebase is already a `usize`
  — `ReaderData.chapter`, `chapter_count`, `nav.rs`'s arithmetic, and
  `epub::LinkTarget.spine_index`, which `follow_link` assigns straight into `chapter()`
  (`src/nav.rs:88`). A non-`usize` `Locator` would be the only spine index that needs
  converting, at three sites, and the conversion it relocates is `i64 as usize` on restore —
  the direction where a corrupt row becomes a huge index instead of an error. With the
  feature on, rusqlite runs `i64::try_from` on write and a checked narrowing on read, so a
  bad value surfaces as `FromSqlError::OutOfRange` at the boundary that owns it.

**Scope note.** Storage only. Do not change `ReaderData`, bridge messages, injected assets,
or the open-book handler yet. Step 4 discovers a selector in JavaScript, Step 5 carries it
into Rust state, and Step 6 calls `save_position`. Re-import can still leave a locator that
no longer matches the replacement EPUB; best-effort restore in Step 8 owns that behavior.

---

## Step 4 — report the first element on the current page (JS)

The store has a shape to fill; nothing produces a selector yet. This step adds the injected
asset that answers *"which element starts the page I'm looking at?"* and posts it to the
parent as `ook-position`. Nothing listens for that message yet — Step 5 teaches
`ook-events-listener.js` to forward it — so this step is provably inert in the app and
verified in devtools.

The measurement is the crux, and `pagination.css` is what makes it work. `body` is a
multi-column container whose `column-width` plus `column-gap` add up to exactly `100vw`, and
paging is `transform: translateX(var(--ook-page) * -100vw)`. A CSS transform is a *paint*
operation — it does not move layout boxes. So `el.offsetLeft` keeps reporting the element's
position in the untransformed column flow no matter which page is on screen, and
`Math.round(el.offsetLeft / window.innerWidth)` is a stable page index. That is the same
measurement `fragment-scroll.js:17` already makes, now run in the other direction.

The tempting alternative, `getBoundingClientRect().left`, *does* include the transform: it
measures against the viewport, so it answers "where is this on screen right now" and changes
every time you turn a page. Both are correct measurements of different questions; this step
needs the layout one.

**Runnable checks.** Two, of different kinds — say which is proving what.

1. **A Rust `#[test]` proves injection, not behavior.** `serving_a_chapter_injects_the_reader_assets`
   in `src/epub.rs` already asserts one marker string per asset. Add a line for this one:

   ```rust
   assert!(xhtml.contains("ook-position")); // page-position.js
   ```

   It fails until the asset exists and is listed in `INJECTED_ASSETS`. That is genuinely all
   a Rust test can prove here — the JS never executes under `cargo test`.

2. **A devtools round-trip proves the behavior.** Under `dx serve`, open a book, open
   devtools, and switch the console's context to the chapter iframe (the context dropdown —
   the frame is the blob/`epub://` document, not the top page). Then:

   ```js
   // The selector must find its way back to the element it was built from.
   const el = firstElementOnPage(currentPage());
   const sel = selectorFor(el);
   [sel, document.querySelector(sel) === el];
   // → ["body > div:nth-child(2) > p:nth-child(7)", true]
   ```

   Turn a page and run it again — the selector should change and still round-trip `true`.
   To see the message itself, run this in the **parent** context first, then turn pages:

   ```js
   addEventListener("message", (e) => e.data?.kind === "ook-position" && console.log(e.data.selector));
   ```

Then `cargo test` and `cargo clippy --all-targets -- -D warnings`.

**Minimal implementation.** A new asset plus one line registering it.

1. **`src/web/assets/page-position.js`** — named for its neighbours `page-count.js` /
   `page-listener.js`, which is the same "one small fact about the current page" family:

   ```js
   // Which page an element lives on. `offsetLeft` is layout position inside body's
   // column flow; the `translateX` that paginates is paint-only and does not move it,
   // so this is stable no matter which page is currently shown.
   function pageOf(el) {
     return Math.round(el.offsetLeft / window.innerWidth);
   }

   // `body > div:nth-child(2) > p:nth-child(7)`, built leaf-first and unshifted so the
   // finished chain reads root-to-leaf.
   function selectorFor(el) {
     const parts = [];
     while (el && el !== document.body) {
       const parent = el.parentElement;
       if (!parent) return null; // detached from the document: no path back to body
       const index = Array.prototype.indexOf.call(parent.children, el) + 1;
       parts.unshift(`${el.localName}:nth-child(${index})`);
       el = parent;
     }
     return ["body", ...parts].join(" > ");
   }

   function firstElementOnPage(page) {
     for (const el of document.body.getElementsByTagName("*")) {
       if (!el.getClientRects().length) continue; // no box: display:none, <style>, …
       if (pageOf(el) === page) return el;
     }
     return null;
   }

   function reportPosition(page) {
     const el = firstElementOnPage(page);
     if (!el) return; // an empty or fully-hidden page has nothing to anchor to
     const selector = selectorFor(el);
     if (!selector) return;
     window.parent.postMessage({ kind: "ook-position", selector: selector }, "*");
   }

   // The page we are on right now, read back from the variable pagination.css
   // paginates by — same read `fragment-scroll.js` does.
   function currentPage() {
     const style = getComputedStyle(document.documentElement);
     return Number(style.getPropertyValue("--ook-page")) || 0;
   }

   window.addEventListener("load", () => reportPosition(currentPage()));
   window.addEventListener("message", function (e) {
     if (!e.data || e.data.kind !== "ook-set-page") return;
     reportPosition(e.data.page);
   });
   ```

2. **Register it** in `src/web/assets.rs`, after `fragment-scroll.js`:

   ```rust
   wrap_js!("./assets/page-position.js"),
   ```

**Why it works.**

- **`nth-child` is 1-based and counts element siblings only**, which is why the index comes
  from `parent.children` (an `HTMLCollection` of elements) and not `parent.childNodes` (which
  includes the whitespace text nodes between tags). Get that wrong and every selector in a
  prettily-indented EPUB is off by one — and it would still *look* plausible.
- **`el.localName`, not `el.tagName`.** In an HTML document `tagName` is upper-cased
  (`"P"`), and while CSS type selectors are case-insensitive in HTML, they are **not** in
  XHTML — which is exactly what these documents are served as
  (`application/xhtml+xml`). `localName` gives you `"p"` in both.
- **The tag name is redundant but load-bearing.** `body > :nth-child(2)` would select the
  same element; writing `body > div:nth-child(2)` makes the selector *self-checking* — if the
  document ever changes shape, it fails to match instead of silently matching a different
  element. A wrong restore is worse than a failed one, because Step 8 can fall back from a
  failure.
- **`getClientRects().length` is the "does this element have a box" test.** A `<style>` in
  the body, a `display: none` wrapper, or an unrendered element all report `offsetLeft === 0`
  and would otherwise claim to be the first element on page 0. Checking for a layout box is
  cheaper and more direct than reasoning about which tags can appear.
- **`getElementsByTagName("*")` yields document order**, so the first match is the earliest
  element on that page — the natural anchor. A container that *starts* on page 0 and spans to
  page 3 has its `offsetLeft` on page 0, so it correctly loses to its own child that actually
  begins page 3.
- **The page comes from `e.data.page`, not from re-reading `--ook-page`.** Both
  `page-listener.js` and this asset handle the same `ook-set-page` message, and listeners fire
  in registration order — which here is injection order in `INJECTED_ASSETS`. Reading the
  variable would silently depend on that ordering staying put; taking the number off the
  message that caused the change depends on nothing. `load` is the one case with no message,
  so it reads the variable.
- **Injection is the only Rust-testable part.** The asset is a string `include_str!`d into a
  `const` — `cargo test` can prove it reaches the served XHTML and nothing more. Being
  explicit that the marker assertion is a *plumbing* test, and that behavior rides on the
  devtools round-trip, is the honest version of "this step is tested."

**Scope note.** The iframe side only. No `BridgeMsg::Position`, no `ook-events-listener.js`
change, no `ReaderData` field, no saving — Steps 5 and 6. Restoring *from* a selector is
Step 7, and it goes in `fragment-scroll.js` rather than here, because reading the hash is
already that asset's job. Also deferred: no debounce (see the phase doc's save-on-every-page
decision), and no handling of a re-imported book whose DOM no longer matches — Step 8 owns
that as best-effort.

One thing to notice for **Step 9's refactor pass**: `currentPage()` is now copy-pasted
between `fragment-scroll.js` and `page-position.js`, and `pageOf` duplicates the
`offsetLeft / innerWidth` arithmetic that `fragment-scroll.js:17` inlines. Three assets now
share the same idea of what a page is. That is fine while each asset stays independently
injectable, but it is the kind of duplication the review step should either extract or
consciously accept.

### ⏱️ Performance: `firstElementOnPage` is a linear scan — to measure at Step 9

`firstElementOnPage` walks **every** element in the chapter body, in document order, reading
`getClientRects()` and `offsetLeft` on each until one lands on the target page. Written for
obvious correctness, not for speed. Recording the concern here so Step 9 tests it rather than
rewrites it on a hunch.

**Why it is probably fine.** Both reads force layout, but the loop never *writes* to the DOM
— so layout is computed once on the first read and every later read hits the clean cached
value. The cost is O(n) property reads, not O(n) reflows, which is the difference between a
fraction of a millisecond and visible jank. It runs once per page turn, not per frame.

**Where it would hurt.** A long chapter (a whole book in one XHTML file — some EPUBs do this)
scanned for a *late* page: the loop has no early exit, so finding page 0 is instant while
finding the last page touches every element. Worst case is the anchor cost growing with how
far into the chapter the reader is.

**The measurement** — under `dx serve`, in the chapter iframe's console, on the longest
chapter available, at the *end* of it:

```js
const p = currentPage();
const t = performance.now();
for (let i = 0; i < 100; i++) firstElementOnPage(p);
(performance.now() - t) / 100; // ms per call
```

Also worth capturing alongside it: `document.body.getElementsByTagName("*").length`, so the
number has a document size attached to it. **Threshold:** under ~1 ms per call on the worst
chapter to hand, leave it alone and delete this section. Above that, or if a page turn ever
*feels* like it hitches, take the cheap fixes first.

**Fixes, cheapest first.**

1. **Early exit.** In a column flow `offsetLeft` is non-decreasing in document order, so once
   `pageOf(el) > page` nothing later can match — `break`. Turns the late-page worst case into
   "scan up to the target", and costs one line.
2. **Narrow the candidate set.** `"*"` includes every `<em>`, `<span>`, `<a>`; a
   `querySelectorAll("p, h1, h2, h3, h4, h5, h6, li, img, blockquote, div, section")` is
   roughly an order of magnitude fewer elements *and* yields a sturdier anchor — a block is a
   better restore target than an inline fragment of one.
3. **`document.elementFromPoint(x, y)`.** The browser's own hit-testing, O(1) from JS: because
   pagination is a `translateX`, the current page's content is in the viewport, so a probe
   point inside the text column returns the topmost element painted there directly. Caveats:
   it returns the *deepest* element (walk up to a block), returns `body`/`html`/`null` if the
   probe lands in a margin, and can only answer for the page currently **painted** — fine for
   `reportPosition`, useless for asking about an off-screen page. Its sibling
   `document.caretPositionFromPoint` returns a text node plus character offset, a finer anchor
   than any element selector, which is the direction a future precise-locator feature wants
   anyway (see the milestone README's deferred "precise, shareable locators").
4. **`IntersectionObserver`.** The idiomatic "which elements are on screen" answer: no forced
   layout from JS at all, the browser reports intersections asynchronously. Rejected for
   *this* step because it is async — it fires after paint, and `reportPosition` wants an
   answer synchronously in the `ook-set-page` handler. It becomes the right tool if position
   tracking ever needs to be continuous rather than one reading per page turn.
5. **Precompute on `load`.** Build `[element, page]` for all block candidates once, then a
   page turn is an array lookup. Needs invalidation on resize and font-size change, since both
   repaginate — the most code and the most state, hence last.

Options 1 and 2 are additive and preserve the current shape; 3–5 are rewrites. Do not reach
past 2 without a number from the measurement above.
