# Glossary — EPUB reader domain

The ubiquitous language for `ook-reader`. Terms are grounded in the EPUB 3 spec and the
[`rbook`](https://docs.rs/rbook) API (our parser), not invented. Keep this current as the
domain model sharpens — it is the shared vocabulary that ADRs, code, and docs all draw on.

Built/refined during grilling sessions (see [`docs/adr/`](adr/) for the decisions that cite
these terms).

## EPUB structure

- **Publication** — one EPUB: a zip container holding a package document (`.opf`) plus all
  resources. The thing a user opens.
- **Manifest** — the complete, *unordered* catalog of every resource in the publication
  (content documents, images, CSS, fonts), each with an id, href, and media type. A lookup
  table, not a sequence. *(rbook: `manifest`, manifest entries; `.kind()` gives the media
  type.)*
- **Content document** — a single XHTML file listed in the manifest; the actual markup that
  gets rendered in the webview.
- **Spine** — the linear **reading order**: an *ordered* list of references (itemrefs) into
  the manifest, each pointing to a content document by idref. Answers "what comes next."
  Carries **no human-readable titles**. Items may be marked `linear="no"` (auxiliary, e.g.
  pop-up footnotes) vs the default `linear="yes"`. *(rbook: `spine`, spine entries created
  by idref, e.g. `spine.push("chapter_1")`.)*
- **Spine item (itemref)** — one entry in the spine: a pointer to one content document, in
  reading-order position.
- **Table of Contents (ToC) / Navigation document** — a **tree** of navigation points
  (EPUB 3: `nav.xhtml`; EPUB 2: `toc.ncx`). Each node has a human **label** and an **href**
  that may include a **fragment**. *(rbook: `toc`, `TocEntry`, `TocEntryKind`, with
  `.label()`, `.href()`, `.children()`.)*
- **ToC entry (nav point)** — one node in the ToC tree: label + href (+ child entries).
- **Fragment** — the `#id` suffix of an href pointing to a location *within* a content
  document (e.g. `c1.xhtml#section-a`). Two ToC entries can target the same file at
  different fragments.

## Theming & rendering

Grounded in [Readium CSS](https://github.com/readium/readium-css) (the de-facto reader
theming model) and [ADR-0003](adr/0003-reader-controlled-theming-injected-layer.md).

- **Reading mode / theme** — a named set of user style values applied to the whole book,
  chiefly background + text colour: **day** (light), **sepia**, **night** (dark). In this
  model a theme is *just* a bundle of `--USER__*` values, so custom themes and caching fall
  out for free.
- **Override layer (cascade)** — the reader's styling expressed as a small CSS layer injected
  *around* the book's own CSS, never replacing it. Three tiers by source order:
  **RS defaults** (injected *before* the book CSS) < **author/publisher CSS** (untouched) <
  **user overrides** (injected *after*). Priority by design: **USER > author > RS**.
- **`--RS__` vs `--USER__` variable** — the two prefixes for the reading-system's CSS custom
  properties. `--RS__*` are reading-system defaults that *lose* to the book; `--USER__*` are
  user settings that *win*. Switching a setting = changing a `--USER__*` value.
- **Advanced-settings flag** — a gate (e.g. a `--USER__*` toggle) that withholds the more
  aggressive overrides (font-family, justification) unless the user opts in, so the reader
  doesn't fight embedded fonts or author `!important` and create invisible-text bugs.
- **Served XHTML (vs `srcdoc`)** — rendering a content document by pointing the iframe at a
  URL served with `Content-Type: application/xhtml+xml`, so the webview parses it as **XML**.
  Contrast `srcdoc`, which parses as **HTML**. The distinction matters: XHTML self-closing
  non-void tags like `<a id="x"/>` are honoured under XML parsing but mis-parsed as unclosed
  under HTML parsing. Served XHTML is also the **injection seam** for the override layer.

## Concepts that are easy to conflate

- **Chapter** — a *navigational* concept = (usually) a ToC entry. **Not** the same as a
  spine item. The ToC↔spine mapping is **many-to-many**: one content document can hold
  several ToC entries (chapters); a chapter can span multiple documents; spine items like
  the cover or front matter may have **no** ToC entry at all.
- **Reading order** — synonym for the spine's order. The sequence "Next" walks.
- **Reading position** — where the user currently is. To survive reopening it must be
  expressed durably (which spine item + an offset/fragment within it). The spec's canonical
  scheme for this is **EPUB CFI** (Canonical Fragment Identifier).
