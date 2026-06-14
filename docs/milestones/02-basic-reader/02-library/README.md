# Feature: Library & Import

[← Milestone 2](../README.md)

**Goal:** let the user add `.epub` files and see their books in a list.

**Status:** ⬜ planned

## Phases / steps

| Step | Focus | Status |
|---|---|---|
| Import | Native file dialog (`rfd`) to add `.epub` files into app storage | ⬜ |
| Library list | Dioxus list of imported books with title + cover (`rbook` metadata/cover) | ⬜ |
| Open | Click a book → route to the reader (Phase 3 renderer) | ⬜ |

## Notes

- Store the library in **`rusqlite`** (bundled), DB located via the `directories` crate
  (`ProjectDirs::data_dir()`). See [`RESEARCH.md`](../../../../RESEARCH.md) §4.
- File picking on desktop: the [`rfd`](https://crates.io/crates/rfd) native dialog crate.
- Web target later needs a different import path (sandboxed file input) — abstract it.

> Detailed phase files will be added when this feature is planned in depth.
</content>
