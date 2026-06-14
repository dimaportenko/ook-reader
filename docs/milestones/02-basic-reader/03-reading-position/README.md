# Feature: Reading Position

[← Milestone 2](../README.md)

**Goal:** remember where the reader stopped in each book and restore it on reopen.

**Status:** ⬜ planned

## Phases / steps

| Step | Focus | Status |
|---|---|---|
| Capture | Track `{spine_index, progression}` (progression = `scrollTop/scrollHeight` via JS eval) | ⬜ |
| Persist | Save the latest locator per book in `rusqlite` | ⬜ |
| Restore | On open, jump to the saved spine item and scroll to progression | ⬜ |

## Notes

- v1 locator = `{spine_index, progression}` — **easy** and enough to resume. Precise,
  shareable locators / highlights (DOM-resolved) are deferred; they're the one
  structurally hard area (see [`RESEARCH.md`](../../../../RESEARCH.md) §3.3).
- EPUB CFI is skipped for v1 (no mature Rust crate; only needed for cross-reader
  portability).

> Detailed phase files will be added when this feature is planned in depth.
</content>
