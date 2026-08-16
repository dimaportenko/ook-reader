# Phase 8 — ToC & Navigation

[← Feature: ToC & Navigation](README.md) · **Status:** 🚧 in progress ·
build log: [`phase-8-toc-navigation-steps.md`](phase-8-toc-navigation-steps.md)

## Goal

Open a book and see **what you are reading**, by name, and be able to go to any chapter by
picking it off a list. Concretely: the nav bar shows the current entry's label instead of
"Chapter 3 of 15", and a contents panel renders the book's own nested ToC with each row a
jump target.

## The crux

**The ToC and the spine are two coordinate systems, and the map between them is
many-to-many.** The spine is a flat ordered list of documents and is what `nav.rs`
navigates by index. The ToC is a *tree of labels pointing at hrefs*, each href optionally
carrying a fragment. In the bundled Sherlock Holmes fixture the mismatch is not theoretical:
15 spine items against 18 ToC entries, the cover with no entry at all, one document carrying
two entries, another carrying four.

So the phase has three distinct problems hiding inside "add a ToC", and each gets its own
step:

- **entry → spine index** (jumping). Well-posed; every entry names exactly one document.
- **spine index → entry** (labelling). *Not* well-posed — zero, one, or four answers. The
  step that does it has to pick a rule and defend it.
- **crossing the borrow.** `EpubTocEntry<'ebook>` borrows the `Epub`. Nothing that borrows
  can live in a `Signal` or a `Store`.

The insight that makes it cheap: **flatten once at the boundary into owned plain data**, the
way `spine_hrefs` already discharges the spine's borrow at `open_with_spine`. After that,
every remaining question is a pure function over a `Vec` — which means `cargo test`, not
`dx serve`, answers most of this phase.

The other economy: **the jump target already exists.** Phase 3 built
`epub::LinkTarget { spine_index, fragment }` and `nav::ReaderState::follow_link` for the
in-book `<a href>` bridge, and a ToC entry is an internal link wearing a label. The last
feature step is wiring, not new machinery.

## Design decisions (recorded up front)

- **A new module, `src/toc.rs`.** `epub.rs` is already 820 lines and owns "talk to rbook and
  serve bytes"; `nav.rs` owns "where am I and how do I move". The ToC is a third thing —
  a parsed, owned model of the book's structure — and it depends on both. Putting it in its
  own file keeps each of the three readable and gives the phase's pure functions an obvious
  home.
- **Depth is normalized to 0-based at the boundary.** `rbook` reports the synthetic root as
  depth 0, so its children come back at depth 1. That root has no label and no href and never
  appears in the flattened list, so an entry the *reader* sees at the top level reporting
  depth 1 is an off-by-one waiting to be forgotten in the indent arithmetic. Subtract once,
  at the one place that crosses the boundary.
- **Entries that resolve nowhere are dropped, not kept as dead rows.** An entry with no href,
  or whose href names a document the spine does not contain, cannot be navigated to; a row
  that does nothing when clicked is worse than an absent row. The alternative — keeping the
  entry with `spine_index: Option<usize>` and greying it out — costs an `Option` in every
  downstream match for a case no well-formed book produces. Revisit if a real book loses
  entries this way.
- **Labelling picks the *first* entry naming the current document**, falling back to the
  nearest preceding entry, and to nothing at all before the first one. Rationale in Step 3.
  Fragment-level precision — knowing you are inside sub-entry "II." rather than at the top of
  chapter I — is **out of scope**: that is the resolve-a-position-in-the-live-DOM problem
  that Milestone 3's board defers for highlights and jump-to-search-hit alike.
- **A book with no ToC is a normal book, not an error.** `contents()` returns `Option`, and
  the answer to `None` is an empty `Vec`, a hidden contents button, and the existing
  "Chapter 3 of 15" label. No new error variant.

## Planned steps

Detail for each lives in
[`phase-8-toc-navigation-steps.md`](phase-8-toc-navigation-steps.md).

- [x] **1. Flatten the ToC into owned entries** — `src/toc.rs`, `TocEntry { label, depth,
      href }`, `toc_entries(&Epub)`. Crosses the borrow; asserts DFS order and nesting.
      `#[test]`. Committed in `7a70db5`, **109 tests green**; the assertion was verified by
      mutating the test rather than watched red first.
- [x] **2. Resolve each entry to a spine index and a fragment** — split the href, look the
      path up in the spine, drop what does not resolve. Pins the many-to-many facts of the
      fixture. `#[test]`. Committed in `cdd91bf`, **111 tests green**; all three assertions
      verified by mutation.
- [x] **3. Name the current chapter** — `entry_index_for_spine(&entries, spine_index)`, the
      reverse lookup, with the first-match/fall-back-to-preceding rule. `#[test]`. Committed
      in `2249211`, **113 tests green**; written by `lbb:next-implement` and watched red,
      then re-verified by mutation after the `simplify` pass changed the return type.
- [ ] **4. Show the name in the nav bar** — the label with "Chapter N of M" as the fallback.
      Pure `chapter_label` gets a `#[test]`; the bar itself is a `dx serve` eyeball.
- [ ] **5. Render the contents panel** — the entries as a list indented by depth, in the
      existing popover. `dx serve` + `cargo clippy`.
- [ ] **6. Jump to an entry** — convert a `TocEntry` into `epub::LinkTarget` and hand it to
      the existing `follow_link`. `#[test]` for the conversion, eyeball for the jump.
- [ ] **7. Review and refactor** — the phase-closing pass.

## Out of scope

Bookmarks (the milestone board lists them alongside ToC; they need their own table and
belong with a later persistence step), `landmarks` / `page-list` — the other two ToC kinds
`rbook` exposes, both absent from the fixture — and any fragment-precise "you are here"
highlighting inside the panel.
