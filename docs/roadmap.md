# Ook Reader — Roadmap

The top of the documentation tree. Structure:

```
roadmap (this file)
└── milestones/        big outcomes
    └── features/      shippable capabilities within a milestone
        └── phases/    concrete, ordered steps of work
```

**How to read:** start here → open a milestone's `README.md` → open a feature's
`README.md` → open a phase file for the actual steps. Background research and rationale
live in [`../RESEARCH.md`](../RESEARCH.md).

**Guides:** reusable runbooks in [`guides/`](guides/) —
[NeoVim Rust / Dioxus Project Setup](guides/neovim-rust-dioxus-project-setup.md)
(portable to any Rust/Dioxus project).

## Vision

A cross-platform EPUB reader written in **Rust** with **Dioxus 0.7** and developed in
**NeoVim**. Two goals: learn Rust + Dioxus and idiomatic patterns, and build a reader
extensible with features missing from existing apps. **Desktop first**, then mobile and
web — one codebase across all targets.

> **Pivoted from Swift.** Earlier milestones were planned for Swift/Readium; the project
> restarted on Dioxus/Rust. See [`../RESEARCH.md`](../RESEARCH.md) §1.

**How we build it:** thin, end-to-end **vertical slices** — each step is something a real
reader can use (open a book, see text, turn a page, resume). The MVP slice sequence and the
"learning rides inside the slices" principle are in
[`vision-mvp-reader.md`](vision-mvp-reader.md); the decision is
[ADR-0001](adr/0001-walking-skeleton-vertical-slices.md). Domain vocabulary:
[`glossary.md`](glossary.md). Decisions: [`adr/`](adr/).

## Status legend

✅ done · 🚧 in progress · ⬜ planned · ⏸ deferred

## Milestones

| # | Milestone | Outcome | Status |
|---|---|---|---|
| 1 | [Foundation](milestones/01-foundation/README.md) | Rust/Dioxus NeoVim toolchain + a buildable desktop app | ⬜ |
| 2 | [Basic EPUB Reader](milestones/02-basic-reader/README.md) | Open an `.epub` and read it with paging + saved position | ✅ |
| 3 | [Reader Enhancements](milestones/03-reader-enhancements/README.md) | The features missing from other readers | 🚧 |
| 4 | [Multi-platform](milestones/04-multiplatform/README.md) | Mobile (iOS/Android) + web (WASM) from the same codebase | 🚧 |

## Current focus

**Milestone 4 → [Phase 9 — Run on the iOS simulator](milestones/04-multiplatform/01-mobile/phase-9-ios-simulator.md), Step 1.**
Read the book on the iPad. The crux is that **the UI ports for free and the assumptions do
not**: `dioxus::desktop` and `dioxus::mobile` are the same crate under two feature-gated
names, so the entire dependency graph — bundled `rusqlite` included — cross-compiled on the
first try and the whole crate failed on *five identifiers*. What the compiler cannot see is
everything the app assumes about being a desktop process: that a picked file has a path, that
a window has a size worth remembering, that pages turn on arrow keys, that the viewport is a
rectangle. So Step 1 is the compiler's half of the phase and Step 2 launches the thing and
writes down what actually breaks.

**Milestone 3 stays open behind it**, with no phase in progress — full-text search,
annotations, sync, and the **bookmarks** that
[Phase 8 deferred out](milestones/03-reader-enhancements/05-toc-navigation/README.md) are all
unclaimed on its [board](milestones/03-reader-enhancements/README.md). Milestone 4 is not
jumping that queue: its deferral was gated on "until the desktop reader works", and it does.
[`TODO.md`](../TODO.md) holds the smaller unscheduled items — two of which, swipe-to-turn and
keyboard paging, Phase 9 walks straight into.

[Phase 8 — ToC & Navigation](milestones/03-reader-enhancements/05-toc-navigation/phase-8-toc-navigation.md)
**closed 2026-08-18** and was the last one worked: the reader names the chapter you are in
and opens a contents panel, scrolled to where you are, whose every row jumps. The phase was
never really about parsing — `rbook` hands over the tree — but about ToC and spine being a
**many-to-many** mapping (see [`glossary.md`](glossary.md)), which the bundled fixture makes
concrete at 15 spine items against 18 entries. "Which chapter am I in?" got a chosen answer
and a defence, not an array lookup. **117 tests green.**

[Phase 4 — Themes & Typography](milestones/03-reader-enhancements/04-themes-typography/phase-4-theming.md)
**closed** before it: six settings (theme, font size, line height, page margins, max line
length, font family) layer over the publisher's CSS without replacing it, persist across
restarts, and re-anchor the reading position after every reflow. 108 tests green.

The **July 2026 review backlog is now empty** — **R6** (case-insensitive matching, the
"Page 1 of 0" label) landed as a sitting inside Phase 4, and **R3** (a real error type) went
into Phase 7 as Step 6a.

> **Milestone 1's board is stale.** It reads ⬜ while the toolchain it describes has plainly
> been in daily use for three milestones. Its phases need reconciling against reality — the
> one criterion that may genuinely be unmet is debugging from NeoVim
> (`:RustLsp debuggables`).
</content>
