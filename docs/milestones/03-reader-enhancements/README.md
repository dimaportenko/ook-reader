# Milestone 3 — Reader Enhancements

[← Roadmap](../../roadmap.md)

**Goal:** the reason this project exists — features missing from other readers. A backlog
to refine once the basic reader (Milestone 2) works.

**Status:** ⬜ planned

## Candidate features (to prioritize later)

| Feature | Idea | Difficulty | Status |
|---|---|---|---|
| Themes & typography | Font, size, spacing, light/dark/sepia via injected CSS variables | Easy (optionally vendor [ReadiumCSS](https://github.com/readium/readium-css)) | ⬜ |
| TOC & navigation | Nested TOC (NCX + nav.xhtml), bookmarks, jump-to-chapter | Easy — `rbook` gives the tree | ⬜ |
| Full-text search | Search within / across the library | Easy with [`tantivy`](https://crates.io/crates/tantivy); jump-to-hit is hard | ⬜ |
| Annotations | Highlights & notes, exportable | Hard — needs a WebView JS bridge for DOM-precise positions | ⬜ |
| Sync | Reading position across devices | Later (depends on persistence backend) | ⬜ |
| _(your missing features)_ | Capture the specific gaps you hit | — | ⬜ |

## Notes

Difficulty ratings and approaches come from [`RESEARCH.md`](../../../RESEARCH.md) §3.3.
The hard items (precise highlights, jump-to-search-hit) all share one root: resolving a
stored position into the **live rendered DOM** — defer and follow Readium's
"store text context, re-find in the DOM" pattern.

> Add a feature directory + phase files here as each idea is chosen and planned.
</content>
