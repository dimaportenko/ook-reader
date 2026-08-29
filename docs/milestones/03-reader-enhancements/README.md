# Milestone 3 — Reader Enhancements

[← Roadmap](../../roadmap.md)

**Goal:** the reason this project exists — features missing from other readers. A backlog
to refine once the basic reader (Milestone 2) works.

**Status:** 🚧 in progress — and now the current focus outright. Themes & Typography
(Phase 4) and ToC & Navigation (Phase 8) are both **done**. The open phase is
[Chrome Material → Phase 10](06-chrome-material/phase-10-liquid-glass.md), whose Step 1 has
landed; [Phase 9](../04-multiplatform/01-mobile/phase-9-ios-simulator.md), the iOS port it
was waiting behind, closed with its review pass.

## Candidate features (to prioritize later)

| Feature | Idea | Difficulty | Status |
|---|---|---|---|
| [Themes & typography](04-themes-typography/README.md) | Font, size, spacing, light/dark/sepia via injected CSS variables | Easy (optionally vendor [ReadiumCSS](https://github.com/readium/readium-css)) | ✅ **done** ([ADR-0003](../../adr/0003-reader-controlled-theming-injected-layer.md)) |
| [ToC & navigation](05-toc-navigation/README.md) | Nested TOC (NCX + nav.xhtml), bookmarks, jump-to-chapter | Easy — `rbook` gives the tree | ✅ **done** ([Phase 8](05-toc-navigation/phase-8-toc-navigation.md)) — parsing was the easy half; the real work was that ToC ↔ spine is **many-to-many**. **Bookmarks were deferred out** and are still unscheduled |
| [Chrome material](06-chrome-material/README.md) | Liquid-glass app chrome — one `.glass` primitive that behaves the same in WebKit and Chromium | Easy-ish — the material is CSS; the hard part is that the bars have nothing behind them to blur | 🚧 **planned** ([Phase 10](06-chrome-material/phase-10-liquid-glass.md)) |
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
> [Chrome material](06-chrome-material/README.md) is the first one added this way — it came
> out of a *"can we get Liquid Glass buttons on iOS?"* question during the iOS port, and
> earned a directory because the answer is two rejected routes and a layout prerequisite,
> not a snippet.

**Started here:** Themes & typography
([Phase 4](04-themes-typography/phase-4-theming.md)) is the first phase of this milestone.
Its plan was written early, under ADR-0002's dogfood-driven prioritization, on the
expectation that it would be worked *ahead* of Milestone 2's Library and Reading-Position
features. In the event those shipped first and Phase 4 is being picked up now — the plan
holds, only the sequencing note it was written under turned out not to.
Decision: [ADR-0003](../../adr/0003-reader-controlled-theming-injected-layer.md).
</content>
