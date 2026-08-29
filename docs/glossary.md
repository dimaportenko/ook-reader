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

## Sync

Grounded in [ADR-0005](adr/0005-backendless-sync-google-drive-per-device-shards.md). These
terms describe syncing **without a server**: every device writes only its own file, so
nothing needs arbitrating and the whole problem becomes a pure merge function.

- **Shard** — one device's state file in the remote store, named `state-<device-id>.json`.
  The defining rule is that **exactly one device ever writes a given shard**, and every
  device reads all of them. That single constraint is what makes a backend unnecessary:
  two writers never touch one file, so a storage layer with no conditional writes (Drive has
  none) is still safe.
- **Snapshot (vs operation log)** — a shard holds the device's *current* value for each
  field, not the history of how it got there. Bounded by library size rather than by time,
  self-healing (the next write replaces a corrupt shard), and no compaction scheme to design.
  The rejected alternative, an append-only log, would buy history — "where was I last
  Tuesday" — which nothing needs yet.
- **Merge** — assembling the app's view by reading every shard and taking, per field, the
  value with the highest clock. A pure function over shards: no network, no OAuth, and
  therefore the part of sync that is almost entirely `#[test]`-able.
- **Last-write-wins (LWW)** — the merge rule: newest stamp for a field wins. For a reading
  position this is not a compromise but the *correct* semantic — you want where you most
  recently were.
- **Hybrid logical clock (HLC)** — the stamp LWW compares. Wall-clock seconds for human
  meaning, plus a Lamport counter (`max(seen) + 1`) that provides the actual ordering.
  Wall clock alone is not enough: a device whose clock is wrong stamps the future and pins a
  stale position permanently.
- **Tombstone** — a deletion recorded as a stamped `deleted` flag on the entry rather than
  by removing it. Necessary because in a merged world a plain local delete is simply undone
  by the next merge, the other device's shard still listing the book. Being a field of the
  snapshot rather than a separate log, it needs no garbage collection scheme.
- **Content hash (book identity)** — the SHA-256 of the EPUB's bytes, and the key a position
  is stored against. `books.id` is a local autoincrement and `books.path` / `source_path`
  are local filesystem paths, so none of them name the same thing on two devices. Contrast
  the OPF **`dc:identifier`**, which is *semantic* but untrustworthy — often missing, often
  duplicated across unrelated books, sometimes regenerated per build. It is recorded but not
  used as a key.
- **Device identity** — a uuid v4 minted on first run and stored locally, plus a
  human-readable name. Names the shard. A reinstall makes a new device; the old shard becomes
  inert until *forget this device* deletes it.
- **`appDataFolder`** — Drive's per-app hidden folder, invisible to the user and to other
  apps. Non-sensitive scope, so no Google verification. Its sharp edge: revoking the app's
  access **permanently deletes it, skipping Trash** — no grace period, no undo.
- **`RemoteStore`** — the project's own four-method boundary (list / get / put / delete) with
  Drive behind it. Its purpose is not provider-swapping but **testability**: an in-memory
  fake lets the merge engine be exercised with no network and no OAuth.
