# Milestone 2 — Basic EPUB Reader

[← Roadmap](../../roadmap.md)

**Goal:** open an `.epub` from disk and read it on the desktop — paginated, with the
reading position remembered between launches. The minimum viable reader to build on.

**Status:** ⬜ planned

## Features

| # | Feature | Outcome | Status |
|---|---|---|---|
| 1 | [EPUB Rendering](01-epub-rendering/README.md) | Parse with `rbook`, render a spine item in a webview iframe | ⬜ |
| 2 | [Library & Import](02-library/README.md) | Add `.epub` files; list books with covers | ⬜ |
| 3 | [Reading Position](03-reading-position/README.md) | Persist & restore where you stopped | ⬜ |

## Exit criteria

- [ ] Import an `.epub` and see it in a library list
- [ ] Open it and turn pages
- [ ] Reopen the app → it resumes at the same spot

## Stack (from research)

Parse with **`rbook`**, render each spine item in a **sandboxed `<iframe>`**, serve
EPUB-internal resources via **`use_asset_handler`**, persist with **`rusqlite`**. See
[`RESEARCH.md`](../../../RESEARCH.md) §3–4, §6.
</content>
