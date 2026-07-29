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
addition") pulled forward because this phase wipes the dev database **twice**, at Step 1 and
again at Step 3. `multiple: true` on the file input, a loop that survives one bad file, one
refresh at the end. Eyeball. Numbered `0` so the arc below keeps its numbering.

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

A prerequisite, not a reading-position step. Everything after this deletes the dev database
by hand — Step 1 for the two new `books` columns, Step 3 again for the `positions` table —
and each deletion is followed by re-importing the whole shelf through a picker that takes
**one file per click**. Two wipes × N books is a tax on every remaining step in the phase,
so it's worth twenty minutes now.

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
