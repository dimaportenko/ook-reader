# Phase 7 — Reading Position

[← Feature: Reading Position](README.md) · **Status:** 🚧 in progress ·
build log: [`phase-7-reading-position-steps.md`](phase-7-reading-position-steps.md)

## Goal

Reopen the app, click a book, and land **where you stopped** — same chapter, same page —
and see the books you actually read floating to the top of the library. This is Feature 3
of Milestone 2 and clears its last exit criterion ("Reopen the app → it resumes at the same
spot").

## The crux

**The thing you see is not a thing you can store.** The page index is *derived*: the
injected `page-count.js` computes `scrollWidth / innerWidth` and `pagination.css` paginates
by translating `--ook-page` a viewport at a time. Resize the window and "page 7" is a
different sentence. Persist the page number and the reader restores the wrong spot at any
other window size — which is why the durable locator is `{spine_index, element selector}`
and the page is **recomputed** on restore.

The insight that keeps this small: **the reader already converts an element into a page.**
`fragment-scroll.js` does exactly that for TOC links —
`Math.round(el.offsetLeft / window.innerWidth)` — reports it over the `ook-scroll` bridge
message, and rides a state machine (`pending_fragment`) that hides the iframe until the
jump settles. So:

- **Capture** is that conversion run *backwards*: given the page, which element is first on
  it?
- **Restore** is that conversion run *as-is*, keyed on a selector instead of an `id` — and
  it can be delivered through the very same channel, the iframe URL's hash, because the
  hash is present **before the document parses** and so needs no load-ordering handshake.

Everything else is the pattern Phase 6 already established: **data first, UI last.** The
columns, the `Locator` type, and the store round-trip get `#[test]`s; the DOM work is an
eyeball, and it's deliberately checkable *in devtools* before any database is wired to it.

## Design decisions (recorded up front)

- **Reset the dev DB for Step 1; no migrator yet.** The new columns went straight into
  `CREATE TABLE IF NOT EXISTS books` and `library.sqlite3` was deleted by hand — the
  pre-release policy from Phase 6 Step 7, extended one more phase. Step 3 does **not** repeat
  the reset: creating the missing `positions` table is idempotent schema initialization,
  unlike adding columns to an existing table. The first real user still forces a migrator
  for column changes. The alternative considered and declined was a `PRAGMA user_version`
  migrator (~20 lines, `ALTER TABLE` per version).
- **The store never reads the clock.** `add_from_path(source, now)` and
  `touch_opened(id, now)` take the timestamp as a parameter; a `now_secs()` helper at the
  UI edge is the only place `SystemTime::now()` is called. This is not ceremony — a test
  that can't distinguish "preserved the original `added_at`" from "restamped it" is not a
  test, and two imports inside the same wall-clock second are indistinguishable unless the
  test supplies the time.
- **Position lives in its own table, one row per book.** `positions(book_id PRIMARY KEY,
  spine_index, selector, updated_at)`, latest-wins upsert. Chosen over two more columns on
  `books` because it keeps the row that describes the *file* separate from the row that
  describes the *reading*, and leaves room for history/bookmarks later without widening
  `books` again.
- **`spine_index` stays a `usize`, paid for with rusqlite's `fallible_uint` feature.**
  rusqlite gates `ToSql`/`FromSql` for `usize`/`u64` because SQLite's only integer is `i64`
  and both conversions can fail. Enabling it is one word in `Cargo.toml` and keeps `Locator`
  assignable straight into `ReaderData.chapter`, the way `epub::LinkTarget` already is;
  an `i64` or `u32` field would instead be the codebase's only non-`usize` spine index and
  push an unchecked `as usize` onto the restore path. The conversion still happens — it just
  happens checked, inside the store boundary.
- **The locator is a `nth-child` selector chain**, e.g. `body > div:nth-child(2) >
  p:nth-child(7)`, resolved with `document.querySelector`. Stable because the document is
  the same bytes every time — we serve it from the same managed copy. A flat index into
  `body.getElementsByTagName("*")` was considered (one integer, no string) and declined: a
  selector is inspectable, debuggable in devtools, and doesn't silently shift if the
  injected assets ever add an element.
- **The selector travels in the iframe URL hash**, prefixed `ook-sel:` to distinguish it
  from a plain element id. This buys the entire existing restore pipeline for free — hash
  set before parse, `load` + `hashchange` handlers, `ook-scroll` reporting, the
  iframe-hidden-until-settled state. See
  [`fragment-scroll-via-url-hash.md`](../01-epub-rendering/fragment-scroll-via-url-hash.md).
- **Save on every page change, no debounce.** One tiny `UPDATE` against a local SQLite file
  per page turn, on a path that already does a `postMessage` round trip. Revisit only if
  the eyeball shows a hitch.
- **Finding the anchor is a linear scan, accepted unmeasured.** `firstElementOnPage` walks
  `body.getElementsByTagName("*")` in document order and reads `offsetLeft` on each element
  until one lands on the target page. That is O(n) *layout reads*, not O(n) reflows — the
  loop never writes to the DOM, so layout is computed once on the first read and every
  later read hits the clean cached value. Once per page turn, on a chapter-sized document,
  that should be invisible; it is chosen for being obviously correct, not for being fast.
  **Not yet measured** — see the measurement and the alternatives under Step 9 below.

## Planned steps

*(smallest-first; the last step is the mandatory review-and-refactor pass)*

- [x] **Step 0 — Import several EPUBs at once.** *(Prerequisite, folded in mid-phase — not
      reading-position work.)* `multiple: true` on the file input and a loop that counts
      successes and failures rather than stopping at the first bad file, with a single
      `refresh_books` after the batch. Eyeball only — the behavior is the native `rfd`
      panel, which no `#[test]` can click. Lands **before** Step 1 because its column change
      resets the dev database and requires re-importing the whole shelf; paying off Feature
      2's deferred "multiple selection" TODO makes that one pick instead of N.
- [x] **Step 1 — Stamp `added_at`, and keep it stable across re-import.** Two nullable
      `INTEGER` columns in the schema, `add_from_path(source, now)`, and an `ON CONFLICT`
      clause that refreshes everything *except* when the book joined the library. `#[test]`.
- [x] **Step 2 — Recency: `touch_opened` + the sort.** `last_opened_at` written when a book
      is opened; `list()` orders by `COALESCE(last_opened_at, added_at) DESC, title`.
      `#[test]` with injected timestamps + eyeball (open a book, close it, it's first).
- [x] **Step 3 — A `Locator` and somewhere to put it.** `Locator { spine_index, selector }`,
      the `positions` table, `save_position` / `position`, and removing a book drops its
      position. Round-trip `#[test]`.
- [x] **Step 4 — Report the first element on the current page (JS).** A new injected asset
      that builds an `nth-child` chain for the first element whose `offsetLeft` lands on the
      current page. Asset-injection `#[test]` + a devtools round-trip eyeball
      (build a selector → `querySelector` finds the same element back).
- [x] **Step 5 — Bridge the selector into reader state.** `ook-position` →
      `position:<selector>` → `BridgeMsg::Position` → `ReaderData.anchor` (renamed from
      `locator` — the field holds only the selector half; `chapter` is the other half, and
      Step 6 pairs them). `BridgeMsg::parse` `#[test]` + eyeball.
- [ ] **Step 6 — Persist it.** The reader saves `{chapter, selector}` for its book on every
      position message. Eyeball + `sqlite3` inspection; storage is already tested at Step 3.
- [ ] **Step 7 — Resolve a selector back to a page (JS).** `fragment-scroll.js` learns the
      `ook-sel:` hash prefix and `querySelector`. Asset `#[test]` + a devtools eyeball that
      sets the hash by hand and watches the page jump — the whole restore mechanism proved
      before the database touches it.
- [ ] **Step 8 — Restore on open.** Seed chapter + pending target from the stored locator;
      the iframe stays hidden until `ook-scroll` lands. Nav `#[test]`s for the pending-state
      transitions + the end-to-end eyeball: quit the app, relaunch, open the book, land on
      the same page. **Ticks the milestone exit criterion.**
- [ ] **Step 9 — Review & refactor** (mandatory phase-closer): the pending-state shape in
      `ReaderData` (three flags that are really one enum), the two halves of the
      page↔element conversion now living in two JS files, the `Library` API surface, and
      the error handling on the save path. **Also: measure `firstElementOnPage`** on a long
      chapter — the scan cost is documented as a decision above and as a to-test item in
      [the steps doc](phase-7-reading-position-steps.md#step-4--report-the-first-element-on-the-current-page-js).

## Known constraints

- **Re-import can invalidate a locator.** Re-importing keeps the row id but replaces the
  bytes; if the new file's spine differs, a stored `spine_index`/selector may point
  somewhere else or nowhere. Restore must therefore be **best-effort** — a selector that
  doesn't resolve falls back to the top of the chapter, never an error (this is why
  `fragment-scroll.js` already reports the current page for an unknown id).
- **`ORDER BY` ties.** Two books imported in the same second sort by title as the
  tiebreaker; the tests assert contents, not order, wherever the times tie.
- **Errors on the save path** are still `Box<dyn Error>` — **R3** (`thiserror`) is
  [still open in the review backlog](../review-2026-07-steps.md). Step 6 should not grow a
  new `expect`; a failed save is a `_ = …` with a note, and R3 can be picked up separately.
