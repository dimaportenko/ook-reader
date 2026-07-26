# Feature: Reading Position

[← Milestone 2](../README.md)

**Goal:** remember where the reader stopped in each book and restore it on reopen, and
surface recently-read books first in the library.

**Status:** ⬜ planned

## Phases / steps

| Step | Focus | Status |
|---|---|---|
| Recency | `added_at` + `last_opened_at` on `books`; list sorted by `COALESCE(last_opened_at, added_at)` | ⬜ |
| Capture | Track `{spine_index, element selector}` — the first element visible on the current page | ⬜ |
| Persist | Save the latest locator per book in `rusqlite` | ⬜ |
| Restore | On open, mount the saved spine item and resolve the selector back to a page | ⬜ |

## Notes

- **The page number is not the position.** Phase 5 paginates with CSS multi-columns, so the
  page index is derived from `window.innerWidth` (`page-count.js`:
  `scrollWidth / innerWidth`) and changes when the window resizes. Persisting it would
  restore the wrong spot at any other size. The durable locator is
  `{spine_index, element selector}`; the page is recomputed on restore.
- **Restore is the fragment-scroll path, generalized.** `fragment-scroll.js` already does
  element → page with `Math.round(el.offsetLeft / window.innerWidth)`, reports it over the
  `ook-scroll` bridge message, and rides the existing `pending_fragment` state machine
  (including hiding the iframe until it settles). Position restore is the same mechanism
  keyed on a selector instead of `location.hash`, so most of the machinery already exists —
  see [`fragment-scroll-via-url-hash.md`](../01-epub-rendering/fragment-scroll-via-url-hash.md).
- An earlier draft of this doc planned `progression = scrollTop/scrollHeight`. That is a
  **vertical-scroll** locator and does not apply to this reader; corrected above.
- Precise, shareable locators / highlights (DOM-resolved) are still deferred; they're the
  one structurally hard area (see [`RESEARCH.md`](../../../../RESEARCH.md) §3.3).
- EPUB CFI is skipped for v1 (no mature Rust crate; only needed for cross-reader
  portability).
- **Schema-change decision pending.** `Library::init` is `CREATE TABLE IF NOT EXISTS` with
  no migrator; the pre-release policy so far has been "delete the dev DB" (Step 7). Adding
  `added_at` / `last_opened_at` forces the choice: reset again, or land the project's first
  `ALTER TABLE` migration. Decide it on the cheap recency columns, before the position
  table needs it.

> Detailed phase files will be added when this feature is planned in depth.
</content>
