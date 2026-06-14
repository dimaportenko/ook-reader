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

## Status legend

✅ done · 🚧 in progress · ⬜ planned · ⏸ deferred

## Milestones

| # | Milestone | Outcome | Status |
|---|---|---|---|
| 1 | [Foundation](milestones/01-foundation/README.md) | Rust/Dioxus NeoVim toolchain + a buildable desktop app | ⬜ |
| 2 | [Basic EPUB Reader](milestones/02-basic-reader/README.md) | Open an `.epub` and read it with paging + saved position | ⬜ |
| 3 | [Reader Enhancements](milestones/03-reader-enhancements/README.md) | The features missing from other readers | ⬜ |
| 4 | [Multi-platform](milestones/04-multiplatform/README.md) | Mobile (iOS/Android) + web (WASM) from the same codebase | ⏸ |

## Current focus

Fresh start on the Dioxus/Rust stack — nothing built yet. Begin at
**Milestone 1 → Dev Environment → Phase 0** (Rust + NeoVim toolchain), then Phase 2
(Dioxus desktop scaffold). Rust + Dioxus fundamentals run in parallel.
</content>
