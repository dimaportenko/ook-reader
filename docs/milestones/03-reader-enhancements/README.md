# Milestone 3 — Reader Enhancements

[← Roadmap](../../roadmap.md)

**Goal:** the reason this project exists — features missing from other readers. A backlog
to refine once the basic reader (Milestone 2) works.

**Status:** 🚧 in progress — Milestone 2 shipped, so this is now the current focus. Themes &
Typography (Phase 4) is **done**; ToC & Navigation (Phase 8) is open.

## Candidate features (to prioritize later)

| Feature | Idea | Difficulty | Status |
|---|---|---|---|
| [Themes & typography](04-themes-typography/README.md) | Font, size, spacing, light/dark/sepia via injected CSS variables | Easy (optionally vendor [ReadiumCSS](https://github.com/readium/readium-css)) | ✅ **done** ([ADR-0003](../../adr/0003-reader-controlled-theming-injected-layer.md)) |
| [ToC & navigation](05-toc-navigation/README.md) | Nested TOC (NCX + nav.xhtml), bookmarks, jump-to-chapter | Easy — `rbook` gives the tree | 🚧 **in progress** ([Phase 8](05-toc-navigation/phase-8-toc-navigation.md)) — parsing is the easy half; the real work is that ToC ↔ spine is **many-to-many**. Bookmarks are deferred out of the phase |
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

**Started here:** Themes & typography
([Phase 4](04-themes-typography/phase-4-theming.md)) is the first phase of this milestone.
Its plan was written early, under ADR-0002's dogfood-driven prioritization, on the
expectation that it would be worked *ahead* of Milestone 2's Library and Reading-Position
features. In the event those shipped first and Phase 4 is being picked up now — the plan
holds, only the sequencing note it was written under turned out not to.
Decision: [ADR-0003](../../adr/0003-reader-controlled-theming-injected-layer.md).
</content>
