# Feature: Library & Import

[← Milestone 2](../README.md)

**Goal:** let the user add `.epub` files and see their books in a list.

**Status:** 🚧 in progress

## Phases

| Phase | Outcome | Status |
|---|---|---|
| [Phase 6 — Library & Import](phase-6-library.md) | Add `.epub` files → list them (title/author/cover) → open one in the reader | 🚧 |

Planned as one phase (build log: [`phase-6-library-steps.md`](phase-6-library-steps.md)),
built data-first in six steps: metadata extraction → `rusqlite` store → `rfd` import →
list view → open (drop the `const BOOK`) → review & refactor.

## Notes

- Store the library in **`rusqlite`** (bundled), DB located via the `directories` crate
  (`ProjectDirs::data_dir()`). See [`RESEARCH.md`](../../../../RESEARCH.md) §4.
- File picking on desktop: the [`rfd`](https://crates.io/crates/rfd) native dialog crate.
- Web target later needs a different import path (sandboxed file input) — abstract it.

> Detailed phase files will be added when this feature is planned in depth.
</content>
