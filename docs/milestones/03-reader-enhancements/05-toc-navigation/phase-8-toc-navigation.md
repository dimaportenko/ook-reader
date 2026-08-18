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
- [x] **4. Show the name in the nav bar** — the label with "Chapter N of M" as the fallback.
      The first step to *call* `toc_entries`, once per mounted book via `use_hook`. Committed
      in `24ad8bd`, **114 tests green**; written by `lbb:next-implement` and watched red on
      the toc arm, the two fallback assertions verified by mutation. It landed in the header
      under the book title rather than in the chapter nav row — better home for "what am I
      reading", and the row keeps the label too for now.
- [x] **5. Render the contents panel** — the entries as a list indented by depth, in the
      existing popover. `dx serve` + `cargo clippy`. Committed in `20535be`, **114 tests
      green** (unchanged — the phase's first step with no test, gated on a clean build plus
      the `dx serve` pass). Written by `lbb:next-implement`. The trigger landed in the header
      beside the settings gear rather than in the chapter `NavRow`, so Step 4's question about
      that row's duplicate label is still open.
- [x] **6. Jump to an entry** — convert a `TocEntry` into `epub::LinkTarget` and hand it to
      the existing `follow_link`. `#[test]` for the conversion, eyeball for the jump.
- [x] **7. Review and refactor** — the phase-closing pass. Nine carried items, too many for one
      diff, so it was **sketched as a triage and landed as lettered sittings** the way Phase 4's
      Step 8 did. One item was closed before the lettering started: Step 4's duplicated chapter
      label in the `NavRow` went in `c5e6d37`. The rest landed across 7a–7h below, ending at
      `75a5cde`; **117 tests green**, every carried item closed, nothing dropped.
  - [x] **7a. One resolver, one target** — `epub::link_target` becomes the single path→spine
        resolver both a ToC entry and an in-book `<a href>` go through, and `TocEntry` carries a
        `LinkTarget` instead of a loose `spine_index`/`fragment` pair. `#[test]`. Committed in
        `54e8ea7`, **115 tests green**; written by `lbb:next-implement`, the red was a compile
        error and both halves of the new assertion were verified by mutation — dropping the trim
        takes the fixture's ToC from 18 entries to 0.
  - [x] **7b. Pin `--toc-depth` across the Rust/CSS gap** — the repo's `assert!(…contains…)`
        idiom for a name two files share with no compiler between them. `#[test]`. Committed in
        `fae2125`, **116 tests green**; written by `lbb:next-implement`, the red was a compile
        error, and the assertion was proved live in both directions plus a control — renaming
        either side fails, deleting the dead `, 0` fallback does not.
  - [x] **7c. One icon component** — item B alone. The tabler SVG preamble on its third copy moves
        into `ui/components/icon.rs`, and the geometry goes with it: `TablerIcon`'s private fields
        make `CLOSE`/`SETTINGS`/`LIST` the only values that exist, so the name and the paths cannot
        disagree. Committed in `6a16fe3`, **116 tests green** (unchanged — no test; a markup
        extraction). Written by `lbb:next-implement`. Verified against `HEAD` instead of by test:
        identical preamble, identical paths, and each const matched to its origin file in order.
        **The `dx serve` eyeball is still outstanding.**
  - [x] **7d. The popover owns its keyboard** — items D and F: the duplicated `onkeydown` guard
        moves into the shared `PopoverContent`, and `settings.rs`'s controlled-open triple turns out
        to be three dead lines re-implementing `use_controlled`'s internal signal by hand. Committed
        in `89a0d7c`, **116 tests green** (unchanged — no test; nothing in the crate can see rendered
        markup). Written by `lbb:next-implement`. The gate was `cargo clippy` plus the `dx serve`
        eyeball, confirmed by hand: arrows in the settings `<select>` no longer turn the page, and
        the panel still opens and closes with the triple deleted.
  - [x] **7e. The frame swallows the outside click** — not a triage item: a bug found by using
        the panel. `use_outside_dismiss` listens on the *host* `document`, and a press inside the
        book's iframe never gets there, so the panel stays open over the one surface it covers.
        The frame posts the press and the host replays it on the frame element. Committed in
        `deecc44`, **117 tests green**; written by `lbb:next-implement` and watched red, the two
        assertions the first one shadowed re-proved by mutation at commit time, and the `dx serve`
        dismissal confirmed by hand.
  - [x] **7f. `ul`/`li` list semantics for the contents panel** — item E, split out of 7d: eighteen
        bare `button`s announce as eighteen buttons with no list and no count. Committed in
        `b92816f`, **117 tests green** (unchanged — no test; nothing in the crate can see the
        accessibility tree). Written by `lbb:next-implement`. The `dx serve` gate **failed the
        first time** and caught a real regression: `display: block` does not widen a `<button>`,
        which sizes to `fit-content` whatever its `display`, so the hover highlight hugged each
        label until `box-sizing: border-box; width: 100%` went on. The handoff had blamed the
        wrong declaration — the `li`, not the dropped flex column, is what took the stretch away.
  - [x] **7g. `toc::label_for_spine`** — Step 4's item; deletes the index-then-index dance from
        three call sites. Committed in `a3c4ad7`, **117 tests green** (unchanged — no test was
        owed; the check was the two existing tests rewritten to call the new function). Written by
        `lbb:next-implement`. The red was a compile error at both call sites, and because that
        proves only that the tests *reach* the function, each rewritten assertion was then
        re-proved by mutation — including the fall-back-to-preceding branch.
  - [x] **7h. Scroll the current row into view** — carried from Step 5, and the phase's only
        behaviour change since Step 6, so it took its own commit. `onmounted` + `scroll_to` on the
        row itself, because the row does not exist until the panel opens — **plus
        `is_modal: false`**, because the popover's default focus trap focuses row one on open and
        focusing scrolls, which silently undid the first draft. Committed in `75a5cde`, **117 tests
        green** (unchanged — no test is possible in-crate for either claim). Written by
        `lbb:next-implement`. **The `dx serve` gate failed the first time** and is what found the
        trap; the corrected version was confirmed by hand on a book whose ToC overflows `80vh`.

  > **7c was split.** The triage bundled B + D + E + F as one "UI-chrome sitting" because they
  > touch the same three files, but that is four ideas and the icon extraction alone is a new
  > module plus three call sites. B became 7c; D + E + F became 7d; the two items after them
  > shifted a letter. Nothing was dropped.
  >
  > **Then 7d was split the same way.** D + F are one idea seen twice — which lines belong to the
  > popover and which to its call sites — and both are gated on pressing keys in a running app. E
  > rewrites the panel's DOM and CSS and is gated on reading markup. E became 7e; the two items
  > after it shifted a letter again. Nothing was dropped.
  >
  > **Then a bug jumped the queue.** The contents panel does not dismiss when you click the book,
  > which is most of the window — a defect in what Steps 5 and 6 shipped, not a polish item, so it
  > goes before the polish. It took the free letter and the three planned items shifted once more,
  > which keeps the letters reading in landing order. Nothing was dropped.

## Out of scope

Bookmarks (the milestone board lists them alongside ToC; they need their own table and
belong with a later persistence step), `landmarks` / `page-list` — the other two ToC kinds
`rbook` exposes, both absent from the fixture — and any fragment-precise "you are here"
highlighting inside the panel.
