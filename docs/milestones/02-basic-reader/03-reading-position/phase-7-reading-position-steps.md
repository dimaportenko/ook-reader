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

> **Status:** done — committed in `4c53520` (44 tests green, injection assertion included);
> devtools round-trip verified — `document.querySelector(selectorFor(el)) === el` returns
> `true`, across page turns.
>
> One bug was caught and fixed before the commit: `selectorFor` built its index from a
> 0-based `indexOf` while `:nth-child()` counts from 1, so a first child produced
> `:nth-child(0)` (matches nothing) and everything else pointed at the *previous* sibling.
> Exactly the failure the round-trip check exists to catch.

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

---

## Step 5 — bridge the selector into reader state

> **Status:** done — committed in `3bd7457` (45 tests green). `on_position` was confirmed
> receiving the selector from the iframe, so the four-file path is verified end to end.
>
> **Two deviations from the plan below, both deliberate.** Item 5 — the temporary readout
> under the nav rows — was skipped; the state write was confirmed directly at `on_position`
> instead, so the throwaway UI never had to be built or removed. And the one-line
> `assert!(BRIDGE_JS.contains("ook-position"))` suggested under "Why it works" was folded
> into the new test, closing the listener half of the cross-file `kind` contract.
>
> Clippy still reports the two Step 3 `dead_code` warnings (`Locator`, `save_position` /
> `position`) — unchanged by this step, cleared when Step 6 calls them.

Step 4 left a selector being shouted into a room with nobody in it: `page-position.js`
posts `{ kind: "ook-position", selector }` to the parent on every page turn, and the parent
ignores it. This step wires the whole existing message path end to end —
`ook-position` → `dioxus.send("position:…")` → `BridgeMsg::Position` → a field on
`ReaderData` — so the current page's anchor is available as ordinary Rust state. Nothing is
saved yet; Step 6 does that.

It is deliberately the smallest possible link in the chain, because the interesting content
is not the plumbing (three files already do exactly this for `link`, `scroll`, and `pages`)
but two decisions the plumbing forces:

**A selector is a colon-rich string.** `body > div:nth-child(2) > p:nth-child(7)` contains
three colons and four spaces. Any parser that reaches for `split(':')` mangles it. The
existing `BridgeMsg::parse` already does the right thing by accident of good taste —
`strip_prefix` returns the *entire* remainder — and the new test's job is to turn that
accident into a promise.

**The field is not a `Locator`.** `library::Locator` is `{ spine_index, selector }`; what
the iframe reports is only the selector half, because the iframe has no idea which spine
entry it is. The spine index is already in `ReaderData.chapter`, and Step 6 pairs them at
save time. So the field is named for what it holds:

> **Rename from the upfront plan.** The phase doc's checklist says `ReaderData.locator`.
> Call it `anchor: Option<String>` instead. At Step 6 both a `library::Locator` and this
> field are in scope in the same function, and giving a bare `String` the name of a
> two-field struct would make that code read wrong. "Anchor" also says what the element
> *is*: the thing the page is pinned to.

**Runnable check.** A `#[test]` next to `bridge_parses_each_message_kind` in
`src/ui/reader.rs`. It fails to compile first (`BridgeMsg::Position` does not exist), which
is the right kind of red.

```rust
#[test]
fn bridge_parses_a_position_selector_whole() {
    // A selector is colon- and space-rich. `strip_prefix` hands back the entire
    // remainder of the message, so nothing here gets split in half.
    assert_eq!(
        BridgeMsg::parse("position:body > div:nth-child(2) > p:nth-child(7)"),
        Some(BridgeMsg::Position(
            "body > div:nth-child(2) > p:nth-child(7)".to_string()
        )),
    );

    // An empty payload is not a position. Reject it here rather than storing a
    // selector that can never resolve — Step 6 writes this straight to SQLite.
    assert_eq!(BridgeMsg::parse("position:"), None);
}
```

```sh
cargo test bridge_parses_a_position_selector_whole
```

Then the eyeball under `dx serve`, which is what proves the *other* three quarters of the
path (the JS branch, the `dioxus.send`, the state write) — none of which `cargo test` can
reach. Item 5 of the implementation adds a temporary readout under the nav rows:

1. Open a book. A selector appears under the page controls within a moment of the chapter
   rendering — that is the `load` report from Step 4 arriving.
2. Turn a page. The selector **changes**, and changes again on the way back. A selector that
   never updates means the `ook-set-page` report is not reaching the parent; a selector that
   is always `body > p:nth-child(1)` means it is reaching the parent but `firstElementOnPage`
   is answering about the wrong page.
3. Jump to a chapter via the TOC. The selector updates there too, and the reader still
   navigates normally — the new message kind must not disturb `link`/`scroll`/`pages`.
4. Nothing flickers or spins. If the app pegs a core and the page number twitches, you have
   built the render loop described under "Why it works" below.

Then `cargo test` and `cargo clippy --all-targets -- -D warnings`.

**Minimal implementation.** Five edits, none longer than four lines.

1. **`src/web/assets/ook-events-listener.js`** — a fourth branch, exactly like its
   neighbours:

   ```js
   if (e.data.kind === "ook-position") {
     dioxus.send("position:" + e.data.selector);
   }
   ```

2. **The variant**, in `src/ui/reader.rs`:

   ```rust
   pub(crate) enum BridgeMsg {
       Link(String),
       Scroll(usize),
       Pages(usize),
       Position(String),
   }
   ```

3. **The parse arm**, appended to the `if let` chain before the final `else`:

   ```rust
   } else if let Some(selector) = msg.strip_prefix("position:") {
       (!selector.is_empty()).then(|| BridgeMsg::Position(selector.to_string()))
   } else {
   ```

4. **The state**, in `src/nav.rs` — one field and one method:

   ```rust
   #[derive(Store, Default)]
   pub(crate) struct ReaderData {
       // … existing fields …
       /// Selector for the first element on the current page, as last reported by
       /// the chapter iframe. `None` until the first report of a freshly loaded
       /// chapter arrives.
       pub(crate) anchor: Option<String>,
   }
   ```

   ```rust
   pub(crate) fn on_position(self, selector: String) {
       self.data.anchor().set(Some(selector));
   }
   ```

   …and the handler arm in `use_bridge`:

   ```rust
   Some(BridgeMsg::Position(selector)) => state.on_position(selector),
   ```

5. **The temporary readout**, in `Reader`'s body and `rsx!` — this is the eyeball, and it is
   scaffolding: it stays useful through Steps 6–8 and comes out at Step 9.

   ```rust
   let anchor = state.data.anchor();
   let anchor_label = anchor().unwrap_or_else(|| "—".to_string());
   ```

   ```rust
   p {
       style: "text-align: center; font-size: 12px; opacity: 0.6;",
       "{anchor_label}"
   }
   ```

**Why it works.**

- **`strip_prefix` is the whole parser.** It returns `Option<&str>` holding *everything*
  after the prefix — colons, spaces, parentheses and all. The alternative shape,
  `msg.split(':')` plus `next()`, would return `"body > div"` and silently drop the rest,
  and the bug would only show up as a restore landing in the wrong place two steps from now.
  The `"position:"` prefix is a framing marker, not a delimiter, and this is the difference.
- **`(!selector.is_empty()).then(|| …)`** is `bool` → `Option` in one expression:
  `then` runs the closure only when the bool is `true`, so the `String` allocation never
  happens for the rejected case. Its sibling `then_some(value)` takes an eager value and
  would allocate first and discard it. Reach for `then` whenever the payload costs anything
  to build.
- **The message field name is a cross-file contract.** `e.data.selector` in the listener has
  to match the `{ kind: "ook-position", selector }` that `page-position.js` posts. Nothing in
  the compiler checks that — the same hazard `the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives`
  exists to catch for `__ookBlobUrl`. `epub.rs`'s injection test already asserts the served
  document contains `ook-position`; adding `assert!(BRIDGE_JS.contains("ook-position"))` to
  the reader test closes the other half of the loop for one line.
- **`Option<String>` needs no new machinery in the store.** `pending_fragment` is already
  exactly this shape, so `#[derive(Store)]` generates the same `anchor()` lens: call it to
  read (which subscribes whatever is reading), `.set(…)` to write.
- **Why this does not build a render loop, and how it easily could.** The chain in play is:
  `page` changes → the `ook-set-page` effect posts to the iframe → `page-position.js` reports
  → the bridge writes `anchor`. That terminates *because the `ook-set-page` effect reads only
  `page()`*. A Dioxus effect re-runs when a signal it read during its last run changes; add
  `anchor()` to that effect's body and the write feeds the post that caused it, forever. The
  readout in item 5 is safe for the same reason inverted: it reads `anchor` during **render**,
  and re-rendering does not re-run effects. (Writing a signal *during* render is the other
  half of this trap — the bridge writes from a `use_future`, which is not render.)

**Scope note.** State only. No `save_position` call, no `Library` in the reader, no restore —
Steps 6–8. The readout is throwaway UI, not a feature.

> **Hand-off to Step 6 — the stale-pair hazard.** Step 6 saves
> `Locator { spine_index: chapter(), selector: anchor() }`, and those two values can briefly
> disagree. On a chapter change `apply` sets `chapter` *and* `page`, so the `ook-set-page`
> effect fires while the iframe still holds the **previous** chapter's document; that document
> answers with its own selector, and pairing it with the already-advanced `chapter` persists a
> position that points into the wrong file. Step 5 is unharmed (a stale string in state is
> overwritten by the next report), so this is Step 6's problem to solve — most likely by not
> saving while a chapter load is in flight, which is a state the reader is close to already
> tracking via `pending_fragment` / `pending_last`. Worth deciding deliberately there rather
> than discovering it as a mystery bad bookmark at Step 8.

---

## Step 6a — a real error type (`thiserror`)

> **Status:** done — committed in `c5389ad` (46 tests green, clippy clean apart from the
> two `dead_code` errors this step was never going to touch). Pulled into this phase on
> request; closes **R3** from the
> [July 2026 review backlog](../review-2026-07-steps.md#r3--a-real-error-type-with-thiserror).

Step 6 was originally going to swallow save failures with `_ = …` and a note pointing at R3
as future work. R3 is being done now instead, which makes it its own step: it touches three
files and eight call sites, none of which have anything to do with reading position. Landing
it separately keeps both commits reviewable and gives Step 6b a clean error to log.

**A correction to what Step 6's plan said, and to the phase doc.** The phase's Known
constraints claim "errors on the save path are still `Box<dyn Error>`." They are not.
`save_position` returns `rusqlite::Result<()>`, which is already a perfectly matchable typed
error — as do `list`, `remove`, `touch_opened`, and `position`. Every `Box<dyn Error>` in the
codebase is on the **import/open** path:

| Site | Today |
|---|---|
| `epub::read_metadata` | `Result<BookMeta, Box<dyn Error>>` |
| `epub::spine_hrefs` | `Result<Vec<String>, Box<dyn Error>>` |
| `Library::add_from_path` | `Result<Book, Box<dyn Error>>` (+ its inner closure) |
| `ui::library::open_epub` | `Result<(Epub, Vec<String>), Box<dyn Error>>` |

So this step does not improve the save path's *type* at all — it improves the import path,
and it gives Step 6b a house style to follow. Worth knowing before you start, so the payoff
lands where you're looking for it.

**A thing you'll find on the way.** `read_metadata` cannot fail. Read its body: there is not
a single `?` in it — the title falls back to `"Untitled"`, the author is an `Option`, and the
cover swallows its error with `.ok()?` inside an `and_then`. It returns `Ok(…)`
unconditionally. Its signature has been lying since it was written, and every caller pays
with a `?` or an `.expect()` for a failure that cannot happen. **Delete the `Result`**, don't
port it. R3's own status note called this out ("infallible `read_metadata`") and this is
where it gets paid off.

### Runnable check

A real `#[test]` this time — the first one in this phase that tests behavior rather than
wiring. It goes in `src/library.rs`'s test module and it asserts the thing `Box<dyn Error>`
makes impossible: that a caller can **match on what went wrong**.

```rust
#[test]
fn importing_a_file_that_is_not_an_epub_reports_a_matchable_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let books_dir = dir.path().join("books");
    std::fs::create_dir_all(&books_dir).expect("books dir");
    let library =
        Library::open(dir.path().join("library.sqlite3"), &books_dir).expect("library opens");

    // A file with the right extension and the wrong bytes — the realistic failure,
    // and the one Feature 2's import panel can actually hand us.
    let source = dir.path().join("not-a-book.epub");
    std::fs::write(&source, b"definitely not a zip archive").expect("write fixture");

    let error = library.add_from_path(&source, 1_000).unwrap_err();

    // The whole point of the enum: the UI can tell "not an EPUB" from "disk full".
    assert!(matches!(error, Error::Ebook(_)), "got {error:?}");
    assert!(!error.to_string().is_empty(), "Display must carry a cause for logs");

    // The failed import cleaned up after itself — no managed copy left behind.
    let leftovers = std::fs::read_dir(&books_dir).expect("books dir readable").count();
    assert_eq!(leftovers, 0, "a failed import must not leak its managed copy");
}
```

`cargo test importing_a_file_that_is_not_an_epub` fails to compile first — `Error` doesn't
exist yet. That's the red.

The last assertion is a freebie worth taking: `add_from_path` copies the source into
`books_dir` *before* it tries to open it, and unwinds that copy in its error branch. Nothing
currently tests that branch, and this step rewrites the very `return Err(…)` that triggers it.

Then the safety net, because everything else here is a behavior-preserving refactor:
`cargo test` (45 → 46) and `cargo clippy --all-targets -- -D warnings`, which still reports
the Step 3 `dead_code` pair until 6b.

### Minimal implementation

```toml
# Cargo.toml
thiserror = "2"
```

**1. `src/epub.rs` — one variant, because one thing can fail.**

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("spine entry with a dangling idref")]
    DanglingIdref,
}
```

`spine_hrefs` returns `Result<Vec<String>, Error>`, and its
`.ok_or("spine entry with a dangling idref")?` becomes `.ok_or(Error::DanglingIdref)?`.
`read_metadata` returns a bare `BookMeta` — drop the `Result`, the `Ok(…)` wrapper, and the
`?`/`.expect()` at all four call sites (`library.rs:126`, `library.rs:371`, `epub.rs:373`,
`epub.rs:403`).

**2. `src/library.rs` — the app's book-handling error.**

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not read the EPUB: {0}")]
    Ebook(#[from] rbook::ebook::errors::EbookError),

    #[error("could not read the EPUB's spine: {0}")]
    Spine(#[from] epub::Error),
}
```

`add_from_path` and its inner closure both return `Result<Book, Error>`, and
`return Err(Box::new(error))` becomes `return Err(error.into())`.

**3. `src/ui/library.rs` — `open_epub` returns `Result<(Epub, Vec<String>), library::Error>`.**

Both of its `?`s convert through the enum above. Its `Display` already feeds
`open_status.set(Some(format!("Open failed: {error}")))`, so the UI copy improves for free.

### Why it works

- **`#[from]` generates the `From` impl that `?` needs.** `?` desugars to "on `Err(e)`,
  `return Err(From::from(e))`". `Box<dyn Error>` works today because the standard library
  ships a blanket `From<E: Error> for Box<dyn Error>` — which is exactly why it accepts
  *anything*, including that bare `&str` in `spine_hrefs`. A `&str` is not an error; it has
  no `source()`, no variant to match, no type. That looseness is the thing being traded away.
- **`?` performs exactly one `From` hop, never a chain.** This is why `library::Error` needs
  *both* `Ebook(#[from] EbookError)` and `Spine(#[from] epub::Error)`. `Epub::open` inside
  `add_from_path` yields rbook's `EbookError`; if the only route were
  `EbookError → epub::Error → library::Error`, `?` would not compile, because it will not
  chain two conversions to find a path. Distinct source types on distinct variants means no
  ambiguity — the compiler picks the impl by the type of the value you gave it.
- **`thiserror` is a derive, not a runtime.** It writes `Display` from your `#[error("…")]`
  strings and `std::error::Error` with `source()` wired to the `#[from]` field. Nothing is
  boxed, nothing is allocated, and the type stays `match`-able. Compare `anyhow`, which is
  the opposite trade: one opaque type, great for applications that only ever print the error,
  useless when a caller must branch on the cause. Writing this by hand is the point — it's
  the `From` / `Display` / `?` machinery you'd otherwise never see.
- **`{0}` in `#[error("database error: {0}")]`** interpolates the variant's first field, so
  the wrapped error's own `Display` is nested inside yours. That's how the rbook message
  survives all the way out to `open_status` in the UI.
- **`rbook::ebook::errors::EbookError` is itself a `thiserror` enum** with
  `#[error(transparent)]` variants for `Archive`, `Format`, `Reader` and `Io`. So a
  malformed-EPUB message arrives already human-readable, and if you ever need to tell "not a
  zip" from "bad OPF", the variants are right there to match on. Pinned version is 0.7.9 per
  `Cargo.lock`.

### Scope note

Refactor only — no behavior changes, which is why the existing 45 tests are the spec and
must pass unchanged. `Library`'s other methods keep returning `rusqlite::Result`; they are
already typed, and widening them to `library::Error` would churn every call site for no
gain. Do **not** touch the `expect`s in `open_default` — those are startup invariants
(no home directory, unopenable database) where crashing *is* the correct response, and R3
never asked for them.

> **Naming, deferred to Step 9.** `library::Error` ends up covering `open_epub`, which does
> no database work at all. It is the app's "something went wrong handling a book" error more
> than it is the library module's. Live with it for now and let the review pass decide
> whether it moves or gets renamed — after Step 8 has added whatever restore needs.

### How it landed

The plan held: four variants, four call sites, `read_metadata` returning a bare `BookMeta`,
45 → 46 tests, and clippy still reporting exactly the two `dead_code` errors it reported
before the step. Two things worth recording.

**The test landed without the comments this doc sketched.** They were agent-written prose
transplanted into `src/`, which is precisely what `CLAUDE.md` now forbids — the reasoning
belongs here, in the doc, where it already is. The assertion messages carry what a failing
run needs to know; the rest is this section's job.

**`Error::Io` collapses two distinct failures.** Both `source_path.canonicalize()` and
`fs::copy` yield `std::io::Error`, so "the file you picked is gone" and "the disk is full"
arrive at the UI as the same variant, distinguishable only by reading the `Display` string.
That is strictly better than `Box<dyn Error>` and not yet good enough to matter: nothing
branches on it today. Worth splitting only when Feature 2's import panel wants to say
something different for each — noted here so that decision starts from a known trade-off
rather than a surprise.

---

## Step 6b — persist the position

> **Status:** done — committed in `f1f95f3` (46 tests green, unchanged as planned; clippy's
> `dead_code` count dropped from two to one).

Step 5 got the selector into Rust and stopped. Step 3 built `save_position` and has had it
sitting unused ever since — it is one of the two `dead_code` warnings clippy has been
reporting for three commits. This step introduces them to each other: on every position
message, pair the selector the iframe just reported with the spine index the reader is on
and write the row. After this the database knows where you are; Steps 7–8 teach the reader
to read it back.

The whole step is one match arm and one widened function signature. Its weight is in three
decisions.

### Decision 1 — the save happens in the bridge, not in an effect

The tempting shape is a `use_effect` that reads `anchor()` and `chapter()` and saves when
either changes. Don't. An effect re-runs when **any** signal it read last time changes, so
that effect fires on a chapter change *before* the new chapter has reported anything — and
it fires with the previous chapter's selector still in `anchor`. The effect form doesn't
just permit the stale pair discussed below, it *manufactures* one on every chapter turn.

The bridge arm has neither problem: it runs exactly once per message, and the selector it
saves is the one that just arrived in that message.

### Decision 2 — the stale pair is accepted, not prevented

This is the hazard Step 5 handed over. On `chapter_next`, `apply` sets `chapter` *and*
`page`; the `ook-set-page` effect fires immediately and posts `page: 0` to an iframe that
still holds the **old** chapter's document. That document answers with its own selector, and
the bridge pairs it with the already-advanced `chapter`. A row is written pointing at an
element that does not exist in that spine entry.

**It is corrected within a frame.** The chapter effect right behind it loads the new
document, whose `load` handler calls `reportPosition` unconditionally — so a correct pair
overwrites the wrong one immediately, and every chapter change ends on a good row. The only
way a bad row survives is if the process dies inside that window.

Weigh that against the phase's own recorded constraint: **restore is best-effort.** A
selector that doesn't resolve falls back to the top of the chapter, never an error. So the
worst realized consequence of losing that race is *reopening at the top of the right
chapter* — which is exactly what a reader with no stored selector does anyway.

The alternative was considered and declined for this step: have `page-position.js` include
its own `document.location.pathname` in the message and have the parent drop any report
whose path isn't `docs[chapter]`. That is a genuine identity check rather than a timing
guess, it is about four lines, and it would make the pairing correct by construction. It is
declined **here** because it belongs with the pending-state cleanup — the reader already has
three overlapping notions of "a load is in flight" (`pending_fragment`, `pending_last`, and
the implicit one at chapter change) and Step 9 is already booked to collapse them into one
enum. Adding a fourth mechanism now and rewriting it in three steps is worse than living
with a self-correcting race for three steps.

> Record this as a decision, not an oversight. If Step 8's eyeball ever lands you at the top
> of the wrong chapter, this paragraph is the suspect — and the fix is the `pathname` check
> above.

### Decision 3 — a failed save is logged, not swallowed and not fatal

The original plan here was `_ = library.save_position(…)` with a note deferring to R3. With
Step 6a landed there is no reason to discard the error — `save_position` returns
`rusqlite::Result<()>`, whose `Display` says what SQLite objected to:

```rust
if let Err(error) = library.save_position(book_id, &locator, library::now_secs()) {
    eprintln!("could not save reading position: {error}");
}
```

Still no `expect`. A panic here would take down a reader mid-page-turn over a bookmark — the
least important write in the app, on a path that runs several times a minute. And still no
UI surface for the failure: there is nowhere sensible to put "your bookmark didn't save" in
the reader chrome, and the next page turn retries anyway. `eprintln!` is the honest middle —
the failure is visible in the terminal you're already running `dx serve` in, which is exactly
where you'd look when the `sqlite3` query below shows a row that stopped moving.

### Runnable check

**There is no new `#[test]` in this step, and that is deliberate.** Everything testable is
already tested: `position_round_trips_and_latest_save_wins` covers the upsert, and
`bridge_parses_a_position_selector_whole` covers the message. What's new is the *wiring*
between them, which lives inside a component and a `use_future` — nothing `cargo test` can
reach. So the gate is a live database, watched.

Before you start, look at what's there now:

```sh
sqlite3 ~/Library/Application\ Support/com.dimaportenko.ook-reader/library.sqlite3 \
  "select * from positions;"
```

That prints nothing today — the table has existed since Step 3 and has never had a row
written to it. That empty result *is* the failing test. Then `dx serve`, open a book, and
run it again:

1. **A row appears** after the chapter renders, with `book_id` matching the book you opened
   and a `selector` that looks like `body > p:nth-child(1)`. This is the `load` report.
2. **Turn a page, re-run the query — `selector` changed**, and `updated_at` moved. Turn back,
   it changes again. A row that never changes means the save is only wired to the load path.
3. **Still exactly one row per book.** `select count(*) from positions;` stays at the number
   of books you have opened, not the number of pages you have turned. Two rows for one book
   means the `ON CONFLICT` clause isn't matching — check that `book_id` is the primary key.
4. **Open a second book, then go back to the first.** Two rows, each remembering its own
   place, neither disturbed by the other.
5. **`spine_index` tracks the chapter.** Jump a few chapters and watch it follow. This is the
   half that the stale pair above can get wrong — if you catch a `spine_index` that doesn't
   match the chapter label on screen, re-run the query a second later and confirm it
   corrected itself.
6. **No hitch on page turns.** The recorded decision is one `UPDATE` per turn with no
   debounce; if paging feels heavier than it did yesterday, say so and it gets revisited.

Then `cargo test` (46 after Step 6a, unchanged by this step) and
`cargo clippy --all-targets -- -D warnings`. Clippy's output is itself a check here: the two
`dead_code` errors standing since Step 3 should drop to **one**. `Locator` gets constructed
and `save_position` gets called, so both go quiet; `position` stays unused and stays
reported until Step 8 reads a locator back. Zero errors would mean you wired up more than
this step asked for; still two would mean the save arm isn't being reached.

### Minimal implementation

Three edits, all in `src/ui/reader.rs`.

1. **Pull the library out of context** in `Reader`, next to the existing `use_context`, and
   pass it and the book id into the bridge:

   ```rust
   let library = use_context::<Rc<Library>>();
   ```

   ```rust
   use_bridge(state, docs, library, book.id);
   ```

   Note this has to be read *before* `book` is partially moved — `book.docs` is already moved
   into `docs` at the top of the function, so keep `book.id` handy or read it early. `id` is
   `i64`, which is `Copy`, so `book.id` after the move of `book.docs` is fine; the field move
   is per-field.

2. **Widen `use_bridge`:**

   ```rust
   fn use_bridge(state: ReaderState, docs: Rc<Vec<String>>, library: Rc<Library>, book_id: i64) {
       use_future(move || {
           let docs = docs.clone();
           let library = Rc::clone(&library);
           async move {
   ```

3. **Save in the `Position` arm:**

   ```rust
   Some(BridgeMsg::Position(selector)) => {
       let locator = library::Locator {
           spine_index: *state.data.chapter().peek(),
           selector: selector.clone(),
       };
       state.on_position(selector);
       if let Err(error) = library.save_position(book_id, &locator, library::now_secs()) {
           eprintln!("could not save reading position: {error}");
       }
   }
   ```

   …plus `library::{self, Library}` on the `use crate::{…}` line at the top.

### Why it works

- **`.peek()`, not `chapter()`.** Reading a signal the ordinary way *subscribes the current
  reactive scope*, and inside `use_future` that means the future is torn down and restarted
  when the signal changes. Restarting this future would drop the bridge mid-stream: a new
  `document::eval(BRIDGE_JS)` registers a second `message` listener while the first one's
  channel is gone. `peek` reads the value without recording the dependency. The `Link` arm a
  few lines up already does exactly this, for exactly this reason — the pattern is
  established, this step just follows it.
- **The clone is the price of using the selector twice.** `on_position` takes `String` by
  value (it stores it) and `Locator` owns its selector, so one of the two has to be a copy.
  Building the `Locator` first and moving the original into `on_position` keeps the clone to
  one and puts it where it reads as intentional. If you'd rather have no clone at all, drop
  the `on_position` call — see the scope note.
- **`Rc<Library>` survives the `.await` because this executor is single-threaded.** Dioxus
  desktop spawns component futures on a local executor, so a non-`Send` type held across an
  await point compiles fine. `docs: Rc<Vec<String>>` has been doing this since the link
  handler landed. On a `Send`-requiring executor this would be a compile error and the fix
  would be `Arc`.
- **`Rc::clone(&library)` inside the closure, not `library`.** `use_future` takes a closure
  that it may call more than once, so the closure has to *own* something it can hand out
  repeatedly — it can't move `library` into the async block directly. Cloning an `Rc` per
  call bumps a refcount; the `Library` itself is never duplicated. Same shape as the existing
  `let docs = docs.clone();` one line up.
- **`now_secs()` is called here, at the UI edge, not inside `save_position`.** That's the
  phase's recorded "the store never reads the clock" decision. It exists so tests can inject
  time and distinguish "preserved" from "restamped" — a store that calls `SystemTime::now()`
  internally cannot be tested for that at all.
- **The upsert makes repetition free.** `ON CONFLICT(book_id) DO UPDATE` means saving on
  every single page turn writes one row forever rather than accumulating history. That's why
  "no debounce" is affordable: the cost of a redundant save is an overwrite, not growth.

### Scope note

Save only. Nothing reads the position back — `Library::position` stays unused and clippy
keeps warning about it until Step 8. No restore, no `ook-sel:` hash, no seeding of `chapter`
on open.

> **`anchor` is now write-only.** With the readout skipped at Step 5, nothing in the app
> reads `ReaderData.anchor` — this step could take the selector straight from the message and
> skip `on_position` entirely. Keeping it is defensible (it is the reader's own answer to
> "where am I", it costs one `Option<String>`, and it is the natural place for a debug
> readout to come back). Deleting it is also defensible. **Don't decide now** — Step 9 is the
> review pass and it will have Step 8's restore code in front of it, which is the code most
> likely to want this field. Just don't add a second reader of it in the meantime without
> noticing you've settled the question.

### How it landed

The plan held with no deviations — three edits, all in `src/ui/reader.rs`, and the code is
the sketch above verbatim. What's worth recording is the evidence, since this step's gate was
a live database rather than a test.

The row, two minutes after the first page turn:

```
book_id  spine_index  selector                                   updated_at
16       11           body > div:nth-child(1) > p:nth-child(67)   2026-08-02 21:20:18
```

That single row answers four of the six checkpoints at once. It exists, which the empty table
before this step did not. Its selector is `p:nth-child(67)` rather than the first element,
so the save is wired to the page-turn path and not only to the `load` report. Its
`spine_index` is 11 rather than 0, so the chapter half tracks. And there is exactly one row
against 21 books — which the schema makes structural rather than lucky, since `book_id` is
the table's `INTEGER PRIMARY KEY` and the `ON CONFLICT` clause has nothing else to match on.
Per-book isolation and no-hitch paging were confirmed by hand.

Clippy was the other half of the gate and it moved exactly as predicted: two `dead_code`
errors down to one. `Locator` is now constructed and `save_position` now called, so both went
quiet; `position` stays reported until Step 8 reads a locator back. Zero would have meant
more than this step asked for; two would have meant the save arm was never reached.

**One dependency this step introduced that the plan didn't name.** `App` renders
`Reader { key: "{book.id}", … }`, so switching books remounts the component and the
`use_future` re-captures a fresh `book_id`. Without that key the bridge would keep saving
under the first book's id after you opened a second one, and checkpoint 4 would have failed
in a way the schema could not protect against. The key predates this step; this step is the
first thing whose correctness rests on it. Worth remembering at Step 9 if anything proposes
to remove it.

---

> **Hand-off to Step 7.** Step 7 is pure JS and touches no Rust: `fragment-scroll.js` learns
> that a hash beginning `ook-sel:` is a selector for `querySelector` rather than an element
> id, and reports the resulting page over the existing `ook-scroll` channel. It is verifiable
> entirely in devtools — set `iframe.contentWindow.location.hash` by hand to a selector you
> copied out of the `positions` table above and watch the page jump. Which means the row this
> step starts writing is the input to Step 7's eyeball: turn some pages, read the selector out
> of SQLite, and paste it into the hash. The database becomes the test fixture.

---

## Step 7 — resolve a selector back to a page (JS)

> **Status:** done — committed in `cbdda5e` (46 tests green, clippy still at the one
> expected `dead_code` error for `position`).
>
> **Fixture note, worth more than the code.** The first eyeball attempt reported nothing and
> looked like a failure; it was the test setup. Two traps, both worth remembering:
>
> - **Hamlet's stored row cannot demonstrate this step.** It is `spine_index 0` with selector
>   `body > div:nth-child(1)` — the title page's outer div, at `offsetLeft: 0`, which is page
>   0, which is the page you are already on. A perfect resolve looks identical to a total
>   miss. The useful row was Sherlock Holmes: `spine_index 8`, `p:nth-child(215)`, far enough
>   into a chapter that landing on it is visible. Pick a fixture whose success and failure
>   look different.
> - **`hashchange` does not fire when the hash is assigned its existing value.** Setting the
>   same fragment three times in the console fires the listener once; attempts two and three
>   are no-ops at the browser level and `reportFragmentPage` never runs. Clear with
>   `location.hash = ""` first, or just call `reportFragmentPage()` — it is a top-level
>   function declaration in a classic script, so it is on `window`.
>
> A selector copied from devtools' "Copy → Selector Path" is also not a selector this app
> wrote; only the three rows in `positions` are. Both traps produce the same symptom —
> nothing happens — which is exactly the symptom a genuine bug in `elementFor` would produce.
> That ambiguity is the argument for the assertion this step added: the automated check
> proves the marker reaches the document, so an eyeball miss is never about *that*.

Pure JS, no Rust. `fragment-scroll.js` learns that a hash beginning `ook-sel:` names a
selector for `querySelector` rather than an element id, resolves it, and reports the page
over the `ook-scroll` channel it already uses.

**The reason this step is small is that the restore pipeline already exists.** Follow it
backwards from the thing Step 8 wants: `on_scroll` sets `page` and clears
`pending_fragment`; `on_scroll` fires on `ook-scroll`; `ook-scroll` is posted by
`reportFragmentPage`; `reportFragmentPage` runs on `load` and `hashchange`; the hash is set
by `chapter-loader.js` from `pending_fragment`. Every link the user has ever clicked has
travelled that path. Restore needs the same path carrying a different kind of payload — so
rather than build a second one, this step widens what a fragment is allowed to be, and
Step 8 becomes "seed `chapter` and `pending_fragment` from the stored row."

That reuse is worth more than the code it saves. `pending_fragment` being `Some` is also
what hides the iframe (`class: if pending_last() || pending_fragment().is_some()`), so a
restore that rides this channel inherits the no-flash-of-the-wrong-page behavior for free.
A parallel channel would have had to reinvent it.

### Runnable check

**One assertion, folded into the existing injection test** — the same shape as Step 5's
`BRIDGE_JS.contains("ook-position")` assertion, and for the same reason. In
`serving_a_chapter_injects_the_reader_assets`, next to its siblings:

```rust
assert!(xhtml.contains("ook-sel:")); // fragment-scroll.js
```

That is red today: no asset contains the string. It is a weak assertion about a strong
thing — `ook-sel:` is a contract between `fragment-scroll.js` and the Rust that will build
the hash in Step 8, and no compiler checks a string that crosses a language boundary. The
test count stays at 46; this is an assertion, not a test.

**The real check is in devtools, and the fixture is your own database.** Read the row out:

```sh
sqlite3 ~/Library/Application\ Support/com.dimaportenko.ook-reader/library.sqlite3 \
  "select spine_index, selector from positions;"
```

Then `dx serve`, open that book, navigate to that chapter, and in the iframe's console:

1. **The happy path.** Set the hash to the prefix plus the selector you just read, encoding
   the whole thing — prefix included — because that is what `chapter-loader.js` does with
   the fragment it is handed:

   ```js
   location.hash = encodeURIComponent("ook-sel:body > div:nth-child(1) > p:nth-child(67)")
   ```

   The page should jump to the page holding that paragraph. This is the round trip closing:
   Step 4 wrote that selector by looking at what was on the page, and Step 7 finds the page
   by looking at that selector.

2. **The garbage path, which matters more.** Set a selector that cannot parse:

   ```js
   location.hash = encodeURIComponent("ook-sel:not a ((valid selector")
   ```

   Nothing should appear in the console, and the reader should stay exactly where it is —
   *not* freeze. Why this is the checkpoint that earns its keep is in **Why it works**.

3. **The old path still works.** Click an internal link in the table of contents. Ordinary
   id fragments must behave exactly as they did — this step adds a branch, it does not
   replace one.

Then `cargo clippy --all-targets -- -D warnings`: still exactly **one** `dead_code` error
(`position`). This step touches no Rust, so a change in that number means something
unintended came along.

### Minimal implementation

All of it in `src/web/assets/fragment-scroll.js`. Pull the lookup out of
`reportFragmentPage` into its own function, because it now has two ways to fail and the
reporting logic should not have to care which:

```js
const SELECTOR_PREFIX = "ook-sel:";

function elementFor(target) {
  if (!target.startsWith(SELECTOR_PREFIX)) {
    return document.getElementById(target);
  }

  try {
    return document.querySelector(target.slice(SELECTOR_PREFIX.length));
  } catch {
    return null;
  }
}
```

`reportFragmentPage` then changes by two lines — the `id` binding becomes a `target`, and
the `getElementById` call becomes `elementFor(target)`:

```js
function reportFragmentPage() {
  const target = decodeURIComponent(location.hash.slice(1));
  if (!target) return;

  const el = elementFor(target);
  const page = el ? Math.round(el.offsetLeft / window.innerWidth) : currentPage();

  document.documentElement.scrollLeft = 0;
  window.parent.postMessage({ kind: "ook-scroll", page: page }, "*");
}
```

**Keep your existing comments.** The one above the `el ? … : currentPage()` line explains
why an unresolvable fragment reports the current page rather than staying silent, and that
reasoning now covers three cases instead of one — a missing id, a selector that matches
nothing, and a selector that will not parse. It might be worth a few words from you saying
so, since the comment currently names only the first.

### Why it works

- **`querySelector` throws where `getElementById` returns `null`, and that asymmetry is the
  entire reason for the `try`.** A malformed selector raises `SyntaxError`. An uncaught
  throw inside `reportFragmentPage` skips the `postMessage` at the bottom — and that
  message is the *only* thing that clears `pending_fragment`. So the failure is not "the
  reader lands on the wrong page." It is: `pending_fragment` stays `Some` forever, the
  iframe keeps its `invisible` class forever, and the reader opens to a blank rectangle
  with no error anywhere. Catching turns the worst case back into the documented one — the
  phase's **Known constraints** already promise that restore is best-effort and an
  unresolvable target falls back rather than erroring.
- **This is not a hypothetical.** The same Known constraints note says re-importing a book
  keeps the row id but replaces the bytes, so a stored selector can outlive the document it
  described. The selector is a string from a database, and by the time it reaches
  `querySelector` it is untrusted input.
- **Decode before you test the prefix.** `encodeURIComponent` escapes `:` to `%3A`, so a
  raw `location.hash` never contains a literal `ook-sel:` even when the fragment did.
  `startsWith` on the still-encoded string silently never matches, every restore quietly
  falls through to `getElementById`, and you get the current page every time — a bug that
  looks like "restore doesn't work" and has nothing to do with selectors. The existing
  `decodeURIComponent` on the line above is already in the right place; the point is not to
  move the prefix check above it.
- **`ook-sel:` is a namespace, not a delimiter.** `slice(SELECTOR_PREFIX.length)` takes
  everything after the marker, which is what keeps `body > p:nth-child(1)` intact — the
  same lesson `strip_prefix` taught in Step 5, in the other language. A `split(":")` here
  would hand back `body > p` and drop the rest, and the symptom would be landing on the
  wrong page rather than a parse error.
- **A document that happened to contain `id="ook-sel:x"` loses, and that is the safe
  direction.** It would be read as the selector `x`, match nothing or an `<x>` element, and
  fall back to the current page. The reverse choice — checking ids first — would let a
  document silently hijack the restore channel.
- **The round trip closes because both sides compute the page the same way.**
  `page-position.js` picks its element with `Math.round(el.offsetLeft / window.innerWidth)`
  and `fragment-scroll.js` reads it back with the identical expression. That is not a
  coincidence to be grateful for; it is a duplicated formula in two files that must agree,
  and it is exactly what Step 9's "two halves of the page↔element conversion living in two
  JS files" item is about. Notice it here so the refactor is obvious later.

### Scope note

No Rust beyond the one assertion, and nothing in the app produces an `ook-sel:` hash yet —
you are setting it by hand. **Step 8** is what seeds `chapter` and `pending_fragment` from
the stored locator on open, which is when the mechanism proved here becomes a feature. The
`position` method stays dead and clippy stays at one warning until then.

Deliberately not handled: an element that resolves but is not currently laid out (a zero-box
element inside a `display: none` ancestor) reports page 0 rather than nothing, because
`offsetLeft` is 0 for it. `page-position.js` never *writes* such a selector — it skips
elements with no `getClientRects()` — so the case needs a document that changed under a
stored locator. It is the same re-import hazard as everything else here, and it fails the
same best-effort way.

---

## Step 8 — restore on open

> **Status:** code committed in `e56d0b8` (48 tests green — both new `nav` tests among them
> — and clippy at **zero** errors for the first time this phase, `Library::position` having
> been dead since Step 3).
>
> **The end-to-end eyeball is not yet confirmed.** What the automated gate proves is that
> `restored_data` computes the right seed and that the `ook-sel:` prefix reaches the injected
> assets. What it cannot prove is that quitting the app, relaunching, and opening the book
> lands on the same page without a flash — that is the Milestone 2 exit criterion, and no
> `#[test]` in this repo can reach a webview to check it. Until someone runs the three checks
> under **Runnable check** above, treat the criterion as unticked.

The payoff step. `Reader` reads the stored `Locator` once at mount and **seeds** `chapter`
and `pending_fragment` from it, so the first render is already pointed at the right chapter
with the right target attached. Ticks the Milestone 2 exit criterion: quit, relaunch, open
the book, land where you stopped.

**Seed, don't correct.** The tempting shape is a `use_effect` that fires after mount and
sets `chapter`. That produces a visible wrong-chapter flash and a wasted fetch: render at
chapter 0 → the loader effect fetches chapter 0 → the effect sets chapter 8 → the loader
effect fetches chapter 8. `use_store`'s initializer runs **once, before the first render**,
so seeding there means chapter 8 is the only chapter the loader ever sees. Restore stops
being a correction applied to a wrong state and becomes the state the reader was born in.

That is also what makes the no-flash behavior fall out for free. `pending_fragment` being
`Some` is what drives `class: if pending_last() || pending_fragment().is_some()` on the
iframe, so a seeded fragment means the iframe is `invisible` from the very first frame and
stays hidden until `ook-scroll` lands and `on_scroll` clears it. Nothing new to build — the
same latch that has hidden TOC-link jumps since Phase 5.

### Runnable check

**Pure `#[test]`s in `nav.rs`** — the decision "given a stored locator, what state does the
reader start in?" is ordinary Rust with no Dioxus in it, provided you put it in a free
function rather than inlining it into the hook. That is the whole reason to extract
`restored_data`: a hook is not unit-testable, a function taking `(Option<Locator>, usize)`
and returning `ReaderData` is.

Two tests, bundled the way `page_nav_rolls_over_chapter_boundaries` bundles its cases:

```rust
#[test]
fn a_stored_position_seeds_the_chapter_and_a_selector_fragment() {
    let locator = Locator {
        spine_index: 8,
        selector: "body > div:nth-child(1) > p:nth-child(215)".to_string(),
    };

    let data = restored_data(Some(locator), 24);
    assert_eq!(data.chapter, 8);
    assert_eq!(
        data.pending_fragment.as_deref(),
        Some("ook-sel:body > div:nth-child(1) > p:nth-child(215)")
    );
    // The page is deliberately *not* restored. It is derived from the window
    // size, so it is recomputed: `fragment-scroll.js` resolves the selector and
    // reports the page back over `ook-scroll`.
    assert_eq!(data.page, 0);

    // No stored position — start at the top of the book.
    let fresh = restored_data(None, 24);
    assert_eq!(fresh.chapter, 0);
    assert_eq!(fresh.pending_fragment, None);

    // A spine index past the end falls back to the start rather than seeding an
    // index that `docs[chapter()]` would panic on. Re-import keeps the row id
    // and replaces the bytes, so a stored index can outlive the spine it named.
    let stale = Locator {
        spine_index: 24,
        selector: "body > p:nth-child(3)".to_string(),
    };
    let data = restored_data(Some(stale), 24);
    assert_eq!(data.chapter, 0);
    assert_eq!(data.pending_fragment, None);
}

#[test]
fn the_fragment_prefix_matches_the_one_the_asset_looks_for() {
    // Rust builds this prefix, `fragment-scroll.js` tests for it, and no
    // compiler checks a string that crosses a language boundary. Same guard as
    // `the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives`.
    assert!(crate::web::assets::INJECTED_ASSETS.contains(SELECTOR_FRAGMENT_PREFIX));
}
```

Red before the implementation: `restored_data` and `SELECTOR_FRAGMENT_PREFIX` don't exist,
so it won't compile. **48 tests** when green (46 + 2).

**Then the end-to-end eyeball, which is the actual milestone criterion.** Sherlock Holmes is
the fixture to use — its stored row is `spine_index 8` with `p:nth-child(215)`, deep enough
into a chapter that a successful restore is unmistakable. (Step 7's status note explains why
Hamlet's row cannot demonstrate anything: it resolves to page 0, which is where you'd land
anyway.)

1. Open the book, turn to somewhere distinctive, **quit the app entirely**.
2. Relaunch, open the same book. It should land on that chapter *and* that page, with **no
   flash** of chapter 1 and no flash of page 1 of the right chapter.
3. Open a book with no stored row — it opens at the top, as before.

Finally `cargo clippy --all-targets -- -D warnings`: this step should take it to **zero
errors**. `Library::position` has been dead since Step 3 and this is the step that calls it,
so the lone `dead_code` error disappearing is itself a signal the wiring is real.

### Minimal implementation

**`nav.rs`** — the prefix constant, the pure seed function, and one new parameter on the hook:

```rust
pub(crate) const SELECTOR_FRAGMENT_PREFIX: &str = "ook-sel:";

fn restored_data(start: Option<Locator>, chapter_count: usize) -> ReaderData {
    match start {
        Some(locator) if locator.spine_index < chapter_count => ReaderData {
            chapter: locator.spine_index,
            pending_fragment: Some(format!("{SELECTOR_FRAGMENT_PREFIX}{}", locator.selector)),
            ..Default::default()
        },
        _ => ReaderData::default(),
    }
}

pub(crate) fn use_reader_state(chapter_count: usize, start: Option<Locator>) -> ReaderState {
    ReaderState {
        data: use_store(move || restored_data(start, chapter_count)),
        chapter_count,
    }
}
```

Add `library::Locator` to the `use crate::{...}` at the top.

**`reader.rs`** — read the row once, hand it to the hook:

```rust
let start = use_hook(|| {
    library.position(book.id).unwrap_or_else(|error| {
        eprintln!("could not read reading position: {error}");
        None
    })
});
let state = nav::use_reader_state(docs.len(), start);
```

This goes after the `use_context::<Rc<Library>>()` line and before `use_bridge` — which
takes `library` by value, so it has to stay last.

### Why it works

- **`use_store`'s initializer is `impl FnOnce() -> T` and runs exactly once**, on the first
  render of this component. That is what makes seeding possible at all, and it is why the
  `move` closure can capture an owned `Option<Locator>` rather than needing a `Clone` on
  every render — `FnOnce` means the store takes the value and never asks for it again.
- **`use_hook` is the "compute once at mount, get a copy each render" primitive.** Without
  it, `library.position(book.id)` would hit SQLite on every single render of `Reader` — a
  query per page turn, per chapter change, per resize. The `Clone + 'static` bound is on the
  *return* type, not the closure, which is why borrowing `library` inside it is fine. You
  have already met its sibling: `use_hook_with_cleanup` is what `use_revoke_blob_on_unmount`
  is built from.
- **The match guard is a panic guard, not politeness.** `docs_for_iframe[chapter()]` in the
  loader effect is a direct index. Seed `chapter` with a stale `spine_index` and the reader
  panics on open, which for the user means a book that can never be opened again — the worst
  possible failure for a bookmark feature. `Some(locator) if locator.spine_index <
  chapter_count` is the cheapest place to stop that, because it is the only place the
  untrusted number enters reactive state.
- **`unwrap_or_else` over `.ok().flatten()`.** Both yield `Option<Locator>`; only one tells
  you the database is broken. It mirrors the save path's `eprintln!` in `use_bridge`, so both
  halves of the round trip fail the same visible-but-non-fatal way. A failed *read* must not
  block opening the book any more than a failed *write* may take down the reader.
- **`..Default::default()` in the struct literal** fills `page`, `page_count`,
  `pending_last`, and `anchor`, so adding a seventh field to `ReaderData` later cannot
  silently forget to initialize it here — it keeps compiling and keeps meaning "start
  neutral." Spelling all six out would be a maintenance trap.
- **The restore rides the existing pipeline end to end**, which is why this step is small.
  Seeded `pending_fragment` → the loader effect sets `frame.src = blob#ook-sel%3A…` →
  `fragment-scroll.js` fires on `load` → `elementFor` takes the selector branch Step 7 added
  → `Math.round(offsetLeft / innerWidth)` → `ook-scroll` → `on_scroll` sets `page` and
  clears `pending_fragment` → the iframe un-hides. Every arrow already existed; this step
  supplies the first one.
- **Failure is already handled, three levels deep.** A selector that matches nothing, or one
  that will not parse, returns `null` from `elementFor`, reports `currentPage()`, and still
  posts `ook-scroll` — so `pending_fragment` still clears and the iframe still un-hides, at
  the top of the chapter. That is the **Known constraints** promise about best-effort restore
  being kept by code that was already written, not by a new branch here.

### Scope note

Not in this step: **saving the position when the reader closes.** Right now a position is
only written on an `ook-position` message, which page turns produce — so a book you open and
close without turning a page keeps its old row. That is correct behavior, not a gap.

Also not here: the `pending_fragment` / `pending_last` / `anchor` trio is now three flags
encoding one "what is the reader waiting for?" question, and seeding makes that shape more
obviously wrong, not less. **Step 9** collapses them into an enum, alongside measuring
`firstElementOnPage` and reuniting the two halves of the page↔element conversion.
