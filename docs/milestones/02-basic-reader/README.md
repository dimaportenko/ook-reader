# Milestone 2 — Basic EPUB Reader

[← Roadmap](../../roadmap.md)

**Goal:** open an `.epub` from disk and read it on the desktop — paginated, with the
reading position remembered between launches. The minimum viable reader to build on.

**Status:** ✅ done — all three features shipped and all three exit criteria met. One item
of the July review backlog (**R6**, hygiene) is still open and rides into the next phase;
it is a small behavior fix, not a milestone blocker.

## Features

| # | Feature | Outcome | Status |
|---|---|---|---|
| 1 | [EPUB Rendering](01-epub-rendering/README.md) | Parse with `rbook`, render a spine item in a webview iframe | ✅ |
| 2 | [Library & Import](02-library/README.md) | Add `.epub` files; list books with covers | ✅ |
| 3 | [Reading Position](03-reading-position/README.md) | Persist & restore where you stopped | ✅ |

## Cross-cutting

Two July 2026 reviews ride alongside the feature phases, each scoped to the whole codebase:

- [`review-2026-07-steps.md`](review-2026-07-steps.md) — a six-item refactor backlog,
  written as independent learn-by-building steps.
- [`performance-review-2026-07.md`](performance-review-2026-07.md) — how the render and
  import paths scale with book size. Findings #1 and #2 traced back to one decision
  (chapters travelling as base64 `data:` URLs) and drove the chapter-transport rework
  below.

The rework itself is a three-document chain filed under the feature it reworks,
[EPUB Rendering](01-epub-rendering/README.md).

## Exit criteria

- [x] Import an `.epub` and see it in a library list
- [x] Open it and turn pages
- [x] Reopen the app → it resumes at the same spot

## Stack (from research)

Parse with **`rbook`**, render each spine item in a **sandboxed `<iframe>`**, serve
EPUB-internal resources via **`use_asset_handler`**, persist with **`rusqlite`**. See
[`RESEARCH.md`](../../../RESEARCH.md) §3–4, §6.
</content>
