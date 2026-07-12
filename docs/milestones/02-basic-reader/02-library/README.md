# Feature: Library & Import

[← Milestone 2](../README.md)

**Goal:** let the user add `.epub` files and see their books in a list.

**Status:** 🚧 in progress

## Phases

| Phase | Outcome | Status |
|---|---|---|
| [Phase 6 — Library & Import](phase-6-library.md) | Add `.epub` files → list them (title/author/cover) → open one in the reader | 🚧 |

Planned as one phase (build log: [`phase-6-library-steps.md`](phase-6-library-steps.md)),
built data-first in six steps: metadata extraction → `rusqlite` store → Dioxus file-input
import → list view → open (drop the `const BOOK`) → review & refactor.

## Notes

- Store the library in **`rusqlite`** (bundled), DB located via the `directories` crate
  (`ProjectDirs::data_dir()`). See [`RESEARCH.md`](../../../../RESEARCH.md) §4.
- File picking uses Dioxus's `<input type="file">` abstraction. Dioxus Desktop opens its
  native dialog internally; a future web build can use the same event API but must consume
  `FileData::read_bytes()` because browsers do not expose absolute filesystem paths.

> Detailed phase files will be added when this feature is planned in depth.
