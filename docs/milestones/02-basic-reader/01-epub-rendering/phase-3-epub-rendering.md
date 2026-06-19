# Phase 3 — EPUB Rendering

[← Feature: EPUB Rendering](README.md) · **Status:** 🚧 in progress (Slice 1 underway) ·
build log: [`phase-3-epub-rendering-steps.md`](phase-3-epub-rendering-steps.md)

## Goal

Add `rbook` and render a real `.epub`: open a file → read the spine → display a chapter's
XHTML in the webview, with working page turns.

> **Build order (ADR-0002, dogfood-driven).** This phase is approached as
> [Slice 1](../../../vision-mvp-reader.md) first: open the bundled book, render the current
> spine item **raw** (`dangerous_inner_html`), Next/Prev to walk it. The "Known constraints"
> below describe the *eventual faithful* renderer — the iframe + asset-protocol items are
> the deferred **faithful-styling unlock**, pulled in when a broken cover / missing styles
> become the worst real annoyance, not up front. Per-step detail in the build log.

## Planned steps

- [ ] Add `rbook` to `Cargo.toml` (`rbook = "0.7"`)
- [ ] Open an `.epub` with `Epub::open`; read metadata, manifest, spine, TOC
- [ ] Register a `use_asset_handler` custom protocol that reads resource bytes out of the
      EPUB zip and `responder.respond(bytes)` — minding the `wry://` vs `http://wry.`
      scheme split across platforms
- [ ] Render the current spine item in a **sandboxed `<iframe>`** (omit `allow-scripts`);
      rewrite OPF-relative URLs (images/CSS/fonts) to the custom-protocol URLs
- [ ] Page turns: start with continuous vertical scroll; spike CSS multi-column +
      `translateX` for true pagination
- [ ] Intercept internal hyperlinks → navigation events (next/prev spine item)
- [ ] Bundle a small DRM-free sample `.epub` for testing

## Known constraints (from research)

- Render in an **iframe** (not `dangerous_inner_html`) for style isolation — the book's
  CSS would otherwise leak into the app.
- Resource paths are **OPF-relative** — rewrite them or inject `<base href>`.
- **Pagination happens in the webview (CSS/JS)**, not in Rust; Rust holds nav state.
- **DRM-free EPUBs only** (no LCP/ADEPT).

## Reference

[`RESEARCH.md`](../../../../RESEARCH.md) §3 ·
[`use_asset_handler`](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/fn.use_asset_handler.html) ·
[rbook](https://docs.rs/rbook/latest/rbook/) ·
[ReadiumCSS pagination](https://github.com/readium/css/blob/master/docs/CSS03-injection_and_pagination.md).
</content>
