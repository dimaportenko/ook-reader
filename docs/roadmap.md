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
| 4 | [Multi-platform](milestones/04-multiplatform/README.md) | Mobile (iOS/Android) + web (WASM) from the same codebase | ⏸ |

## Current focus

**Milestone 3 → Themes & Typography →
[Phase 4](milestones/03-reader-enhancements/04-themes-typography/phase-4-theming.md)** — the
MVP reader is done: import a book, page through it, quit, reopen, land where you stopped.
What it can't do is let *you* decide how the page looks — the publisher's CSS still dictates
colour and type. Phase 4 injects a reader-controlled override layer around the book's own
CSS (day / sepia / night first, typography after), and its plan has been written since
[ADR-0003](adr/0003-reader-controlled-theming-injected-layer.md).

Riding alongside: one open item in the
[July 2026 review backlog](milestones/02-basic-reader/review-2026-07-steps.md) — **R6**
hygiene (case-insensitive content types, the "Page 1 of 0" label). It is a behavior fix, so
Phase 7's refactor step deliberately did not absorb it; it lands as a sitting inside Phase 4.
**R3** (a real error type) was pulled into Phase 7 as Step 6a and is done.

> **Milestone 1's board is stale.** It reads ⬜ while the toolchain it describes has plainly
> been in daily use for three milestones. Its phases need reconciling against reality — the
> one criterion that may genuinely be unmet is debugging from NeoVim
> (`:RustLsp debuggables`).
</content>
