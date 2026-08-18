# Feature: ToC & Navigation

[← Milestone 3: Reader Enhancements](../README.md)

**Outcome:** the reader knows what it is showing you. The nav bar says
*"I. A Scandal in Bohemia"* instead of *"Chapter 3 of 15"*, and a contents panel lists the
book's own chapter tree so you can jump to any of them. **Status:** ✅ done — shipped by
[Phase 8](phase-8-toc-navigation.md), closed 2026-08-18.

> **Bookmarks are not in it.** The milestone board files them under this feature's heading, but
> Phase 8 deferred them out on day one — they need their own table and belong with a later
> persistence step. Nothing schedules them yet; whoever picks them up opens Phase 9 here rather
> than reopening Phase 8.

## Why this, why now

*Written when the feature opened; kept in the present tense it was written in. Phase 8 has
since answered all of it.*

Phase 4 closed Themes & Typography, and the reader is now pleasant to *look* at while still
being blind to its own structure. Nothing in `src/` reads a ToC today: `open_with_spine`
collects the spine's hrefs and the whole navigation layer — `nav.rs`, the Prev/Next
buttons, the restored `Locator` — addresses chapters by **spine index**. The nav bar's
"Chapter 3 of 15" is that index plus one. It is not wrong, it is just not a name.

This is also the cheapest of Milestone 3's remaining candidates to *start* and the most
useful to *finish*: `rbook` hands over the parsed tree, and two of the pieces the phase
needs already exist — `LinkTarget { spine_index, fragment }` and
`ReaderState::follow_link`, built in Phase 3 for the in-book `<a href>` bridge. A ToC entry
**is** an internal link. The last step of this phase is mostly wiring.

## The crux

**The ToC and the spine are two different coordinate systems, and the map between them is
many-to-many.** The spine is a flat, totally-ordered list of documents. The ToC is a tree of
*labels pointing at hrefs*, and an href may carry a fragment. Nothing requires the two to
line up, and in the bundled fixture they emphatically do not:

| | |
|---|---|
| spine items | 15 |
| ToC entries | 18 |
| spine item 0 (`wrap0000.xhtml`, the cover) | **no ToC entry at all** |
| spine item 1 (`…-h-0`) | **two** entries — "The Adventures of Sherlock Holmes", "Contents" |
| spine item 2 (`…-h-1`) | **four** entries — the chapter plus three nested sub-entries |
| distinct documents named by the ToC | 14 of the 15 |

So "which chapter am I in?" is a real question with a chosen answer, not an array lookup —
and that choice is what the phase is actually about. See
[`glossary.md`](../../../glossary.md), which already draws this line: a **chapter** is a
*navigational* concept, and is not the same thing as a spine item.

The insight that keeps it small: **flatten the tree once, at the boundary, into owned
plain-data entries.** `rbook`'s `EpubTocEntry<'ebook>` borrows the `Epub`, so it can never
be stored in a signal or a `Store` (both want `'static`). Discharging that borrow at open
time — exactly as `spine_hrefs` already does for the spine — turns everything downstream
into index arithmetic over a `Vec`, which `cargo test` can reach.

## Phases

| # | Phase | Outcome | Status |
|---|-------|---------|--------|
| 8 | [ToC & Navigation](phase-8-toc-navigation.md) | Flatten the ToC, name the current chapter, and jump to any entry | ✅ |

## Reference

[EPUB 3 Navigation Document](https://www.w3.org/TR/epub-33/#sec-nav) ·
[EPUB 2 NCX](https://idpf.org/epub/20/spec/OPF_2.0_final_spec.html#Section2.4.1) ·
[`rbook::epub::toc`](https://docs.rs/rbook/0.7.9/rbook/epub/toc/index.html) ·
[Glossary](../../../glossary.md) (ToC, spine, chapter, fragment).

> **Numbering.** The directory is `05-` (the second feature opened under Milestone 3, after
> `04-themes-typography`); the phase is `8` because phase numbers run as one sequence across
> the whole project and 7 was Reading Position.
