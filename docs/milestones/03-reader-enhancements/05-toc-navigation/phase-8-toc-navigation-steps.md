# Phase 8 — ToC & Navigation — build log

[← Phase doc](phase-8-toc-navigation.md)

Per-step test → minimal code → why, appended newest-last. The
[phase doc](phase-8-toc-navigation.md)'s "Planned steps" checklist is the high-level index;
this file is the detail and the build log.

## The crux

**The ToC and the spine are two coordinate systems, and the map between them is
many-to-many.** The spine is the flat ordered list of documents that `nav.rs` addresses by
index. The ToC is a *tree of labels pointing at hrefs*, each href optionally carrying a
fragment. Nothing makes them line up, and the bundled fixture proves it:

```
spine                                    toc
 0  wrap0000.xhtml            ──►  (nothing — the cover has no entry)
 1  …-h-0.htm.xhtml           ──►  "The Adventures of Sherlock Holmes"
                              ──►  "Contents"
 2  …-h-1.htm.xhtml           ──►  "I. A SCANDAL IN BOHEMIA"
                              ──►    "I."      (nested, depth 1)
                              ──►    "II."
                              ──►    "III."
 3  …-h-2.htm.xhtml           ──►  "II. THE RED-HEADED LEAGUE"
 …                                  …
14  …-h-13.htm.xhtml          ──►  "THE FULL PROJECT GUTENBERG™ LICENSE"

15 spine items · 18 toc entries · 14 documents named · max depth 2
```

Three separate problems live inside "add a ToC", and each gets its own step:

- **entry → spine index** is well-posed — every entry names exactly one document.
- **spine index → entry** is *not* — zero, one, or four answers. Whichever step does it has
  to choose a rule and say why.
- **crossing the borrow** — `EpubTocEntry<'ebook>` borrows the `Epub`, and nothing that
  borrows can live in a `Signal` or a `Store`, both of which want `'static`.

What keeps the phase small: **flatten once, at the boundary, into owned plain data** — the
same move `spine_hrefs` already makes for the spine at `open_with_spine`. Past that line
every remaining question is a pure function over a `Vec`, so `cargo test` answers most of
this phase and `dx serve` only has to confirm the pixels.

And the jump target is already built. Phase 3 gave us
`epub::LinkTarget { spine_index, fragment }` and `nav::ReaderState::follow_link` for the
in-book `<a href>` bridge; a ToC entry is an internal link wearing a label. Step 6 is wiring.

## Step plan

1. **Flatten the ToC into owned entries** — `src/toc.rs`, `TocEntry { label, depth, href }`,
   `toc_entries(&Epub)`. The borrow-crossing step. `#[test]`.
2. **Resolve to a spine index and a fragment** — split the href, look the path up in the
   spine `Vec<String>`, drop what does not resolve. `#[test]`.
3. **Name the current chapter** — `entry_index_for_spine`, the reverse lookup, with its chosen
   rule. `#[test]`.
4. **Show the name in the nav bar** — pure `chapter_label` under `#[test]`, the bar itself an
   eyeball.
5. **Render the contents panel** — indented by depth, in the existing popover. Eyeball.
6. **Jump to an entry** — `TocEntry` → `LinkTarget` → `follow_link`. `#[test]` + eyeball.
7. **Review and refactor** — the phase-closing pass.

---

## Step 1 — Flatten the ToC into owned entries

The whole step is one boundary crossing: take `rbook`'s borrowed tree and hand back a flat
`Vec` of owned rows that Rust will let you put in a signal. No spine resolution yet, no UI —
just proof that the tree came across intact, in order, with its nesting still legible.

New file, `src/toc.rs`, and a `mod toc;` line in `main.rs` alongside the other module
declarations.

### The check — `cargo test`

Add this to a `#[cfg(test)] mod test` at the bottom of the new `src/toc.rs`. It should fail
to compile first (nothing to call yet); that red is the point.

```rust
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_toc_flattens_to_a_depth_tagged_list_in_reading_order() {
        let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");

        let entries = toc_entries(&epub);

        assert_eq!(entries.len(), 18);

        assert_eq!(entries[0].label, "The Adventures of Sherlock Holmes");
        assert_eq!(entries[0].depth, 0);
        assert_eq!(
            entries[0].href,
            "OEBPS/5186027266282590649_1661-h-0.htm.xhtml#pgepubid00000"
        );

        assert_eq!(entries[2].label, "I. A SCANDAL IN BOHEMIA");
        assert_eq!(entries[2].depth, 0);
        assert_eq!(entries[3].label, "I.");
        assert_eq!(entries[3].depth, 1);
        assert_eq!(entries[5].label, "III.");
        assert_eq!(entries[5].depth, 1);
        assert_eq!(entries[6].label, "II. THE RED-HEADED LEAGUE");
        assert_eq!(entries[6].depth, 0);

        assert_eq!(entries[17].label, "THE FULL PROJECT GUTENBERG™ LICENSE");
    }
}
```

Run it on its own while you work:

```
cargo test the_toc_flattens
```

(No `--lib` — `ook-reader` is a binary crate with no library target, so `cargo test --lib`
errors with *"no library targets found"*. The filter alone is enough.)

The three assertions on indices 2 → 3 → 5 → 6 are doing more work than they look. They pin
**depth-first order**: the three nested sub-entries land *immediately after* their parent and
*before* the next top-level chapter. A breadth-first flatten, or a flatten that lost the
nesting, changes those indices and the test goes red. That is the structure of the tree,
asserted through a flat list.

### The code

```rust
use rbook::Epub;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) href: String,
}

pub(crate) fn toc_entries(epub: &Epub) -> Vec<TocEntry> {
    let Some(root) = epub.toc().contents() else {
        return Vec::new();
    };

    root.flatten()
        .filter_map(|entry| {
            Some(TocEntry {
                label: entry.label().to_string(),
                depth: entry.depth().saturating_sub(1),
                href: entry.href()?.decode().trim_start_matches('/').to_string(),
            })
        })
        .collect()
}
```

### Why it works

**`.to_string()` on every field is the whole point, not laziness.** `entry.label()` returns
`&'ebook str` — a borrow of the `Epub`. So does the href. Keep any of them and `TocEntry`
inherits the lifetime, and a `TocEntry<'a>` can never go into a `Signal`, a `Store`, or a
component prop, all of which require `'static`. Copying the strings *discharges* the borrow:
after `toc_entries` returns, the `Vec` owes the `Epub` nothing and can outlive it. This is
`spine_hrefs` again — same reason, same shape, one file over.

**`let … else` because there is nothing to salvage.** `contents()` is `Option` because a book
may ship no navigation document at all, and that is a legal book, not an error. `let Some(x)
= … else { return … }` handles the absent case and then leaves `root` bound, unwrapped, for
the rest of the function — where `if let Some(root) = …` would have indented everything that
follows inside it. Early-return-and-flatten is the idiom whenever the `None` arm is trivial.

**`filter_map` with `?` inside a closure.** The `?` on `entry.href()?` returns from *the
closure*, not from `toc_entries` — inside a closure returning `Option<TocEntry>`, `?` is
"give up on this one item". `filter_map` then drops the `None`s. So an entry with no href
(EPUB 2 allows a `navPoint` with a label and no `content`) silently disappears rather than
becoming a row that does nothing when clicked. That is the drop policy the phase doc records;
Step 2 applies the same rule to hrefs that resolve to no spine item.

**`saturating_sub(1)` normalizes depth.** `rbook` counts from a synthetic root at depth 0, so
the entries you actually see arrive at depth 1 and 2. That root has no label and no href and
never appears in the flattened list, so exposing its numbering downstream just plants an
off-by-one in the indent arithmetic Step 5 will write. Subtract once, here, at the only place
that crosses the boundary. `saturating_sub` rather than `- 1` because the root itself is
never yielded by `flatten()` — but a `usize` underflow panics, and a defensive zero beats a
crash if a future `rbook` ever changes its mind.

**`.decode()` and the leading slash.** `href()` gives you rbook's *normalized* href —
absolute within the archive, percent-encoded — as an `Href`, and `.decode()` turns it into a
`Cow<str>` with the encoding undone. rbook writes it as `/OEBPS/…`; `spine_hrefs` already
trims that leading `/` when it builds `docs`, so trimming here too means Step 2 can compare
the two strings directly instead of discovering the mismatch at runtime. Note the href still
carries its `#fragment` at this step — splitting it is Step 2's job.

### Scope note

This step deliberately stops at `href: String`. Step 2 **replaces that field** with
`spine_index: usize` and `fragment: Option<String>` — the form the reader can actually
navigate with. Carrying the raw href for one step is not wasted work: it is what lets this
test assert that the *right string* crossed the boundary, before any interpretation of it can
go wrong. Nothing renders yet, and `toc_entries` is not called from anywhere outside its test
until Step 4.

> **Status:** done — committed in `7a70db5`, **109 tests green** (108 → 109).
>
> The test was written *with* the implementation rather than watched red first, so it was
> verified by mutation instead: flipping `entries[3].depth` from `1` to `0` — the nesting
> assertion, the one carrying the depth-first claim — produced
> `assertion left == right failed / left: 1 / right: 0` at `src/toc.rs:48`, and the file was
> then diffed byte-for-byte against a pre-mutation copy. The mutation was applied to the test
> only; the implementation was never touched to produce a red run.
>
> `cargo clippy --all-targets` is clean apart from two `dead_code` warnings naming `TocEntry`
> and `toc_entries`. They are correct — `#[cfg(test)]` does not count as a use for the binary
> target, and there is no caller until Step 4 — and were left un-silenced, because an
> `#[allow(dead_code)]` added to cover three steps is the kind that never comes back off.
>
> One correction to the step as written: it prescribed `cargo test --lib the_toc_flattens`,
> which fails with *"no library targets found in package `ook-reader`"*. `ook-reader` is a
> binary crate; the filter alone is the whole command.

---

## Step 2 — Resolve each entry to a spine index and a fragment

Step 1 brought the tree across the borrow but left every entry pointing at a **string** the
reader cannot act on. This step turns that string into the two things `nav.rs` already
navigates by: a **spine index** and an optional **fragment**. It is where the many-to-many
mapping stops being a claim in the phase doc and becomes assertions in a test.

`toc_entries` grows a second parameter — the spine `Vec<String>` that `open_with_spine`
already returns and the reader already holds as `book.docs`.

### The check — `cargo test`

Two tests, replacing nothing: the Step 1 test keeps asserting order and nesting, these
assert resolution. Both go in `src/toc.rs`'s `mod test`.

```rust
#[test]
fn many_entries_can_name_one_spine_item() {
    let (epub, docs) =
        epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

    let entries = toc_entries(&epub, &docs);

    assert_eq!(docs.len(), 15);
    assert_eq!(entries.len(), 18);

    assert_eq!(entries[0].spine_index, 1);
    assert_eq!(entries[0].fragment.as_deref(), Some("pgepubid00000"));
    assert_eq!(entries[1].label, "Contents");
    assert_eq!(entries[1].spine_index, 1);

    for entry in &entries[2..=5] {
        assert_eq!(
            entry.spine_index, 2,
            "{:?} is in the same document as its siblings",
            entry.label
        );
    }

    assert!(
        !entries.iter().any(|entry| entry.spine_index == 0),
        "the cover is in the spine but in no toc entry"
    );

    assert_eq!(entries[17].spine_index, 14);
}

#[test]
fn an_entry_naming_no_spine_item_is_dropped() {
    let (epub, docs) =
        epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

    let entries = toc_entries(&epub, &docs[..3]);

    assert_eq!(entries.len(), 6);
    assert_eq!(entries[5].label, "III.");
}
```

Run them with `cargo test toc::`.

The first test is the phase's crux written as assertions, and every line is a fact about the
bundled book rather than a hypothetical:

- **18 entries over 15 spine items** — the two lists are not the same length and never were.
- **`entries[0]` and `entries[1]` both resolve to spine 1** — one document, two entries.
- **`entries[2..=5]` all resolve to spine 2** — one document, four entries, three of them
  nested. This is the case that makes Step 3's reverse lookup a *choice*.
- **nothing resolves to spine 0** — the cover is a real spine item that the ToC never names,
  so "the current chapter" is sometimes legitimately nameless.

The second test exercises the drop branch without needing a malformed fixture: hand the
function a **truncated spine** and every entry naming a document past index 2 has nowhere to
land. 18 entries in, 6 out. That is the drop policy demonstrated rather than described.

### The code

Replace `href: String` in the struct, and the closure body:

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) spine_index: usize,
    pub(crate) fragment: Option<String>,
}

pub(crate) fn toc_entries(epub: &Epub, docs: &[String]) -> Vec<TocEntry> {
    let Some(root) = epub.toc().contents() else {
        return Vec::new();
    };

    root.flatten()
        .filter_map(|entry| {
            let href = entry.href()?;
            let path = href.path().decode();
            let path = path.trim_start_matches('/');

            Some(TocEntry {
                label: entry.label().to_string(),
                depth: entry.depth().saturating_sub(1),
                spine_index: docs.iter().position(|doc| doc == path)?,
                fragment: href.fragment().map(|fragment| {
                    percent_encoding::percent_decode_str(fragment)
                        .decode_utf8_lossy()
                        .into_owned()
                }),
            })
        })
        .collect()
}
```

### Why it works

**`href.path()` splits the fragment off for you, and it costs nothing.** `Href` is a newtype
over `&str` — `pub struct Href<'a>(&'a str)` — so `path()` returns another `Href` borrowing
the same buffer up to the `?` or `#`, and `fragment()` returns the slice after the `#`.
There is no parsing and no allocation in either. That is why the code reaches for rbook's
accessors instead of `split_once('#')`: the library already knows where the boundaries are,
including the query string that `split_once('#')` would silently leave glued to the path.

**Two `let path` bindings, and the shadowing is load-bearing.** `decode()` returns
`Cow<'a, str>` — owned when the href was percent-encoded, borrowed when it wasn't. You cannot
write `let path = href.path().decode().trim_start_matches('/')` in one line: `trim_start_matches`
borrows *from* the `Cow`, and the `Cow` would be a temporary dropped at the end of the
statement, leaving the `&str` dangling. Binding it first gives the `Cow` a name that lives to
the end of the closure; the second `let path` then shadows it with a `&str` borrowed from it.
The compiler will insist on this — it's worth recognizing the error message rather than
fighting it.

**`.position()` is where "many-to-many" actually happens.** It returns the *first* index whose
document matches, and it is called once per entry — so four entries all naming `…-h-1` all get
`2`, independently, and nothing notices they collided. The mapping is many-to-one in this
direction and that is fine; it is the *other* direction, in Step 3, where "first" stops being
obviously right.

**The second `?` is the drop policy, in the same place as the first.** `docs.iter().position(…)?`
sits inside the same `filter_map` closure as `entry.href()?`, so both mean "give up on this
entry" and both feed the same filter. An entry whose href names a document outside the spine —
a book referencing a file it forgot to declare, or one that lists it as an unlinked resource —
disappears rather than becoming a row whose click would panic on `docs[chapter()]`.

**The fragment is decoded, the path is decoded, and they are decoded by different code.**
`decode()` handles the path because rbook hands you an `Href`; the fragment gets
`percent_encoding` directly because `Href::new` is `pub(crate)` and there is no way to wrap the
fragment slice back into an `Href` from outside the crate. Both must be decoded for the same
reason: `fragment-scroll.js` ends up calling `document.getElementById(fragment)`, and the DOM
id is the *decoded* text. This is exactly what `epub::resolve_internal_link` already does for
in-book links, one decode call each.

**`&[String]` rather than `&Vec<String>`.** The function never needs to grow or shrink the
spine, so taking a slice lets it accept a `Vec`, an array, or — as the second test does — a
**sub-slice**. That is not a style rule here; it is what makes the drop branch testable
without a malformed fixture.

### Scope note

The duplication is real and deliberate: `epub::resolve_internal_link` also splits a fragment,
percent-decodes, and looks the path up in `docs`. It is not shared yet because the two differ at
their edges — `resolve_internal_link` starts from a `dioxus://` URL and must strip the prefix,
this starts from an archive-relative href — and folding them together before Step 6 shows what
the ToC jump actually needs would be guessing. **Step 7 owns that call**, and by then there will
be three call sites to judge it from.

Still not wired to anything: `toc_entries` remains uncalled outside its tests, and the two
`dead_code` warnings stand until Step 4.

Step 1's test also changes here, unavoidably: its `entries[0].href` assertion named a field
that no longer exists. The href is not lost — it is asserted in more useful form by
`entries[0].spine_index == 1` plus `entries[0].fragment == Some("pgepubid00000")`, which is
the same fact stated in the coordinates the reader uses. Step 1's test keeps its labels and
depths and goes on doing its own job, which is order and nesting.

Both tests now open the book through `epub::open_with_spine` rather than `Epub::open`, since
they need the spine alongside the `Epub` — the same call the reader makes at
`OpenBook`.

> **Status:** done — committed in `cdd91bf`, **111 tests green** (109 → 111).
>
> Written with the implementation rather than watched red first, so all three assertions were
> verified by mutating the test: `entries[1].spine_index` 1 → 2 (red at `toc.rs:80`), the drop
> test's `entries.len()` 6 → 18 (red at `toc.rs:101`), and the sibling loop's `spine_index`
> 2 → 3 (red at `toc.rs:83`). Each mutation went into the test only, and the file was restored
> from a byte-identical copy afterwards.
>
> One correction landed during the step: `cargo clippy --all-targets` flagged
> `needless_range_loop` on `for i in 2..=5`. Rewritten as `for entry in &entries[2..=5]`, which
> also lets the failure message name the entry rather than its index; the snippet above is the
> corrected form. Clippy is back to the two expected `dead_code` warnings.

---

## Step 3 — Name the current chapter

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Steps 1 and 2 built the map in the well-posed direction: an entry names exactly one spine
index. This step runs it **backwards**, and backwards it is not a function — spine 2 has four
entries, spine 0 has none. So this step is not a lookup, it is a **rule**, and the test's job
is to pin the rule rather than to prove an algorithm.

The rule, as the phase doc records it:

1. **The first entry naming this document**, if any. Of the four entries on spine 2, the
   reader is told "I. A SCANDAL IN BOHEMIA" — the parent — not "III.", the sub-entry that
   happens to be last.
2. Otherwise **the nearest preceding entry**. A document the ToC skips is not nameless; it is
   still *inside* the chapter the last entry opened.
3. Otherwise **nothing**. Before the first entry there is no chapter to be inside of — the
   fixture's cover, spine 0, is exactly this.

### The check — `cargo test`

Two tests: one over the real fixture for rule 1 and rule 3, one over a hand-built `Vec` for
rule 2, which the fixture cannot exercise (its ToC skips only spine 0, and spine 0 has no
preceding entry).

```rust
#[test]
fn the_current_chapter_is_the_first_entry_naming_its_document() {
    let (epub, docs) =
        epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

    let entries = toc_entries(&epub, &docs);
    let label = |spine_index| {
        entry_index_for_spine(&entries, spine_index).map(|index| entries[index].label.as_str())
    };

    assert_eq!(label(1), Some("The Adventures of Sherlock Holmes"));
    assert_eq!(label(2), Some("I. A SCANDAL IN BOHEMIA"));
    assert_eq!(label(14), Some("THE FULL PROJECT GUTENBERG™ LICENSE"));

    assert_eq!(label(0), None);
}

#[test]
fn a_document_the_toc_skips_keeps_the_preceding_entry() {
    let entry = |label: &str, spine_index| TocEntry {
        label: label.to_string(),
        depth: 0,
        spine_index,
        fragment: None,
    };
    let entries = vec![entry("One", 1), entry("Two", 4)];
    let label = |spine_index| {
        entry_index_for_spine(&entries, spine_index).map(|index| entries[index].label.as_str())
    };

    assert_eq!(label(2), Some("One"));
    assert_eq!(label(3), Some("One"));
    assert_eq!(label(4), Some("Two"));
    assert_eq!(label(5), Some("Two"));

    assert_eq!(label(0), None);
}
```

`label(1)` and `label(2)` are the two halves of rule 1 and they are not the same assertion:
spine 1 carries **two** entries and spine 2 carries **four**, so both pin "first", but only
`label(2)` distinguishes first-of-four from last-of-four. `label(0)` is rule 3 stated against
the fixture's real cover — the spine item the ToC genuinely never names.

The second test's synthetic list is deliberate. A fixture-driven test for rule 2 would have to
mutate the real entries to manufacture a gap, and a test that edits its own input to reach the
branch reads worse than two rows written by hand. `label(4)` and `label(5)` are in there so the
fallback cannot be mistaken for "always the preceding entry": 4 is an exact hit, 5 falls back to
the same row, and only the exact-first branch makes them agree for the right reason.

### The code

```rust
pub(crate) fn entry_index_for_spine(entries: &[TocEntry], spine_index: usize) -> Option<usize> {
    entries
        .iter()
        .position(|entry| entry.spine_index == spine_index)
        .or_else(|| {
            entries
                .iter()
                .rposition(|entry| entry.spine_index < spine_index)
        })
}
```

### Why it works

**`position` / `rposition` is the rule spelled out in two words.** `position` takes the first
match in iteration order — which is ToC order, which is reading order — so of the four entries
on spine 2 it returns the parent. `rposition` walks from the back and takes the first match
*there*, i.e. the **last** entry that precedes the target. The two halves want opposite ends of
the list, and the standard library has a name for each; writing it any other way
(`filter(...).last()`, `take_while(...).last()`, a manual fold) restates the same thing with
more moving parts. `rposition` is available because `slice::Iter` is a `DoubleEndedIterator`
*and* an `ExactSizeIterator` — it can be driven from either end and still know which index it
landed on, both of which a `Vec`'s contiguous memory makes free.

**`or_else`, not `or`.** `or(x)` evaluates `x` eagerly; `or_else(|| x)` only runs the closure
when the first arm was `None`. Here that means the second scan does not happen at all on the
common path — every chapter the ToC actually names, which is 14 of the fixture's 15. Reach for
`or_else` whenever the fallback costs more than a constant.

**Returning `Option<usize>` rather than `Option<&TocEntry>` — the index is the more
informative answer.** Both avoid cloning, so neither allocates; the difference is what the
caller can still do afterwards. Index → entry is `entries[index]`. Entry → index is *not*
recoverable without a second scan or pointer identity, because two rows can compare equal:
`TocEntry` derives `PartialEq`, and a book with two "Contents" rows at the same depth on the
same document produces genuine duplicates. Step 5 renders `entries.iter().enumerate()` and
wants to mark the current row, which is `Some(index) == current` — a comparison the reference
form cannot make honestly.

The second half is a Dioxus constraint. `usize` is `Copy` and `'static`, so the answer can sit
in a `use_memo` or ride into a child component as a prop. `Option<&TocEntry>` can do neither:
once the entries live in a signal, `read()` hands back a guard, and a reference borrowed from
that guard cannot outlive it. The borrow does not propagate into the render tree the way it
looks like it should.

**No `usize` arithmetic anywhere.** The obvious alternative implementation — "binary-search the
entries for `spine_index`, then walk back" — is faster on paper and wrong in practice: it
assumes `entries` is sorted by `spine_index`, which is true of every well-formed ToC and is not
guaranteed by anything. A ToC may point backwards (an appendix listed before the chapter that
references it, a "return to contents" entry). Two linear scans over 18 rows cost nothing and
have no precondition to violate.

### Scope note

`entry_index_for_spine` still has no caller — Step 4 puts the label in the nav bar, and
the `dead_code` warnings are now three rather than two. The rule this step chose is
**document-level only**: reading page 40 of chapter I, deep inside sub-entry "III.", still
answers "I. A SCANDAL IN BOHEMIA". Fragment-level precision needs the live DOM and is out of
scope for the whole phase, per the phase doc.

> **Status:** done — committed in `2249211`, **113 tests green** (111 → 113).
> `cargo clippy --all-targets` clean apart from the three expected `dead_code` warnings,
> `src/toc.rs` rustfmt-clean.
>
> Unlike Steps 1 and 2, this one was **watched red**. First red was a compile error —
> `cannot find function 'entry_for_spine' in this scope`, twice. Then the exact-match half was
> implemented alone, which turned that into a real assertion failure in the fallback test:
> `assertion left == right failed / left: None / right: Some("One")` at `src/toc.rs:132`.
> Adding the `or_else` arm turned it green. Both branches of the rule were therefore observed
> failing before either was implemented.
>
> **The return type changed after a `simplify` pass.** The step was first written as
> `entry_for_spine -> Option<&TocEntry>`, with the signature flagged in the handoff as the
> step's open question. The cleanup review argued the index is strictly the more informative
> answer and that `Option<&TocEntry>` will not survive contact with a signal read guard in
> Step 4; `find`/`rfind` became `position`/`rposition`, which mirror them exactly, so the rule
> and its reasoning carried over unchanged. The tests kept every assertion — only the `label`
> closure changed, from `.map(|entry| entry.label…)` to `.map(|index| entries[index].label…)`.
> Reverting is a four-line edit if the reference form reads better to you.
>
> At commit time both assertions were re-verified by mutation, because the red above was
> watched in the pre-`simplify` `find`/`rfind` shape and the shipped code is
> `position`/`rposition`. `label(2)`'s expectation swapped to `Some("III.")` — the
> first-vs-last claim — went red at `src/toc.rs:120`; `label(3)`'s swapped to `None` — the
> fallback — went red at `src/toc.rs:140`. Both mutations touched the test only, and the file
> was restored from a byte-identical copy each time.

---

## Step 4 — Show the name in the nav bar

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Three steps of pure functions over a `Vec` and nothing on screen has changed. This step is
the **first wire**: `toc_entries` and `entry_index_for_spine` acquire their first real
callers, and the chapter nav row stops saying "Chapter 3 of 15" and starts saying
"I. A SCANDAL IN BOHEMIA".

Two pieces, and only one of them is testable:

- **`chapter_label`** — a pure `(&[TocEntry], chapter, chapter_count) -> String`. Entirely a
  `#[test]`.
- **the flatten, once per book** — where in the component tree `toc_entries` actually gets
  called, and what keeps its result alive. That one is a `dx serve` eyeball, because it is a
  statement about Dioxus's hook lifecycle rather than about a value.

### The check — `cargo test`

In `src/ui/reader.rs`'s existing `mod test`, next to `the_page_label_waits_for_a_real_count` —
this is the same kind of function, doing the same job, for the row above.

```rust
#[test]
fn the_chapter_label_prefers_the_toc_entry_over_the_ordinal() {
    let (epub, docs) =
        epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");
    let entries = toc::toc_entries(&epub, &docs);

    assert_eq!(
        chapter_label(&entries, 2, docs.len()),
        "I. A SCANDAL IN BOHEMIA"
    );

    assert_eq!(chapter_label(&entries, 0, docs.len()), "Chapter 1 of 15");

    assert_eq!(chapter_label(&[], 2, 15), "Chapter 3 of 15");
}
```

Run it with `cargo test the_chapter_label`.

Three assertions, and each reaches a place the other two cannot:

- **spine 2 → the parent entry.** The chapter carrying four entries, so this is Step 3's rule
  arriving intact at the UI rather than being re-decided here.
- **spine 0 → "Chapter 1 of 15".** The fixture's cover, the one spine item the ToC genuinely
  never names. The fallback is not a defensive branch for malformed books; the *bundled* book
  reaches it on the page it opens to. This is also the assertion that would catch the tempting
  wrong implementation — `if entries.is_empty() { ordinal } else { entries[index].label }`
  panics here and passes the line below.
- **`&[]` → "Chapter 3 of 15".** A book with no ToC at all, which the phase doc calls a normal
  book. `&[]` is not a mock — it is exactly what `toc_entries` returns when `contents()` is
  `None`, so the empty slice tests the real path, and it pins the `chapter + 1` offset at a
  chapter where an off-by-one would show.

Deliberately *not* asserted here: that spine 2 resolves to the *first* of its four entries
rather than the last, and that spine 0 resolves to nothing at all. Those are claims about
`entry_index_for_spine`, and `src/toc.rs` already pins them. Restating them in a UI module
would mean fixing the same expectation twice the day the bundled fixture changes.

### The eyeball — `dx serve`

Open the Sherlock fixture. The label should read **"Chapter 1 of 15"** on the cover, and
turning one chapter forward should switch it to **"The Adventures of Sherlock Holmes"**, then
**"I. A SCANDAL IN BOHEMIA"**. Page-turning *within* a chapter must not change it — the label
is a function of the spine index, and paging does not move the spine index.

**Where it ended up differs from where this step proposed to put it.** The step said "the nav
bar", meaning the chapter `NavRow`'s label slot. In the built version the label is a *second
line in the centred header*, under the book title, and the chapter `NavRow` keeps it too — so
it renders in both places from the same `chapter_label` string:

```
header:   The Adventures of Sherlock Holmes
          I. A SCANDAL IN BOHEMIA

  [Prev]  I. A SCANDAL IN BOHEMIA  [Next]
  [Prev]        Page 4 of 22       [Next]
```

The header is the better home — "what am I reading" belongs with the title, not wedged
between two buttons — and the absolutely-positioned title `p` had to become a `div` wrapping
two `p`s to hold both lines, which is what the surrounding padding tweaks are for. The
duplicate in the `NavRow` was kept knowingly for now; Step 5 gives the chapter row a contents
button and is the natural moment to decide whether that row still wants a label at all.

### The code

```rust
use crate::toc::{self, TocEntry};

fn chapter_label(entries: &[TocEntry], chapter: usize, chapter_count: usize) -> String {
    match toc::entry_index_for_spine(entries, chapter) {
        Some(index) => entries[index].label.clone(),
        None => format!("Chapter {} of {}", chapter + 1, chapter_count),
    }
}
```

and in `Reader`, two lines:

```rust
let docs = book.docs;
let entries = use_hook(|| Rc::new(toc::toc_entries(&book.epub, &docs)));
…
let chapter_label = chapter_label(&entries, chapter(), state.chapter_count);
```

### Why it works

**`use_hook`, not `use_memo`, and not a plain call.** A component body in Dioxus re-runs on
every render — every page turn, every theme change. Calling `toc_entries` there would reparse
and reallocate the whole ToC each time. `use_hook` runs its closure **once per mounted
component** and returns a clone of the stored value on every subsequent render; the ToC is
derived from the `Epub`, which cannot change while the book is open, so "once per mount" is
exactly right. `use_memo` is the tool when the value depends on signals and must recompute
when they change — this one depends on nothing reactive, so a memo would add a subscription
and a `Signal` allocation to buy nothing. The precedent is one line down: `start` (the stored
reading position) is a `use_hook` for the same reason.

**Once per mount is once per book, because of the `key`.** `main.rs` mounts the reader as
`Reader { key: "{book.id}", book }`. Without that key, opening a second book would hand the
*same* component instance a new `book` prop, `use_hook` would not re-run, and you would read
book two under book one's table of contents. The key makes a different id a different
component, so the hook state is torn down and rebuilt. This is not a new guarantee this step
introduces — `start` already depends on it — but it is the guarantee that makes `use_hook`
legal here, and it lives in a different file from the code that relies on it.

**`Rc::new` around the `Vec`, because `use_hook` clones.** `use_hook<T: Clone>` hands back
`T::clone()` on every render. Cloning a `Vec<TocEntry>` is 18 `String` allocations per render
— per page turn — for a value that is never mutated. `Rc` makes that clone a refcount bump.
It is the same reasoning, and the same shape, as `OpenBook::docs: Rc<Vec<String>>` two files
over. `&entries` then derefs through the `Rc` to the `Vec` and unsizes to `&[TocEntry]`
without anything written down.

**`&[TocEntry]` in the signature, not `&Rc<Vec<TocEntry>>` or `&Vec<TocEntry>`.** The slice is
the weakest thing the function can ask for, which is why the test can pass it a literal `&[]`
and never construct an `Rc` at all. A function that takes `&Vec` forces every caller to own a
`Vec`; a function that takes `&[T]` accepts vectors, arrays, slices and empty literals alike.
Same rule that let Step 2 test its drop branch with `&docs[..3]`.

**`.clone()` on the label is the honest cost, and it is small.** The alternative is returning
`&str` borrowed from `entries`, which would make `chapter_label` generic over a lifetime and
push the borrow into the rsx block — where the string is interpolated into a `String` prop
anyway. One short allocation per render, on a path that already formats `page_label` from
scratch each time. Cloning where the data ends up owned regardless is not waste; it is the
absence of a borrow that would have to be discharged one line later.

**The fallback is not error handling.** `None` from `entry_index_for_spine` means "no entry
names this document and none precedes it", which the fixture produces on page one. So both
arms of the `match` are normal operation, and the ordinal is a *label*, not a diagnostic.
That is also why the arm keeps `chapter_count` — "Chapter 1 of 15" tells you where you are in
a way "Chapter 1" does not, and it is the only place `chapter_count` is still used.

### Scope note

The label is document-level, per Step 3's rule: deep inside sub-entry "III." the bar still
reads "I. A SCANDAL IN BOHEMIA". Nothing here renders a list — Step 5 puts the entries in the
popover, and it is the reason the entries are held in a shareable `Rc` rather than being
computed inside `chapter_label` and dropped. `TocEntry::depth` and `TocEntry::fragment` still
have no reader; Steps 5 and 6 respectively.

`chapter_label` lives in `src/ui/reader.rs` next to `page_label` rather than in `src/toc.rs`,
because the "…of 15" half is a fact about the nav bar and not about the table of contents;
`toc.rs` stays a pure model with no opinion about how it is displayed. If Step 5's panel wants
the same string, that is the moment to reconsider.

One incidental: the three `dead_code` warnings carried since Step 1 are gone. `toc_entries`
and `entry_index_for_spine` have real callers now, which is what those warnings were waiting
for — and why they were never silenced with an `#[allow]`.

Two calls the `simplify` pass looked at and left alone, both worth knowing were *decided*
rather than defaulted:

- **The entries are derived in `Reader`, not carried on `OpenBook` beside `docs`.** `docs`
  earns its seat there because `open_with_spine` *returns* it — it is the boundary's own
  output. `toc_entries` would be a call `library.rs` makes purely on the reader's behalf, and
  the library screen never reads a ToC. Since the `key` makes `use_hook` fire exactly once per
  book either way, the `OpenBook` version buys no timing and costs a wider struct shared by
  three modules. Revisit if the library screen ever wants the ToC, or if the `key` goes away.
- **`chapter_label` stays in `ui/reader.rs`.** It owns UI copy — the fallback *wording*, and
  the policy of falling back to an ordinal at all — and its `chapter_count` argument is nav
  state, not ToC data. A function in `toc.rs` taking a spine count it has no other use for is
  the tell that it is in the wrong module.

The one deferral: `entry_index_for_spine(…).map(|i| entries[i].label)` is now written three
times, twice in `toc.rs`'s tests and once here. A `toc::label_for_spine(&[TocEntry], usize)
-> Option<&str>` would collapse all three and leave `chapter_label` as nothing but its
fallback. Two of the three sites are test-local, so it goes on **Step 7**'s list next to the
`resolve_internal_link` overlap Step 2 parked there.

> **Status:** done — committed in `24ad8bd`, **114 tests green** (113 → 114).
> `cargo clippy --all-targets` is clean for the first time this phase, and `src/ui/reader.rs`
> is rustfmt-clean.
>
> Watched red twice, both on the shape that shipped. First `cannot find function
> 'chapter_label' in this scope` ×4. Then the ordinal arm was implemented **alone**, which
> turned that into a real assertion failure — `left: "Chapter 3 of 15" / right: "I. A SCANDAL
> IN BOHEMIA"` — so the toc arm was observed failing before it existed. Unlike Step 3 the
> `simplify` pass did not reshape the implementation afterwards, so that red stands as
> written.
>
> The two fallback assertions were green from the moment the ordinal-only stub existed and so
> were never observed failing; both were verified by mutation at commit time instead.
> `chapter_label(&entries, 0, …)`'s expectation swapped to `"The Adventures of Sherlock
> Holmes"` went red at `src/ui/reader.rs:357`, and `chapter_label(&[], 2, 15)`'s swapped to
> `"Chapter 2 of 15"` went red at `src/ui/reader.rs:359`. Both mutations touched the test
> only, and the file was restored from a byte-identical copy each time.
>
> The `simplify` pass ran four ways over the diff. Reuse and altitude both returned "keep it
> as is" — the two forks recorded above. Simplification found one real redundancy: an
> assertion on spine 14 that reached the same `match` arm as the spine-2 one and duplicated a
> claim `toc.rs` already pins. It was dropped, taking the test from four assertions to three.
>
> The visual half is the learner's, and the layout above is theirs: the step proposed the
> chapter `NavRow`'s label slot and the header is where it actually landed. The doubled label
> was reviewed and kept deliberately.

---

## Step 5 — Render the contents panel

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Step 4 showed you *one* label — the chapter you are in. This step shows you **all eighteen**,
nested the way the book nests them, with your own row marked. It is the first step in the
phase whose deliverable is pixels rather than a value, and the first to read
`TocEntry::depth`, which has sat unused since Step 1.

Nothing here jumps. A row is a `<button>` that hovers, focuses, and does nothing when
clicked; Step 6 adds the `onclick` and nothing else. That split is deliberate — rendering the
list and navigating from it fail in completely different ways, and debugging them together is
how you end up unsure which half is broken.

### The check — `dx serve`

**There is no `cargo test` for this step, and no red to watch.** The step's whole claim is
about layout in a webview: that eighteen rows appear, that three of them sit indented under
their parent, that one is bold. None of that is reachable from a unit test — Dioxus renders
into a real webview here, and asserting on an rsx tree would test the macro rather than the
panel.

Two things stand in for a test, and both are real:

- **`cargo clippy --all-targets`**, which is the phase doc's stated gate for this step.
- **The `css_module` macro**, which is stronger than a test would have been. Every class in
  `src/ui/toc.rs` is written as `Styles::contents_popover__entry`, not as a string, and the
  constant is generated from `src/ui/toc.css` at compile time. A class that the stylesheet
  does not define **fails the build**:

  ```
  error[E0599]: no associated function or constant named `contents_popover__bogus`
                found for struct `ui::toc::Styles` in the current scope
  ```

  That was verified by mutation rather than assumed — renaming one class in the `.rs` file
  produced exactly that error, and the file was restored afterwards. This is the class of bug
  the repo's "two files agree on a string" tests exist to catch (`ook-set-theme`,
  `__ookBlobUrl`), and here the compiler catches it for free.

So the gate is your eyes. Open the Sherlock fixture and check, in this order:

1. A **list icon** appears in the top-right of the header, left of the settings gear, the same
   round 40px `icon-button` shape.
2. Clicking it opens a panel with **18 rows**, top row "The Adventures of Sherlock Holmes",
   bottom row "THE FULL PROJECT GUTENBERG™ LICENSE".
3. **"I.", "II." and "III." sit one step to the right**, directly under "I. A SCANDAL IN
   BOHEMIA". That is `depth` reaching the screen — and it is the assertion the Step 1 test
   made about *order* now made visible as *shape*.
4. On the cover (spine 0) **no row is bold**. Turn one chapter: the top row goes bold. Turn
   again: "I. A SCANDAL IN BOHEMIA" goes bold and the three sub-entries under it do **not**.
   That is Step 3's first-match rule, seen rather than asserted.
5. **Arrow keys with the panel open do not turn the page.** This is the one behaviour here
   that is a bug if absent rather than merely ugly — see the `stop_propagation` note below.
6. Escape and a click outside both close the panel.

### The code

A new component file, `src/ui/toc.rs`, plus its stylesheet `src/ui/toc.css`, plus
`pub mod toc;` in `src/ui/mod.rs`.

```rust
#[css_module("/src/ui/toc.css")]
struct Styles;

#[component]
pub(crate) fn ContentsPopover(entries: Rc<Vec<TocEntry>>, chapter: usize) -> Element {
    if entries.is_empty() {
        return rsx! {};
    }

    let current = toc::entry_index_for_spine(&entries, chapter);

    rsx! {
        PopoverRoot {
            PopoverTrigger { svg { /* tabler list icon */ } }
            PopoverContent {
                class: Styles::contents_popover__content.to_string(),
                align: ContentAlign::End,
                nav {
                    class: "{Styles::contents_popover__list}",
                    onkeydown: move |e| e.stop_propagation(),
                    for (index , entry) in entries.iter().enumerate() {
                        button {
                            class: "{Styles::contents_popover__entry}",
                            aria_current: if Some(index) == current { "page" },
                            style: "--toc-depth: {entry.depth};",
                            "{entry.label}"
                        }
                    }
                }
            }
        }
    }
}
```

and in `Reader`'s header, into the flex row that already holds the gear:

```rust
ContentsPopover {
    entries: entries.clone(),
    chapter: chapter(),
}
SettingsPopover {}
```

### Why it works

**`src/ui/toc.rs` beside `src/toc.rs`, and the name collision is not one.** The repo already
runs this exact split twice: `src/settings/` holds the model and `src/ui/settings.rs` holds
`SettingsPopover`; `src/library/` and `src/ui/library.rs` likewise. A module's own name is not
in its own scope — only the `self` keyword refers to it — so inside `src/ui/toc.rs` the line
`use crate::toc::{self, TocEntry}` binds `toc` to the *model*, and `toc::entry_index_for_spine`
means the model's function with no ambiguity to resolve. In `reader.rs` only `ContentsPopover`
is imported from `ui::toc`, so bare `toc::` there still means `crate::toc`.

**The component computes `current` itself rather than taking it as a prop.** `current` is
*derivable* from `entries` and `chapter`, both already props. Passing it as a third prop would
create a second source of truth that a caller could hand over stale next to fresh entries, and
nothing would catch it. That `chapter_label` in `reader.rs` resolves the same index is not
duplication of the *rule* — the rule lives in exactly one place, `toc::entry_index_for_spine`,
and both call sites follow it if it changes.

**`Rc<Vec<TocEntry>>` as a prop, and why `TocEntry` grew `Eq` this step.** Dioxus decides
whether to re-render a child by comparing its props with `PartialEq`, on **every** render of
the parent — every page turn, every scroll message from the frame. `Rc`'s `PartialEq` has a
`ptr_eq` fast path, but the standard library only enables it when `T: Eq`, because a
`PartialEq` impl is allowed to be deliberately irreflexive and a pointer shortcut would lie
about it. `Vec<TocEntry>: Eq` needs `TocEntry: Eq` — so without it, every `Reader` render
deep-compared 18 entries, `String` by `String`, to answer a question one pointer comparison
settles. Both `Rc`s are clones of the same allocation, so the fast path is always the right
answer. One word on the derive in `src/toc.rs`; the cost it removes is micro on this fixture
and linear in ToC size on a real technical book.

**`PopoverRoot` with no `open` signal.** `SettingsPopover` threads a
`use_signal(|| false)` through `open` / `on_open_change`, and this component pointedly does
not. `dioxus_primitives`' `PopoverRoot` calls `use_controlled(props.open, …)`, which keeps its
**own** internal signal and uses it whenever the `open` prop is absent — and `open` is
auto-optional because of its `ReadSignal<Option<bool>>` shape. The trigger, the escape
listener and the outside-dismiss all write through the same context either way. So the signal
in this component would have been state that is read by nothing. Step 6 will most likely bring
it back, because closing the panel after a jump is a thing only the caller can decide — and
then it will be earning its place.

**The empty guard is safe because there are no hooks left.** `if entries.is_empty() { return
rsx! {}; }` before any hook call would be a rules-of-hooks bug in the general case: Dioxus
identifies hook state positionally, so a render that skips a hook desynchronises every later
one. With the `open` signal gone the component has **zero** hooks and the early return is
unconditionally safe. It stayed inside the component rather than moving to `Reader`'s rsx
because "a book with no ToC has no contents button" is a fact about the contents feature —
the phase doc's own framing — and `Reader` should not have to know the shape of `entries`.

**`aria_current` instead of a modifier class.** "You are here" gets stated **once**, in the
accessibility layer, and the stylesheet selects on it with
`.contents-popover__entry[aria-current="page"]`. A `--current` modifier class would state the
same fact twice and let the two drift. The token is `"page"` rather than `"true"` because
inside a `<nav>` of chapter links that is the ARIA value meaning "the current page in a set",
and a screen reader announces it as such. `aria_current` is a real typed attribute in
`dioxus-html`, so the name is compiler-checked; and `if … { … }` with no `else` omits the
attribute entirely, which is what the cover's `None` needs.

**`--toc-depth` carries the datum, the stylesheet carries the look.** The row's padding is one
rule in `toc.css`:

```css
padding: 0.35rem 0.75rem 0.35rem calc(0.75rem + var(--toc-depth, 0) * 1rem);
```

and the inline style is only `--toc-depth: {entry.depth};`. Putting the whole `padding`
inline would have duplicated `0.75rem` across a Rust string and a CSS file — and the `0.75rem`
inside the `calc` is *the same* `0.75rem` as the base, which is only visible when they sit on
one line. `var(--toc-depth, 0)` defaults to 0 so the rule is still valid for anything that
renders the class without the property.

**`stop_propagation` on the list is load-bearing.** `reader.rs` puts an `onkeydown` on
`.reader-root` that turns pages on the arrow keys, and the popover content is still a
descendant of it in the DOM however it is positioned. Without this line, arrowing through an
open contents panel would silently turn pages behind it. It does **not** break Escape-to-close:
`dioxus_primitives` handles Escape through a document-level listener installed by
`use_global_escape_listener`, entirely outside Dioxus's synthetic event bubbling. `settings.rs`
carries the identical line for the identical reason — see the scope note.

**No `key` on the rows.** `key` buys Dioxus identity across reorders; an *index* key is
exactly the positional diff it already does unkeyed, so it would have been 18 `String`
allocations per render buying nothing. Contrast `library.rs`'s `key: "{book.id}"` and
`picker.rs`'s `key: "{slug}"`, which are real identities on lists that change. This list is
fixed for the life of the mounted book, and the labels are not unique anyway — the fixture has
four rows whose text repeats across chapters.

### Scope note

**No jumping.** The rows are buttons with no `onclick`, so the panel advertises with
`cursor: pointer` and a hover background something it cannot yet do. Step 6 converts a
`TocEntry` into `epub::LinkTarget` and hands it to the existing `nav::ReaderState::follow_link`.
The shape here is built for that to be a pure addition: `index` already comes off
`.enumerate()` as a `Copy` scalar, so the click closure will capture nothing borrowed from
`entries`. The likely signature is a third prop, `on_pick: EventHandler<usize>` — matching
`picker.rs` — because `ReaderState` derives `Clone, Copy` but **not** `PartialEq` and so
cannot be a component prop, while `EventHandler` props are exempt from the props comparison.

**The chapter `NavRow` still duplicates the header's chapter label.** Step 4 parked that
decision here on the theory that this step would give the chapter row a contents button. It
did not — the button went in the header next to the gear, where the popover machinery and the
`icon-button` shape already live and where "what am I reading" was put in Step 4. So the
duplicate label is still there and is still the learner's call.

**Deferred to Step 7, all of it found by the `simplify` pass and left deliberately:**

- **The tabler SVG preamble is now on its third copy.** Ten identical attributes plus a
  `d: "M0 0h24v24H0z"` path that renders nothing (`stroke: none` *and* `fill: none`) plus an
  `icon-tabler` class list that matches no stylesheet in the repo, repeated across
  `reader.rs`, `settings.rs` and now `toc.rs` — about twelve lines each. This diff is what
  makes it the third occurrence and therefore worth an `Icon { children }` wrapper. Not done
  here because collapsing it edits two files outside the step.
- **The `stop_propagation` line belongs to the shared popover, not to each caller.** Two
  identical occurrences is where that becomes true. It was left alone because
  `PopoverContent` forwards `attributes` verbatim and an event handler is not an `Attribute` —
  hoisting it needs either `merge_attributes` or an extra wrapper `div` between
  `.dx-popover-content`'s flex column and its children, which would disturb the `gap` the
  settings panel relies on. A shared-component change with a layout blast radius is Step 7's
  business.
- **The panel does not scroll the current row into view.** `.contents-popover__list` is
  `max-height: 80vh; overflow-y: auto`, so on the 18-entry fixture nothing scrolls and this is
  invisible. On a book with a long ToC, opening the panel shows the top while `aria-current`
  marks a row off-screen.
- **The rows are loose `<button>`s inside a `<nav>` with no list semantics**, so assistive
  tech gets no "list, 18 items" orientation. `library.rs` uses `ul`/`li` for its book grid and
  is the precedent if this is worth fixing; it was left flat here because it costs two
  elements and two more CSS rules for a decision better made once, alongside the scroll-into-view
  one.

Also parked here since Step 2: the overlap between `toc_entries`' href resolution and
`epub::resolve_internal_link`, and since Step 4: a possible `toc::label_for_spine` collapsing
the three `entry_index_for_spine(…).map(|i| entries[i].label)` sites.

> **Status:** done — committed in `20535be`, **114 tests green** (unchanged). `cargo clippy
> --all-targets` clean, and the touched files are rustfmt-clean.
>
> **This is the first step in the phase that added no test, and the count staying at 114 is
> the honest record of that.** The step's claim is about layout in a webview and there was no
> red to watch. The gate was a clean build plus the learner's own `dx serve` pass — evidenced
> here by the stylesheet arriving tuned (`min-width` 16rem → 24rem, `max-height` 60vh → 80vh,
> list padding 0.25rem → 1rem), which are not values anyone picks without having looked at the
> panel. The agent did not and cannot eyeball the webview.
>
> What the compiler covers instead of a test: every class name is a `css_module` constant
> generated from `toc.css`, so a class the stylesheet does not define fails the build. That
> was verified by mutation rather than assumed — renaming one class in `src/ui/toc.rs`
> produced `error[E0599]: no associated function or constant named
> 'contents_popover__bogus'`, and the file was restored afterwards.
>
> **One gap found at commit time and deliberately left, because closing it needs an
> implementation edit:** `--toc-depth` is spelled in two files with no compiler between them —
> `src/ui/toc.rs` writes it inline, `src/ui/toc.css` reads it as `var(--toc-depth, 0)`. Rename
> it on one side and the indent silently goes flat; nothing errors, the nesting just
> disappears. This is exactly the hazard the repo's `ook-set-theme` / `__ookBlobUrl` tripwire
> tests exist for, and the class names next to it are already immune by construction. The test
> cannot be written honestly against the current shape: the Rust half of the pair is a literal
> inside an `rsx!` string, so nothing can read it back at runtime, and a test asserting only
> the CSS half pins one direction of a two-way rename. Hoisting the name to a
> `const DEPTH_VAR: &str = "--toc-depth";` used as `style: "{DEPTH_VAR}: {entry.depth};"` would
> make one spelling authoritative and let a one-line `assert!(TOC_CSS.contains(DEPTH_VAR))`
> pin the other — a change to the implementation, so it is the learner's to make. Carried to
> **Step 7** with the other deferrals.
>
> The `simplify` pass ran four ways over the diff before the commit. It applied four findings:
> the `Eq` derive above (the only one with a runtime cost behind it), splitting the row padding
> into `--toc-depth` plus a stylesheet rule, dropping an index `key` that restated Dioxus's
> default, and moving the `aria-current` token from `"true"` to `"page"`. It deliberately left
> four for Step 7 — the tabler SVG preamble now on its third copy, hoisting the
> `stop_propagation` line into the shared `PopoverContent`, scroll-into-view for the current
> row, and `ul`/`li` list semantics — each because the fix reaches outside this step's files.
>
> Two forks worth knowing were decided rather than defaulted, and both are cheap to reverse.
> The `open` signal was **removed**, which makes this popover differ from `SettingsPopover`'s
> identical-but-unused triple; Step 6 will likely restore it, because closing the panel after a
> jump is a decision only the caller can make. And the trigger went in the **header** beside
> the settings gear rather than in the chapter `NavRow`, which is where Step 4 predicted it —
> so the duplicated chapter label in that row survives another step, still unresolved.

---

## Step 6 — Jump to an entry

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

The rows have been buttons since Step 5, hovering and focusing and doing nothing. This step
gives them their `onclick`, and the phase's promise — *pick a chapter off a list and go there*
— finally closes.

**The crux is that there is almost nothing to build.** Phase 3 already shipped the whole jump
machinery for the in-book `<a href>` bridge: `epub::LinkTarget { spine_index, fragment }` names
a destination, and `nav::ReaderState::follow_link` goes there — setting `Phase::Loading` when
the document changes, resetting the page, and parking the fragment in `Pending` for
`chapter-loader.js` to scroll to. A ToC entry **is** an internal link wearing a label: it
already carries a `spine_index` (Step 2 resolved it) and an already-percent-decoded `fragment`
(Step 2 decoded it). So the step is a four-line `From` impl and a click handler. That the
phase's headline feature costs four lines is the payoff for Step 2 having done the resolution
eagerly instead of leaving hrefs lying around to be parsed at click time.

### The check — `cargo test`

```rust
#[test]
fn an_entry_becomes_the_link_target_its_href_would_have() {
    let (epub, docs) =
        epub::open_with_spine(Path::new(crate::TEST_BOOK)).expect("open fixture book");

    let entries = toc_entries(&epub, &docs);
    let target = epub::LinkTarget::from(&entries[2]);

    assert_eq!(target.spine_index, 2);
    assert_eq!(target.fragment.as_deref(), Some("pgepubid00002"));

    let href = format!("{}#pgepubid00002", epub::chapter_url(&docs[2]));
    assert_eq!(
        epub::resolve_internal_link(&docs, 0, &href),
        Some(target),
        "a picked entry and a followed link reach `follow_link` the same way"
    );

    let coverless = TocEntry {
        label: "Cover".to_string(),
        depth: 0,
        spine_index: 0,
        fragment: None,
    };
    assert_eq!(epub::LinkTarget::from(&coverless).fragment, None);
}
```

**The red was a compile error, and that is the honest report:**

```
error[E0308]: mismatched types
   --> src/toc.rs:153:45
    |
153 |         let target = epub::LinkTarget::from(&entries[2]);
    |                      ---------------------- ^^^^^^^^^^^ expected `LinkTarget`, found `&TocEntry`
```

`From` is a trait with a blanket `impl<T> From<T> for T`, so `LinkTarget::from` always resolves
to *something* — the identity impl — and the failure is a type mismatch rather than "no such
method". Worth knowing, because it means a `From` you forgot to write never fails as a missing
name.

**The middle assertion is the one that matters.** The first two only check that four fields
were copied. The third pins the claim the step actually rests on: the `LinkTarget` built from a
ToC entry is **byte-identical** to the one `resolve_internal_link` builds from the equivalent
`<a href>`. Two independent paths — the reader clicking a panel row, and the reader clicking a
link inside the book — converge on one value and one call to `follow_link`. If they ever
diverged (one decodes `%20`, the other doesn't; one strips the leading `/`, the other doesn't)
the panel would jump somewhere subtly different from the link, and only a test that builds both
sides can see it. `epub::chapter_url` is used rather than a hand-written URL so the test
percent-encodes the path exactly the way the frame does.

**The `coverless` case earns its lines**, though the `simplify` pass argued otherwise — see the
status note.

### The code

`src/toc.rs`:

```rust
use crate::epub::LinkTarget;

impl From<&TocEntry> for LinkTarget {
    fn from(entry: &TocEntry) -> Self {
        LinkTarget {
            spine_index: entry.spine_index,
            fragment: entry.fragment.clone(),
        }
    }
}
```

`src/ui/toc.rs` — a third prop, a controlled `open` signal, and the handler:

```rust
pub(crate) fn ContentsPopover(
    entries: Rc<Vec<TocEntry>>,
    chapter: usize,
    on_pick: EventHandler<LinkTarget>,
) -> Element {
    let mut open = use_signal(|| false);
    ...
        PopoverRoot {
            open: open(),
            on_open_change: move |v| open.set(v),
            ...
                        button {
                            ...
                            onclick: {
                                let target = LinkTarget::from(entry);
                                move |_| {
                                    open.set(false);
                                    on_pick.call(target.clone());
                                }
                            },
```

`src/ui/reader.rs`:

```rust
let on_pick = use_callback(move |target| state.follow_link(target));
...
    ContentsPopover {
        entries: entries.clone(),
        chapter: chapter(),
        on_pick,
    }
```

### Why it works

**`From<&TocEntry>` rather than `From<TocEntry>`.** The entries live in an
`Rc<Vec<TocEntry>>` shared between `Reader` and the panel; nothing can hand over an owned
`TocEntry` without cloning the label too, and the label is the one field a `LinkTarget` does not
want. Taking `&TocEntry` clones exactly the `Option<String>` that has to be owned and leaves the
`String` label where it is. Implementing `From` on the reference is the standard Rust move for
"cheap projection out of a borrowed struct", and it gives `.into()` at every call site for free.

**The impl lives in `src/toc.rs`, not `src/epub.rs`.** The orphan rules permit either — both
types are local — so layering decides. `epub.rs` is the lower layer: it knows about zip
resources, URLs and the spine, and nothing about tables of contents. `toc.rs` already imports
`epub`. Putting the impl in `epub.rs` would make the bottom of the stack name a type from the
top of it for no gain.

**`use_signal` moved *above* the `entries.is_empty()` early return, and that is load-bearing.**
Step 5's note explained that the early return was safe precisely because the component had zero
hooks. This step reintroduces one, and Dioxus identifies hook state **positionally** — a render
that returns before a hook call, followed by one that reaches it, desynchronises every hook
after it. In practice `entries` never changes for a mounted book, so the bug would not fire
today; the ordering is correct by construction instead of correct by luck.

**Why the panel has to be *controlled* to close itself.** `dioxus_primitives` keeps the open
state in a private `PopoverCtx` and exposes no imperative "close" handle. `use_controlled`
prefers the `open` prop when one is present, so lifting the state into our own signal is the
only way `onclick` can dismiss the panel. This restores the exact triple
(`use_signal` → `open:` → `on_open_change:`) that Step 5 deliberately deleted as unused — Step
5 predicted it would come back, and it did, now earning its place.

**The `onclick` value is a block, and the block is why the payload can be a `LinkTarget`.** An
event handler must be `'static`, but `entry` is borrowed out of `entries` for the duration of
the loop. Writing

```rust
onclick: {
    let target = LinkTarget::from(entry);
    move |_| { ... on_pick.call(target.clone()); }
},
```

resolves the borrow **at render time** into an owned `LinkTarget`, which the `move` closure then
owns outright. `rsx!` parses an attribute value as a full `syn::Expr`, so a block expression
introducing a binding is legal there — the one place in `rsx!` you can run a statement per
iteration of a `for`. The inner `.clone()` is needed because the closure is `FnMut` and may fire
more than once (`EventHandler` never consumes its captures).

**`use_callback` in `Reader` is not decoration — it is what keeps the panel memoized.** Dioxus
decides whether to re-render a child by comparing props, and the generated `PartialEq` for
`ContentsPopoverProps` compares **all three** fields, `on_pick` included:

```rust
fn eq(&self, other: &Self) -> bool {
    self.entries == other.entries && self.chapter == other.chapter && self.on_pick == other.on_pick
}
```

`Callback`'s `PartialEq` is pointer identity, and an inline closure in `rsx!` becomes a **fresh**
`Callback::new` on every render — so an inline `on_pick` would make that comparison false every
single time, and `memoize` compares *before* it repoints the handler:

```rust
fn memoize(&mut self, new: &Self) -> bool {
    let equal = self == new;
    self.on_pick.__point_to(&new.on_pick);
    ...
}
```

`Reader` re-renders on every page turn, every scroll message and every reflow, so an inline
closure would have thrown away exactly the fast path Step 5 bought by deriving `Eq` on
`TocEntry` — the `Rc` pointer comparison that lets 18 entries compare in one instruction.
`use_callback` allocates the `Callback` once in a hook and *replaces the boxed closure in place*
on later renders, returning the same handle, so pointer identity holds and the panel goes back
to re-rendering only when the chapter or the open state actually changes.

**Nothing new happens after `follow_link`.** It sets `Phase::Loading` only when the document
actually changes, so picking a sub-entry inside the chapter you are already reading does not
flash the spinner; it resets the page to 0 and parks `Pending::Fragment`, and the existing
`use_effect` on `chapter()`/`pending()` re-sends the URL and fragment to `chapter-loader.js`.
The fragment path is the same one the in-book links have used since Phase 3.

### Scope note

**The panel does not follow you.** Jumping updates `aria-current` on the next open, but the list
still does not scroll the current row into view — carried from Step 5 and still open, and now
slightly more visible since jumping is how you move a long way through a long ToC.

**Sub-entry precision is still out of scope**, as the phase doc says up front. Jumping *to*
"II." works — it is a real fragment and the loader scrolls to it. Knowing you are *in* "II."
rather than at the top of the chapter is the resolve-a-position-in-the-live-DOM problem the
milestone defers.

**Everything Step 5 parked is still parked**, and Step 7 is now the phase's last step and owns
all of it: the tabler SVG preamble on its third copy, the `stop_propagation` line that belongs
in the shared `PopoverContent`, `ul`/`li` list semantics, scroll-into-view, the `--toc-depth`
string spelled in two files with no compiler between them, the `toc_entries` /
`resolve_internal_link` overlap from Step 2, the possible `toc::label_for_spine` from Step 4,
and the duplicated chapter label in the `NavRow`. Step 6 adds one more: the controlled-open
triple is now verbatim in **two** files (`settings.rs` and `toc.rs`), which makes hoisting a
close handle into the repo's own `PopoverRoot` wrapper a real candidate rather than a
speculative one.

### What the `simplify` pass changed, and what it did not

The pass ran four ways over the diff and **rewrote the step's main interface**, which is worth
recording because the first draft was worse and the reason is instructive.

The draft had `on_pick: EventHandler<usize>` — the panel reported *which row* was clicked and
`Reader` looked the entry back up through a second `Rc` clone, `picks.get(index)`. Three of the
four reviewers independently flagged it: the row-order invariant spanned two files with nothing
in the type saying so, the `if let Some(...)` guard was an unreachable branch dressed as error
handling, and the component's real contract had become "take these entries *and* keep your own
copy". The draft justified `usize` as "matching `picker.rs`" — which is backwards:
`SlugPicker`'s `on_pick` is an `EventHandler<String>`, the picked *value*, not its position.
Handing over the resolved `LinkTarget` deleted the second `Rc`, the guard, and the cross-file
invariant together.

The fourth reviewer caught the memoization regression described above, and `use_callback` came
out of it.

**Two findings were deliberately skipped:**

- **"Delete the `coverless` fixture — the `From` impl is branchless, so it cannot fail on its
  own."** True as far as it goes, but it misses what the case pins. If the conversion ever
  became `unwrap_or_default()`, a fragment-less entry would produce `Some("")`, `follow_link`
  would park `Pending::Fragment("")` instead of `Pending::Nothing`, and the chapter would stay
  hidden behind the spinner waiting for a scroll report that never comes. That is a real
  failure mode of the `Pending` enum, not a tautology, and the fixture is the only thing
  watching it.
- **"Hoist the controlled-open triple into the repo's own `PopoverRoot` wrapper."** Correct, and
  now genuinely earned at two occurrences — but it edits a shared component with a layout blast
  radius, which is Step 7's business. Recorded in the scope note.

> **Status:** done — committed in `68dc393`, **115 tests green** (114 → 115), `cargo clippy
> --all-targets` clean, and the three touched files are rustfmt-clean. The pre-existing
> rustfmt drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is untouched by
> this step.

The eyeball half of the check was blocked, and not by this step. Clicking a panel row did
nothing and closed the panel, and so did every button in `SettingsPopover`. macOS WebKit does
not focus a `<button>` on mousedown, so focus fell to the nearest focusable ancestor —
`reader-root`, which has carried `tabindex="0"` since `57654d1` gave the arrow keys somewhere
to land. That element is outside the popover root, so `use_outside_dismiss` read the `focusin`
as a press on "outside" and unmounted the content mid-press; with no element left under the
pointer, `mouseup` never completed a `click` and no handler ran.

Two observations named it. The `<select>` pickers kept working while every button beside them
was dead, because form controls *do* take focus on click; and Tab-then-Enter drove the whole
jump correctly, because `FocusTrap` focuses the button directly and focus never leaves the
root — which is also what proved this step's code was right before the bug was found. Fixed in
`e67c428` by putting `tabindex="-1"` on the popover content, so the ancestor walk stops inside
the panel. It sits on the wrapper rather than on each button, and leaves the arrow-key feature
alone: `reader-root` being a focus sink is what makes that feature work, and only its position
relative to the popover was ever wrong.
>
> **The jump itself has not been eyeballed** — the agent cannot see the webview. The gate is
> the learner opening the panel under `dx serve`, clicking a top-level chapter (the document
> changes, the spinner shows, the panel closes), then a nested sub-entry such as "II." inside
> the chapter already open (no spinner, the frame scrolls to the fragment, the panel closes).

---

## Step 7 — review & refactor (sketched)

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

The phase-closing pass. Steps 5 and 6 each ended by parking something for this step rather than
doing it mid-feature, and the list is now nine items long — too long for one diff. Phase 4's
Step 8 hit the same wall and answered it by **sketching the triage first and landing the items
as lettered sittings** (`8a`…`8e`); this step follows that shape. What follows is the triage:
every carried item, what it actually costs, and whether it is worth doing.

**One item is already closed.** Step 4's duplicated chapter label in the `NavRow` went in
`c5e6d37` — the row was showing the chapter name a second time under the header that already
showed it. It is struck from the list below.

### The triage

**A. The two link resolvers overlap** *(Step 2's item — do it first)*. `toc_entries` and
`epub::resolve_internal_link` both turn "a path plus an optional fragment" into a
`LinkTarget`, and both spell the percent-decode by hand. Step 6 wrote a test asserting the two
agree *because nothing structural made them*. This is the layering item and Phase 4's rule
applies: do it first, because every remaining item is easier to judge once the module boundary
is honest. **Landed as [7a](#step-7a--one-resolver-one-target).**

**B. The tabler SVG preamble, third copy.** `reader.rs` (close), `settings.rs` (gear) and
`toc.rs` (list) each carry the same ten attributes — `xmlns`, `width`, `height`, `view_box`,
`fill`, `stroke`, `stroke_width`, `stroke_linecap`, `stroke_linejoin`, plus the `path` that
blanks the 24×24 box — and differ only in the icon's own `d` strings and class suffix. Thirty
lines of noise around three lines of signal. Wants an `ui/components/icon.rs` that takes the
paths; the layer already exists, because 8d built it. **Worth doing** — visual gate only.

**C. `--toc-depth` is spelled in two files with no compiler between them.** `ui/toc.rs:80`
writes it, `ui/toc.css:14` reads it. Rename one and the indentation silently flattens — no
error, no warning. The repo already has the answer for this hazard: the `assert!(JS.contains(…))`
tests in `ui/reader.rs` that pin `ook-reflow`, `ook-key`, `ook-warn` and `__ookBlobUrl` across
exactly this kind of gap. **Worth doing**, and it is the only item on the list that is genuinely
test-first.

**D. `stop_propagation` belongs in the shared `PopoverContent`.** Both popovers hang
`onkeydown: move |e| e.stop_propagation()` on their own inner wrapper, for the same reason —
`reader-root`'s arrow-key handler is an ancestor and would turn the page while you are typing
in the panel. Two copies of one defence against one specific ancestor. **Worth doing**, but it
edits the shared component, so pair it with F.

**E. `ul`/`li` list semantics for the panel.** The contents list is a `nav` full of bare
`button`s. A screen reader announces eighteen buttons with no sense that they are one list, and
no count. **Worth doing**, cheap, visual gate.

**F. The controlled-open triple, now in two files.** `settings.rs` and `toc.rs` both write
`use_signal` → `open:` → `on_open_change:`. Step 6 called this "a real candidate rather than a
speculative one" at two occurrences. **But look again before hoisting:** only `toc.rs` ever
calls `open.set(false)`. `settings.rs`'s copy never closes the panel from inside, so its triple
reproduces exactly what `PopoverRoot` does uncontrolled — and `open` is an optional prop
(`ReadSignal<Option<bool>>`, the same shape as `id`, which no caller passes). If that holds, the
finding is not "hoist the duplication" but **"delete three dead lines from `settings.rs`"**, and
what is left is one occurrence, which does not earn a shared abstraction. **Verify first, then
probably just delete.**

**G. Scroll the current row into view.** Carried from Step 5, made more visible by Step 6 —
jumping is how you move a long way through a long ToC, and the panel still opens at the top
every time. **This is a feature, not a refactor**, and a refactor step that also changes
behavior is two steps pretending to be one (Phase 4's 8e made that its opening rule). Either
give it its own commit at the end of this step, or push it to the feature's next phase. It also
needs a `use_effect` + `scrollIntoView` over an element the panel only just mounted, which is
the most machinery on this list.

**H. `toc::label_for_spine`.** Step 4's item. `entry_index_for_spine` returns an *index*, and
`ui/reader.rs`'s `chapter_label` immediately does `entries[index].label.clone()` — as do both
test closures in `toc.rs`, spelled identically. The panel is the only caller that genuinely
wants the index (for `aria-current`). A `label_for_spine(&entries, spine_index) -> Option<&str>`
built on top would delete the indexing dance from three places. **Small, worth doing, no rush.**

**I. `TocEntry` should carry a `LinkTarget`.** Not on the carried list — it fell out of A while
writing it, and is folded into 7a. See there.

### The ordering

A first (layering), then C (the one real test), then B + E + D + F as one UI-chrome sitting
(they touch the same three files), then H. G last and on its own commit, or deferred.

---

## Step 7a — one resolver, one target

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Item **A** from the triage, plus **I**, which is the same idea seen from the other end.

**The crux: Step 6 shipped a test where it should have shipped a function.** Its convergence
assertion — that the `LinkTarget` built from a ToC entry is byte-identical to the one
`resolve_internal_link` builds from the equivalent `<a href>` — was the right *claim*, and the
handoff defended it well: if the two paths ever drifted (one decodes `%20`, the other doesn't;
one strips a leading `/`, the other doesn't) the panel would jump somewhere subtly different
from the link, and only a test building both sides could see it.

But a test that watches two code paths agree is a **weaker instrument than one code path**. The
test can only catch the drift after someone writes it; a shared function makes the drift
unwriteable. And the drift was not hypothetical — the two resolvers were *already* asymmetric:
`toc_entries` trimmed a leading `/` from its path and `resolve_internal_link` did not, and each
spelled the percent-decode as its own three-line `percent_decode_str(…).decode_utf8_lossy()
.into_owned()` chain.

The second half falls out of the first. Once `toc_entries` calls the shared resolver, it gets a
whole `LinkTarget` back and immediately takes it apart into two `TocEntry` fields — which
`ui/toc.rs` then puts back together through the `From` impl Step 6 wrote. A value built,
destructured and rebuilt across three files is the type asking to be stored whole:

```rust
pub(crate) struct TocEntry {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) target: LinkTarget,
}
```

A ToC entry **is** a labelled, indented link target. Saying so deletes the `From` impl one step
after it was written, which is exactly what a review pass is for.

### The check — `cargo test`

`src/epub.rs`, next to the two tests that already own the URL side of link resolution:

```rust
#[test]
fn a_link_target_decodes_and_trims_before_matching_the_spine() {
    let hrefs = vec!["OEBPS/Chapter 1.xhtml".to_string()];

    assert_eq!(
        link_target(&hrefs, "/OEBPS/Chapter%201.xhtml", Some("s%20a")),
        Some(LinkTarget {
            spine_index: 0,
            fragment: Some("s a".to_string()),
        })
    );

    assert_eq!(link_target(&hrefs, "OEBPS/missing.xhtml", None), None);
}
```

**The red was a compile error**, the same shape as Step 6's:

```
error[E0425]: cannot find function `link_target` in this scope
```

A refactor cannot show a behavioural red — that is the point of a refactor — so the honest
report is: **the red is that the shared function does not exist yet**, and the safety net is the
114 tests that already described the old behaviour. What makes this test worth writing rather
than trivially green is that it pins the *new* contract in one line: `link_target` takes text
**exactly as a book wrote it** — percent-encoded, possibly with a leading `/` — and does the
normalizing itself.

**Both halves were then proved live by mutation**, which matters more here than usual because a
refactor's test is the only thing standing between "green" and "green but wrong":

| mutation | result |
|---|---|
| drop `.trim_start_matches('/')` | **7 tests fail**, `18` entries → `0` |
| drop `percent_decode(path)` | **2 tests fail**, this one and `resolves_a_percent_encoded_href_to_a_decoded_target` |

The first number is the interesting one. The fixture's ToC hrefs **do** carry a leading `/` while
its spine hrefs do not, so the trim is not defensive tidiness — it is the only reason the panel
has ever had eighteen rows instead of zero. Step 2 wrote that trim in `toc.rs` and nothing said
why; it now sits next to the comparison it exists to make succeed.

### The code

`src/epub.rs` — one decoder, one resolver:

```rust
fn percent_decode(raw: &str) -> String {
    percent_encoding::percent_decode_str(raw)
        .decode_utf8_lossy()
        .into_owned()
}

pub(crate) fn link_target(
    hrefs: &[String],
    path: &str,
    fragment: Option<&str>,
) -> Option<LinkTarget> {
    let path = percent_decode(path);

    Some(LinkTarget {
        spine_index: hrefs
            .iter()
            .position(|href| href == path.trim_start_matches('/'))?,
        fragment: fragment.map(percent_decode),
    })
}

pub(crate) fn resolve_internal_link(
    hrefs: &[String],
    current_index: usize,
    href: &str,
) -> Option<LinkTarget> {
    let (path, fragment) = match href.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (href, None),
    };

    if path.is_empty() {
        return Some(LinkTarget {
            spine_index: current_index,
            fragment: fragment.map(percent_decode),
        });
    }

    link_target(hrefs, path.strip_prefix(EPUB_URL_PREFIX)?, fragment)
}
```

`src/toc.rs` — the whole per-entry body, after:

```rust
let href = entry.href()?;

Some(TocEntry {
    label: entry.label().to_string(),
    depth: entry.depth().saturating_sub(1),
    target: epub::link_target(docs, href.path().as_str(), href.fragment())?,
})
```

`src/ui/toc.rs` — the `From` call becomes what it always was:

```rust
let target = entry.target.clone();
```

### Why it works

**`toc.rs` stopped knowing three things it had no business knowing.** Before, it knew that spine
hrefs carry no leading slash, that rbook's fragments arrive percent-encoded, and how this repo
spells a lossy percent-decode. All three are facts about `epub.rs`'s data — and the first one is
established inside `spine_hrefs`, which is **private**, so `toc.rs` was hand-satisfying a
precondition of a function it cannot even see. Now it knows one thing: that `epub` will resolve
an href against the spine. `use percent_encoding` is gone from the file entirely.

**The direction of the decode was the real design decision, and the first draft got it
backwards.** A shared resolver can take *decoded* text (callers decode, helper compares) or
*raw* text (helper decodes). The draft took decoded text because that is the shape the two
callers happened to have lying around — and that produced a signature with a decoded `path` and
a raw `fragment`, an asymmetric precondition that nothing enforces and whose violation is
**silent**: hand `link_target` a raw path and you get `None`, which reads as "not in this book".
Taking raw text on both sides gives one rule — *hand me exactly what the book wrote* — and it is
the rule that cannot be violated by accident, because raw text is what every caller already has
before it does anything.

**`href.path().as_str()` versus `href.path().decode()`.** rbook's `Href<'a>` is a newtype over
`&'a str`; `path()` returns another `Href` narrowed to the path segment, `as_str()` hands back
the borrowed original, and `decode()` is the opt-in that allocates a `Cow`. Passing `as_str()`
means the ToC path is decoded in exactly one place — inside `link_target` — instead of once by
rbook and then compared against a spine that `spine_hrefs` decoded separately. Note the returned
lifetime is `'a`, not tied to `&self`, so `href.path().as_str()` on a temporary `Href` is fine:
the `&str` outlives the temporary it came from.

**Why the bare-fragment branch does not go through `link_target`.** `#note` with no path means
"this document", and `link_target` resolves by `position()` — which would find the *first* href
equal to the current one, not `current_index` itself. In a book where two spine entries share an
href (legal; the spine is a list of idrefs, not a set) that silently jumps you backwards. So the
branch keeps its own `LinkTarget`, and `fragment.map(percent_decode)` is spelled twice in the
file on purpose.

**The convergence test survives and still earns its place.** With one resolver, "a picked entry
and a followed link agree" is nearly true by construction — but only *nearly*: the two still
arrive by different routes (`/OEBPS/…` from rbook's nav document, `dioxus://index.html/epub/…`
from the webview) and the test pins that both routes normalize to the same key. The trim mutation
above breaks it, so it is not a tautology.

**`TocEntry` nesting a `LinkTarget` does not cost the memoization Step 5 bought.** `LinkTarget`
derives `Eq`, so `TocEntry` keeps its `Eq` derive, so `Vec<TocEntry>: Eq` — and `PartialEq for
Rc<T>` short-circuits on `Rc::ptr_eq` when `T: Eq`. The panel's prop diff stays a single pointer
comparison and never walks the eighteen entries or the new nested field. That chain is
load-bearing rather than incidental: had `LinkTarget` derived only `PartialEq`, `TocEntry`'s
`#[derive(Eq)]` would have failed to compile rather than silently degrading to an
element-by-element compare.

### Scope note

**This step is A and I from the triage only.** B (the icon preamble), C (`--toc-depth`),
D (`stop_propagation`), E (list semantics), F (the controlled-open triple) and H
(`label_for_spine`) are untouched, and G (scroll-into-view) is still a behaviour change that
wants its own commit. The ordering stands: C next, then B + D + E + F as one UI sitting, then H.

**`link_target`'s name does not carry its altitude.** `resolve_internal_link` takes a webview URL
and `link_target` takes a book-relative path, and nothing in the names says which is which — the
review flagged `spine_target` / `target_for_spine_path` as alternatives. Left alone: the pair
sits four lines apart in one file, and the signature now makes the input shape unambiguous
anyway. Reconsider if a third caller appears.

### What the `simplify` pass changed, and what it did not

The pass ran four ways and **reversed the step's central decision twice**, which is the useful
part of the record.

- **The draft had callers decode and `link_target` compare.** The simplification reviewer flagged
  the mixed contract (decoded path, raw fragment) and proposed making both arguments *decoded*;
  the altitude reviewer flagged the same asymmetry and proposed making both *raw*. Same defect,
  opposite fixes. The altitude direction won on the argument above — one enforceable rule beats
  one uniform-but-unenforceable one — and it is strictly smaller: `toc.rs` lost its `path`
  binding, its trim and its decode, and `percent_decode` went back to private because nothing
  outside `epub.rs` calls it any more.
- **The trim moved into `link_target`** on the altitude reviewer's argument that `toc.rs` was
  satisfying a private function's precondition. The mutation table above is what proved the
  point worth acting on.
- **`zip_path_for` was still hand-rolling the decode** a hundred lines below the new helper — the
  reuse reviewer caught the extraction being two-thirds done. One-line fix, and it means
  `percent_decode_str` now appears exactly once in the crate.
- **The first test triplicated an existing fixture.** The draft asserted the decoding rule that
  `resolves_a_percent_encoded_href_to_a_decoded_target` already owns; it was cut back to what
  `link_target` alone decides — the trim, and the miss returning `None`.
- **A leftover `use crate::epub;` in `toc.rs`'s test module** became dead when line 3 changed to
  `epub::{self, LinkTarget}`. No warning, because an explicit import shadows a glob. Deleted.

**One finding was skipped:** a `TocEntry::spine_index()` accessor to flatten the `.target.`
hop at the two `entry_index_for_spine` reads. Cosmetic, and most of the `.target.` noise is in
tests, where being explicit about which coordinate is being compared is a feature.

> **Status:** done — committed in `54e8ea7`, **115 tests green** (114 → 115), `cargo clippy
> --all-targets` clean, and the three touched files are rustfmt-clean. The pre-existing rustfmt
> drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is untouched by this step.
>
> **No eyeball needed.** This is the phase's first step with no UI surface at all: the panel, the
> header label and the jump are behaviourally identical, and the fixture tests cover every path
> through the new resolver. Worth one `dx serve` anyway to confirm the panel still shows eighteen
> rows, since the mutation table shows how quiet the failure mode is if the normalization is
> wrong.

---

## Step 7b — pin `--toc-depth` across the Rust/CSS gap

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Item **C** from the triage, and the only item on the list that is genuinely test-first.

**The crux: the ToC panel's indentation is held together by a string that no compiler reads.**
`ui/toc.rs` writes `--toc-depth` into an inline `style` attribute; `ui/toc.css` reads it back out
of a `calc()`. Between those two lines there is nothing — not rustc, not the `#[css_module]`
macro, not the CSS parser — that knows the two names are supposed to be the same word. Rename
either side and the build stays green, clippy stays quiet, and the panel silently flattens: every
row draws at the fallback `0`, so an eighteen-entry nested ToC renders as eighteen entries at the
same indent. Nothing errors. It just stops meaning anything.

This is the same hazard the repo has already met four times on the Rust↔JS side, and it already
has the answer. `ui/reader.rs` pins `ook-reflow`, `ook-key`, `ook-warn` and `__ookBlobUrl` with
`assert!(SOME_JS.contains("…"))`, each under a comment naming the silent failure — *"rename it in
the loader and the cleanup silently revokes nothing, leaking a chapter per book."* The gap here is
Rust↔CSS rather than Rust↔JS, but it is the same gap and it takes the same instrument.

**The one thing the idiom needs that a straight copy does not give.** In `reader.rs` both sides of
the shared name are *files*, so two `contains` calls pin both ends. Here one side is *code* — a
literal inside `rsx!`. Asserting `TOC_CSS.contains("--toc-depth")` would pin only the CSS: rename
the Rust literal and the test sails through, which is the direction the failure is most likely to
come from, since the Rust side is the one someone edits while writing a component. So the name has
to become a value first:

```rust
const DEPTH_VAR: &str = "--toc-depth";
```

Once the component interpolates the const instead of spelling the word, the test compares the
const against the CSS and **both** directions are live. That is the whole step: hoisting a string
literal into a constant is what turns an untestable gap into a testable one.

### The check — `cargo test`

`src/ui/toc.rs` gets its first test module:

```rust
#[cfg(test)]
mod test {
    use super::*;

    const TOC_CSS: &str = include_str!("toc.css");

    #[test]
    fn the_depth_variable_is_spelled_the_same_on_both_sides_of_the_css_gap() {
        assert!(TOC_CSS.contains(&format!("var({DEPTH_VAR}")));
    }
}
```

**The red was a compile error**, and honestly so — the constant the test needs is the constant the
step exists to introduce:

```
error[E0425]: cannot find value `DEPTH_VAR` in this scope
   --> src/ui/toc.rs:107:49
```

**Then both directions were proved live by mutation**, which is the only evidence that matters for
a test whose whole job is to notice a rename:

| mutation | result |
|---|---|
| `DEPTH_VAR` → `"--toc-indent"` (Rust side) | **fails** |
| `var(--toc-depth, 0)` → `var(--toc-indent, 0)` (CSS side) | **fails** |
| *control:* `var(--toc-depth, 0)` → `var(--toc-depth)` | **passes** |

Two mutations rather than one is the point. A test that only caught the CSS rename would have been
half a test, and it would have looked exactly the same in the diff. The control row is the other
half of the claim — see the `var(` discussion below.

**Why `var({DEPTH_VAR}` and not just `DEPTH_VAR`.** The bare form passes on any mention of the name
anywhere in the file — including a comment, or a `--toc-depth:` *declaration* that sets the
variable without anything ever reading it. Asserting the `var(` prefix pins that the CSS **reads**
the property, which is the relationship that actually has to hold.

**And why the prefix stops before the comma.** The draft asserted `var({DEPTH_VAR},`, which pins
the fallback's existence as a side effect. But the fallback is unreachable: `TocEntry::depth` is a
plain `usize`, unconditionally set, and the `style` attribute is written unconditionally on every
row — so no `.contents-popover__entry` can ever exist without the property already set. Deleting
`, 0` would be a legitimate future tidy-up, and a test that fails on it fails for a reason that has
nothing to do with what it is named after. The control row above is what proves the trimmed form
draws the line in the right place: rename either side and it fails, delete the dead fallback and it
does not.

### The code

`src/ui/toc.rs`, two lines:

```rust
const DEPTH_VAR: &str = "--toc-depth";
```

```rust
style: "{DEPTH_VAR}: {entry.depth};",
```

### Why it works

**`include_str!` resolves relative to the source file, so the test reads the real CSS, not a
copy.** `include_str!("toc.css")` next to `src/ui/toc.rs` embeds `src/ui/toc.css` at compile time —
the same bytes the `#[css_module]` macro above it consumes. There is no build step to keep in sync
and no fixture to go stale: edit the CSS and the next `cargo test` compiles the new bytes into the
constant. (The two spellings of the path — `"/src/ui/toc.css"` for the macro, `"toc.css"` for
`include_str!` — are a wart, but not one this step can fix: the macro takes a workspace-absolute
path and `include_str!` takes a file-relative one, and neither accepts the other's form.)

**The constant lives in the test module, so nothing pays for it at runtime.** `TOC_CSS` is inside
`#[cfg(test)]`, so the CSS text is compiled into the test binary only; the shipped binary never
carries a second copy of a stylesheet it already links through the macro. Had the constant gone at
module level next to `DEPTH_VAR`, it would have been both dead code in release builds and a
`dead_code` warning in them.

**`{DEPTH_VAR}` inside `rsx!` is an ordinary formatted string, and that is why it is safe.** Dioxus
0.7's `rsx!` treats an attribute's string value as a format string over identifiers in scope — the
same machinery already used one line above for `class: "{Styles::contents_popover__entry}"`. A
`&'static str` const interpolates like any other `Display` value, so the attribute still produces
one `String` per row per render exactly as the literal did. The change is in what the compiler
*sees*: a literal is opaque text, whereas the const is a name the test can reach.

**Why a test rather than a deeper fix — checked, not assumed.** The obvious deeper fix would be for
`#[css_module]` to expose custom properties as checked symbols the way it exposes classes as
`Styles::foo`; then `Styles::TOC_DEPTH` would be a compile error to misspell and no test would be
needed. The macro comes from `manganis-macro` 0.7.9, and its `css_module_parser.rs` builds the
`Styles` symbols in `get_class_mappings`, which walks `CssFragment::Class` and `Global` selectors
only — there is no handling of `--custom-property` declarations or `var(…)` usages anywhere in the
crate. So the checked symbol does not exist to reach for, and manufacturing it means editing a
third-party macro, which is well outside a phase-closing refactor. The test is the cheapest
instrument that closes the gap today; if manganis grows the feature later, this test is the thing
that gets deleted.

### Scope note

**This step is C from the triage only.** B (the icon preamble), D (`stop_propagation`), E
(list semantics) and F (the controlled-open triple) remain, and the plan still lands them as one
UI-chrome sitting because they touch the same three files. H (`label_for_spine`) follows, and G
(scroll-into-view) is still a behaviour change wanting its own commit or a push to the next phase.

**Nothing else in the panel crosses an unchecked gap.** Every other name shared between
`ui/toc.rs` and `ui/toc.css` is a class, and classes go through `Styles::…`, which rustc checks.
`--toc-depth` was the only one, which is why this step is one assertion and not a suite.

### What the `simplify` pass changed, and what it did not

Three of the four reviewers came back clean; the useful part is that the two findings that landed
**pulled in opposite directions on the same line**, and the resolution is not either one's.

- **The assertion lost its trailing comma.** The simplification reviewer wanted the whole `format!`
  gone — `contains(DEPTH_VAR)`, on the argument that `--toc-depth` is distinctive enough to pin by
  itself. The altitude reviewer, answering a different question, established that the `, 0` fallback
  is dead code. Put together they say the draft was pinning *two* things and only one of them is a
  real invariant: keep `var(`, which pins the relationship that must hold, and drop the comma, which
  pinned a line someone is entitled to delete. The control mutation was added to the table to make
  that boundary a fact rather than a claim.
- **The test name said `indent` while everything it names says `depth`.** Renamed to
  `the_depth_variable_…`. A test whose name does not match the symbol it pins is the one kind of
  drift this step cannot afford to ship.
- **The altitude reviewer read the macro rather than guessing at it**, which is what turned "a test
  is probably the right depth here" into the `manganis-macro` paragraph above. The draft asserted
  the same conclusion with no evidence behind it.

**One finding was skipped:** the two spellings of one path — `#[css_module("/src/ui/toc.css")]`
against `include_str!("toc.css")` — are a second, smaller version of the gap this step closes. Real,
but not closeable: the attribute macro resolves from the crate root and `include_str!` resolves from
the source file, neither accepts the other's form, and no literal can be shared between an attribute
argument and a macro call. Noted in the "Why it works" section instead. Unlike the CSS variable, this
one fails **loudly** at compile time if it ever breaks, which is why it is a wart and not a hazard.

**Reuse and efficiency both came back clean.** No existing helper covers this gap —
`Settings::css_vars` and `Theme::css_vars` build the global `:root` push, a different mechanism for
a different job — and the const costs one extra `Display::fmt` of an 11-byte `&'static str` per row
on a line that was already allocating a `String` for `{entry.depth}`.

> **Status:** done — committed in `fae2125`, **116 tests green** (115 → 116), `cargo clippy
> --all-targets` clean, and `src/ui/toc.rs` is rustfmt-clean. The pre-existing rustfmt drift in
> `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is untouched by this step.
>
> At commit time the assertion was re-proved live the sanctioned way — by inverting the test
> itself (`assert!(!…)`) and watching it fail, then restoring it. That is a weaker instrument than
> the mutation table above, which moves the *names*, but it is the one that never touches the
> implementation, and it agrees with it.
>
> **No eyeball needed.** The rendered `style` attribute is byte-identical before and after: the
> const holds the same eleven characters the literal did. Nothing about the panel changed, which is
> the whole point — this step buys a compiler-adjacent guarantee, not a behaviour.

---

## Step 7c — one icon component

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Item **B** from the triage, alone. **The triage's "UI-chrome sitting" was split before writing
it**: B + D + E + F was four ideas bundled on the grounds that they touch the same three files,
and the icon extraction by itself is a new module plus three call sites. D + E + F became 7d, and
the two items after them shifted a letter. Nothing was dropped.

**The crux: the thing worth deduplicating is not the thing that looks duplicated.** Three files
carried the same ten `svg` attributes and the same inert `M0 0h24v24H0z` blanking path — thirty
lines of preamble around three lines of signal, and the obvious move is to hoist the preamble into
a component that takes the paths. That move is right, and it is also *half the step*, because it
leaves each icon's actual identity — its geometry — sitting at the call site it was pasted into.
`reader.rs` still owns the two `M` strings that spell an ✕; nothing can reuse them; nothing checks
a fourth copy against them.

The other half only becomes visible once the component exists. A component taking `name` and
`paths` as two independent props has **two values that must agree and no compiler between them** —
`Icon { name: "x", paths: GEAR }` compiles fine and renders a gear tagged `.icon-tabler-x`. That is
precisely the hazard [Step 7b](#step-7b--pin---toc-depth-across-the-rustcss-gap) just spent a whole
sitting closing at the Rust/CSS layer, and a component that reopened it one level up would be a
poor advertisement for the step before it.

Rust closes it outright, and the mechanism is the plainest one in the language: **make the fields
private and hand out constants.**

```rust
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TablerIcon {
    name: &'static str,
    paths: &'static [&'static str],
}

pub(crate) const CLOSE: TablerIcon = TablerIcon {
    name: "x",
    paths: &["M18 6l-12 12", "M6 6l12 12"],
};
```

`name` and `paths` have no `pub`, so outside `icon.rs` the struct cannot be built at all. The only
`TablerIcon` values that exist are the three the module defines, each of which pairs a class suffix
with the geometry it belongs to, once. A call site does not get to disagree — there is no
expression it can write that would.

### The check — `dx serve` + `cargo clippy`

**There is no red to report, and it should be said plainly.** This is a pure markup extraction:
the assertion worth making is "the three icons render exactly as before", and nothing in the crate
can see rendered markup. There is no `dioxus-ssr` dev-dependency, so no test can compare the
`svg` subtree the component emits against the literal blocks it replaced. Like
[Step 5](#step-5--render-the-contents-panel), the gate is a clean build plus an eyeball.

**What to look for under `dx serve`:** the ✕ in the reader's top-left, the gear and the list icon
side by side in the header. All three should be the same 24×24 outline glyphs at the same weight
as before — if the `stroke_width`, `stroke_linecap` or the blanking path had been dropped in the
move, they would render heavier, with squared-off ends, or with a filled black box behind them.

**One claim in this step *is* mechanically checkable, and was checked.** That the mismatch is now
impossible is a statement about the compiler, so the compiler can be asked. Replacing a call site
with a hand-built struct — `Icon { icon: icon::TablerIcon { name: "x", paths: &["M9 6l11 0"] } }` —
gives:

```
error[E0451]: fields `name` and `paths` of struct `TablerIcon` are private
  --> src/ui/toc.rs:39:49
```

That error is the step's real deliverable. Before this step the equivalent mistake was thirty lines
of copy-paste with a wrong word in the middle of it, and it compiled.

### The code

`src/ui/components/icon.rs` — the struct and three consts above, then:

```rust
#[component]
pub(crate) fn Icon(icon: TablerIcon) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            class: "icon icon-tabler icons-tabler-outline icon-tabler-{icon.name}",
            path {
                stroke: "none",
                d: "M0 0h24v24H0z",
                fill: "none",
            }
            for d in icon.paths.iter().copied() {
                path { d }
            }
        }
    }
}
```

`src/ui/components/mod.rs` gains `pub mod icon;`, and the three call sites collapse to one line
each:

```rust
Icon { icon: icon::CLOSE }
Icon { icon: icon::SETTINGS }
Icon { icon: icon::LIST }
```

Net across the three call sites and `mod.rs`: **84 lines deleted, 19 added**, against a 58-line
new module — so the crate is about even on total lines and three files lighter by thirty each.
That trade is the point: the lines that left were *copies*, and the lines that arrived are the
only copy.

### Why it works

**Private fields are the enforcement, not a naming convention.** Rust's field privacy is
module-scoped, so `icon.name` reads fine inside `icon.rs` — the component needs it — while
`icon::TablerIcon { .. }` from `toc.rs` is `E0451`. This is the cheapest form of "make illegal
states unrepresentable" available: no enum, no constructor function, no validation, just the
absence of `pub` on two lines. The type is still `pub(crate)` because call sites must be able to
*name* it in the prop position; only its interior is closed.

**`Copy` is what lets the consts be used more than once.** `TablerIcon` is two `&'static`
references — 32 bytes of pointers, no ownership — so `Copy` is free and correct. Without it,
`icon::CLOSE` would move on first use and a second call site referencing the same const would need
an explicit `.clone()`. `PartialEq` is not optional either: Dioxus derives the props struct's
equality from its fields, and that comparison is what lets a re-render skip a child whose props did
not change.

**`for d in icon.paths.iter().copied()` buys legibility, not speed — and the difference matters
because the first draft of this note claimed otherwise.** Iterating `&'static [&'static str]`
directly yields `&&'static str`, which has no `IntoAttributeValue` impl, so the slice cannot feed
rsx's bare-ident shorthand without flattening first; `.copied()` is what makes `path { d }` (i.e.
`d: d`) typecheck at all. The alternatives are `path { d: *d }` or `path { d: "{d}" }`, and **all
three allocate exactly the same**, because `impl IntoAttributeValue for &str` in
`dioxus-core-0.7.9/src/nodes.rs:1008` is `AttributeValue::Text(self.to_string())` with no `Cow` or
`Rc<str>` fast path. The choice is a readability one and nothing more.

**The extraction does add allocations the inline markup never had, and memoization is what makes
that fine.** A literal `d: "M18 6l-12 12"` is a *static* template attribute — baked into the
compiled `Template`, allocated never. Looping over a slice makes it a *dynamic* attribute, so each
path costs a `String`, and the `class` string costs a `format!` on top because static text precedes
`{icon.name}`. What keeps this from being a per-frame cost is the `PartialEq` derive: Dioxus checks
`Properties::memoize` *before* running a child's body, `TablerIcon` is two `&'static` references
that never change, so the props compare equal on every re-render after the first and the body is
skipped entirely. The eight `String`s and three `format!`s across the three icons happen once each,
at mount.

**That is also the argument for `#[component]` over a plain function.** A bare
`fn icon(..) -> Element` called inline gets no scope and no memoize check, so its body would
re-execute on every parent render — rebuilding the class string and every path `String` each time.
For the ✕, which lives inside `Reader` and therefore re-renders on every page turn, the plain
function would be strictly worse. The price of the component form is one small `Box<dyn AnyProps>`
per parent render, allocated in `VComponent::new` *before* the memo check can skip anything; that
is the single genuine cost this step adds, and it buys skipping all the work above.

### Scope note

**This step is B only.** D (`stop_propagation` into the shared `PopoverContent`), E (`ul`/`li` list
semantics) and F (the controlled-open triple, which looks like three dead lines in `settings.rs`)
are now 7d. `label_for_spine` is 7e and scroll-into-view is 7f.

**The reader's close button is still a hand-rolled `button { class: "icon-button" }`** while the
other two icons reach the same class through `PopoverTrigger`. That duplication predates this step
and this step does not touch it; it belongs with 7d's popover-chrome work if it is worth closing at
all.

**No `attributes` pass-through.** `PopoverTrigger` and `PopoverContent` take `props: XProps` and
merge caller attributes; `Icon` takes a named prop like `SlugPicker` does. Adding a pass-through
for a component no caller wants to customize is speculative generality — revisit when a call site
actually needs to put an `aria_hidden` on an icon.

### What the `simplify` pass changed, and what it did not

The pass **rewrote the step's central design**, and then two of its reviewers **contradicted each
other** on a line of the result. Both are worth the record.

- **The draft stopped at half the idea.** It shipped `Icon { name, paths }` with the geometry left
  inline at each call site — the preamble deduplicated, the identity still scattered. The altitude
  reviewer named the consequence: a fourth call site wanting the ✕ has nothing to import and must
  re-type two `M` strings with no compiler checking the copy. It then made the sharper point, that
  two independent props which must agree is the *same hazard class Step 7b had just closed* one
  layer down. Hoisting the pair into private-field consts fixed both at once, and the `E0451` probe
  in the check section is that fix being asked to prove itself.
- **The simplification and efficiency reviewers disagreed about `path { d }`.** Simplification said
  the bare-ident shorthand passes the `&'static str` through with no allocation and that
  `d: "{d}"` wastes a `String` per path; efficiency said `AttributeValue::Text` always owns a
  `String`, so the two forms are identical. **Efficiency was right**, confirmed by reading
  `dioxus-core-0.7.9/src/nodes.rs:1008` rather than taking either agent's word: `&str` converts via
  `self.to_string()` unconditionally. The change was kept — `path { d }` reads better than
  `path { d: *d }` — but the "why it works" paragraph above was rewritten, because it had already
  repeated the wrong reason.
- **The efficiency reviewer supplied the argument the draft was missing**: that `#[component]`'s
  memoize check is what turns a per-render cost into a per-mount one, and that a plain function
  would have been strictly worse for the icon living inside `Reader`.

**Two findings were skipped.** `Icon` takes named props rather than the `props: XProps` +
`merge_attributes` pass-through that `PopoverTrigger`/`PopoverContent` use — noted in the scope
note as speculative until a caller needs it. And the reviewer suggested renaming the component
`TablerIcon` for vendor honesty; the *type* took that name instead, so `icon::CLOSE` is visibly a
`TablerIcon` while call sites keep the role-shaped `Icon { .. }`. If the icon set is ever swapped,
the call sites should not have to care.

**Reuse came back clean on coverage**, independently confirmed: `grep` for `svg {` and
`icon-tabler` across `src/` now matches only `icon.rs`, and `assets/main.css` styles only
`.icon-button` — the wrapper this step never touched — so no CSS was stranded.

> **Status:** done — committed in `6a16fe3`, **116 tests green** (unchanged; this step adds no
> test), `cargo clippy --all-targets` clean, and all five touched files are rustfmt-clean. The
> pre-existing drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is untouched.
>
> **A markup extraction cannot be unit-tested here, so it was verified against `HEAD` instead.**
> Three mechanical comparisons stand in for the test this step does not have:
>
> | check | result |
> |---|---|
> | the ten `svg` preamble attributes, old vs new | byte-identical; the only change is `icon-tabler-x` → `icon-tabler-{icon.name}` |
> | all 10 non-blank path strings, as a set | identical — nothing lost or altered in the move |
> | each const against the file it came from, in order | `CLOSE`←`reader.rs` (2), `SETTINGS`←`settings.rs` (2), `LIST`←`toc.rs` (6) — no swap between icons |
>
> The third is the one that matters: comparing the paths as a *set* would have passed even if two
> icons had traded geometry, which is exactly the mistake this kind of move invites.
>
> **The eyeball is still outstanding.** The agent cannot see the webview, and the checks above
> prove the markup is the same, not that it *renders* the same. The gate is opening `dx serve` and
> confirming three 24×24 outline glyphs at unchanged weight: the ✕ top-left, the gear and the list
> icon in the header. A dropped `stroke_width`, `stroke_linecap` or blanking path would show as
> heavier strokes, squared-off ends, or a filled box behind a glyph.

---

## Step 7d — the popover owns its keyboard

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Items **D** and **F** from the triage. **7d was split again**, on the same grounds 7c was: the
leftover bundle was D + E + F, and `ul`/`li` list semantics (E) is a different idea with a different
gate — it rewrites the panel's DOM *and* its CSS, and it is checked by reading markup rather than by
pressing keys. E is now 7e, `label_for_spine` is 7f and scroll-into-view is 7g. Nothing was dropped.

D and F belong together because they are the same question asked twice: **which of these lines is
the call site's, and which belongs to the popover?** One answer was "shared". The other was
"nobody's".

### The crux

Both popovers hung `onkeydown: move |e| e.stop_propagation()` on their own innermost wrapper, for a
reason that belongs to neither of them. `reader.rs:168` puts an `onkeydown` on `.reader-root` that
maps ArrowLeft/ArrowRight to page turns, and every popover renders inside it — so an arrow pressed
while the settings panel's `<select>` has focus would move the selection *and* turn the page behind
the panel. That is not a property of "the settings panel" or "the contents list", it is a property
of **being a popover in this app**, which is exactly what the repo's own wrapper is for.

The part worth reading the vendor source for is **what `stop_propagation` actually stops**, because
three layers have to agree for the guard to work *and* for it not to break the popover's own
keyboard:

1. **Dioxus desktop registers one delegated `keydown` listener, on the mount root.**
   `BaseInterpreter.createListener` puts every bubbling event on `this.root`
   (`dioxus-interpreter-js-0.7.9/src/ts/core.ts:113-118`), i.e. `#main`. The native event has
   therefore already bubbled past all of our nodes before Rust hears about it at all.
2. **Rust then walks the *virtual* tree** from the event's target upward, calling each `onkeydown`
   it finds. `e.stop_propagation()` sets `propagate = false`, which ends that walk. That is what
   stops `reader-root`'s handler — and it is why the guard works from *any* ancestor position of the
   focused element, which is what makes moving it from the inner `nav` up to the content div
   behaviour-preserving rather than a rewrite.
3. **It never touches the native event.** `native.js` calls `event.stopPropagation()` only if the
   response carries a `stopPropagation` field, and desktop's `SynchronousEventResponse`
   (`dioxus-desktop-0.7.9/src/webview.rs:628-632`) serializes `preventDefault` and nothing else. On
   desktop that branch is dead.

Point 3 is what makes a *blanket* keydown guard safe here instead of reckless. `dioxus-primitives`
closes a popover on Escape with a plain `document.addEventListener('keydown', …)` in the bubble
phase (`primitives/src/lib.rs:187-199`), and `document` sits above `#main`. If `stop_propagation`
reached the native event, guarding keydown anywhere inside a popover would silently kill
Escape-to-close — and it would already have been dead before this step, since the old guard was an
ancestor of every element the focus trap can focus (`focus-trap.js` focuses the first focusable
*descendant* on open). Because the flag never leaves Rust, the old position and the new one both
leave Escape alone. Outside-click dismissal was never at risk either way: it listens for
`pointerdown` in the **capture** phase (`lib.rs:210-225`), which nothing downstream can stop.

**The second half is a triple that turned out to be nobody's.** `use_controlled`
(`primitives/src/lib.rs:119-133`) keeps its own `internal_value` signal and defers to the `open`
prop only when that prop is `Some`. `SettingsPopover` never called `open.set(false)` — nothing
inside the settings panel closes it — so its `use_signal` → `open:` → `on_open_change:` triple
re-implemented the primitive's internal signal by hand, one layer out, and added nothing but work.
`ContentsPopover` keeps its copy, because `open.set(false)` after a pick is a real programmatic
close and the primitives expose no `PopoverClose` to reuse instead.

### The check — `dx serve` + `cargo clippy`

**There is no red and no test**, and both halves of that deserve a reason. Nothing in the crate can
see rendered markup (no `dioxus-ssr` dev-dependency), so no test can assert "the guard is on the
content div now"; and the claim that actually matters is about a keypress travelling through a
webview, which is not a claim `cargo test` can hold.

**The compiler does check half of it.** F's claim is a claim about the props builder: that
`SettingsPopover`'s triple is *removable*. `open: ReadSignal<Option<bool>>` is an optional prop by
the same Option-typed-prop rule that already lets both call sites omit `id`, so the fact that the
crate still builds with `open:` and `on_open_change:` deleted is the proof — had `open` been
required, this would not have compiled at all.

**What to look for under `dx serve`** — four things, and the last two are the ones that would catch
a mistake:

1. **The guard.** Open the settings panel, put focus in the theme or font `<select>`, press
   ArrowLeft/ArrowRight several times. The selection moves; the page behind the panel must *not*
   turn. Then the same in the contents panel with a row focused.
2. **F under test.** The settings panel must still open **and close** from its own trigger, and
   still dismiss on Escape and on an outside click. If the deleted triple had been load-bearing,
   the symptom would be a panel that opens once and then refuses to close.
3. **Mutation, to watch the assertion fail.** Comment out the `onkeydown` line in `PopoverContent`
   and repeat (1): the page should turn under the open panel, in *both* popovers. That is the only
   way this step's claim can be observed red, and doing it in both panels is what proves one shared
   line replaced two.
4. **7c's outstanding eyeball, while you are in there.** The ✕, gear and list icons at unchanged
   24×24 weight.

### The code

`src/ui/components/popover/component.rs` — the guard joins the base attribute list that already
carried `tabindex`:

```rust
let base = attributes!(div {
    tabindex: "-1",
    onkeydown: move |e| e.stop_propagation(),
});
let merged = merge_attributes(vec![base, props.attributes]);
```

`src/ui/settings.rs` — the triple and the call site's own guard go:

```rust
pub(crate) fn SettingsPopover() -> Element {
    rsx! {
        PopoverRoot {
            PopoverTrigger { Icon { icon: icon::SETTINGS } }
```

`src/ui/toc.rs` — one line off the `nav`; the `open` signal stays.

Net: **4 insertions, 7 deletions** across three files.

### Why it works

**`attributes!` takes event handlers, and takes them with inference intact.** The macro parses each
entry with `dioxus_rsx`'s own attribute parser and sets the element name before calling
`rendered_as_dynamic_attr`, so `onkeydown` resolves through `div`'s attribute definition exactly as
it would inside `rsx!`. The closure therefore needs no type annotation — the first draft wrote
`move |e: KeyboardEvent|` and the annotation turned out to be unnecessary, which matters for a small
reason worth having: the line is now byte-identical to the two lines it replaced, so the move is
visibly a move.

**Ancestor position is the whole mechanism, and it is why this move is safe.** Because Dioxus walks
the virtual tree from the event target upward (crux, point 2), a handler anywhere on that path
stops the walk. The old guards sat on the `nav`/`div` one level below the content div; the new one
sits on the content div. Every element the focus trap can focus is a descendant of both, so the set
of events the guard sees is unchanged — with one addition, the content div *itself*, which
`tabindex: "-1"` makes click-focusable. A key pressed with the container focused used to reach
`reader-root` and turn the page; now it does not. That is the one behaviour difference in the diff,
and it is the bug the guard exists to prevent.

**Deleting the triple is not just tidier, it removes per-render work.** `open: open()` passes a
plain `bool` into a `ReadSignal<Option<bool>>` prop, which goes through `SuperFrom` —
`ReadSignal::new(Signal::new(…))` (`dioxus-signals-0.7.9/src/props.rs:12`) — so **every** render
built a fresh signal, owned by the scope and living until unmount. Reading `open()` in the body also
subscribed `SettingsPopover`'s scope, so each toggle re-ran `SettingsPopover`, all three wrapper
components, and their `attributes!` + `merge_attributes` passes. And `use_controlled`'s `set_value`
did `internal_value.set(x)` *and* `on_change.call(x)` → `open.set(v)`, dirtying two scopes per
toggle. With the prop gone, the write lands on the primitive's own internal signal inside its own
scope: one write, and the re-render is confined to the primitive subtree. The six control children
were memoized before and after — zero-field props always compare equal.

**Why the wrapper and not the primitives.** The invariant is ours: it exists because
`.reader-root` claims arrow keys for page turns. Patching a vendored component to know about that
would put an app rule in a library; the wrapper layer exists precisely to hold app rules about
somebody else's component.

### Scope note

**This step is D + F only.** E (`ul`/`li` list semantics) is 7e, `toc::label_for_spine` is 7f, and
scroll-the-current-row-into-view is 7g.

**The hazard this step leaves open, recorded here because nothing else can hold it.**
`merge_attributes` documents that *"event handler attributes are not merged/combined yet"*
(`primitives/src/lib.rs:347`) and later lists win, with `props.attributes` last — so a future
`PopoverContent { onkeydown: … }` **replaces** the guard rather than adding to it, and the page
quietly starts turning behind that popover. No test in this crate can see it, and `CLAUDE.md`
forbids the comment that would warn about it at the site, so this paragraph is the whole defence.
The fix, if it ever bites, is to stop merging and read the caller's handler explicitly.

**The depth question left unresolved on purpose.** The altitude reviewer argued the guard belongs in
`reader.rs` instead, as "turn pages only when no overlay is open" — one node, and every future
overlay (a dialog, a search field, a bare `<input>` in the header) gets it for free instead of
having to rediscover the incantation. That is a better shape and it is **not this step**: it needs
app-level overlay state, it deletes the popover guard rather than moving it, and it changes
behaviour in a case the current fix misses (panel open, focus stray on the body). It is the natural
next move if a third overlay ever appears.

**Left alone, all noted and none of them this step's idea:** the settings panel's wrapper `div`,
which now carries only styling (`padding: 0.5rem` + `gap: 0.5rem` against `.dx-popover-content`'s
own `0.25rem` flex column) and its now-inert `gap: "0.25rem"` prop; the hand-rolled `if let/else` +
`format!` class merge in `PopoverContent`, which `merge_attributes` would join for free; the entry
`onclick`'s `stop_propagation` in `toc.rs`, which has no ancestor `onclick` to stop; `PopoverRoot`'s
`props.id`, which the wrapper accepts and then never forwards to the primitive; and the reader's
hand-rolled `button { class: "icon-button" }`, carried from 7c.

### What the `simplify` pass changed, and what it did not

**It changed nothing, and that is the honest report** — no edits, so the numbers below are the same
ones the step was already green on. Reuse and efficiency both came back clean; the other two found
real things that are not this diff's to fix.

- **Reuse confirmed the move is complete**: after this step the only `stop_propagation` outside the
  wrapper is `toc.rs`'s entry *onclick*, a different concern. It also floated an
  `IconPopover { icon, children }` to fold away the now-identical
  `PopoverRoot` + `PopoverTrigger { Icon }` + `PopoverContent { align: End }` shell — declined,
  because it saves about four lines across two call sites and `ContentsPopover` would still need its
  own open signal.
- **Efficiency supplied the accounting in "why it works"** — the per-render `Signal::new` living
  until unmount, the subscribe-and-re-render-the-subtree cost of reading `open()`, and the two
  writes per toggle collapsing to one. It also noted that a listener in the base list permanently
  defeats props memoization of the primitive's content components, and that this is not a
  regression: `children: Element` compares by `Rc::ptr_eq` and already forced those re-renders.
- **Three simplification findings were skipped as neighboring code**, all now in the scope note:
  the settings wrapper `div`, its dead `gap`, and the class merge. Deleting the `div` is the
  tempting one and it is a *visual* change — its padding and gap differ from the shared content
  style — so it does not belong in a diff whose only gate is "nothing looks different".
- **Altitude contributed both the recorded hazard and the depth argument** above.

**One correction to my own reading, since it changed what this step claims.** Working forward from
`native.js` alone — which does call `event.stopPropagation()` on `response.stopPropagation` — it
looked as though a blanket keydown guard would swallow the primitives' `document`-level Escape
listener, making the move a small regression. Reading the Rust side settled it the other way:
desktop's `SynchronousEventResponse` has one field, `preventDefault`, so the flag never crosses back
into JS and Escape is untouched. The crux says so now because the wrong version of that paragraph
was written first.

> **Status:** done — committed in `89a0d7c`, **116 tests green** (unchanged; this step adds no
> test), `cargo clippy --all-targets` clean, and the three touched files are rustfmt-clean. The
> pre-existing drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is still
> untouched.
>
> **No test was owed and none was written.** The step planned none, and `lbb:commit` found nothing
> to finish: the diff adds one closure and deletes state, so there is no new logic to pin, and F's
> only mechanical claim — that `open` is an optional prop — is proved by the crate compiling
> without it. Freelancing a test here would have asserted something the step does not claim.
>
> **The `dx serve` eyeball was run and confirmed by hand.** Arrows pressed in the settings panel's
> `<select>` move the selection without turning the page behind it, and the settings panel still
> opens and closes from its own trigger with the controlled-open triple deleted — which is the
> visible half of F. 7c's outstanding icon eyeball is therefore also closed: the ✕, gear and list
> glyphs render at unchanged weight.
>
> **What this step could not verify, recorded so the next reader does not assume it was.** The
> mutation described in the check section — commenting out the guard and watching the page turn
> under both open panels — is the only way this step's assertion can be observed red, and it was
> not run. The guard is confirmed working, not confirmed *necessary*, and the two are different
> claims.

---

## Step 7e — the frame swallows the outside click

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**Not a triage item.** The nine carried items were all quality; this one is a *defect* in what
Steps 5 and 6 shipped, found by using the panel: open the contents list, click the book to
dismiss it, and nothing happens. Click the header, the nav row, or the margins and it closes
normally. A bug in the panel goes before the polish on the panel, so it took the free letter and
7e/7f/7g became 7f/7g/7h.

### The crux

**The dismissal is not broken — it is looking at the wrong document.** Everything about the
popover works; the click never arrives.

`use_outside_dismiss` (`primitives/src/lib.rs:212-235`) is one `document::eval` that registers
two capture-phase listeners on the **host** page and dismisses when the target is not inside the
popover root:

```js
const f = e => {
  const root = document.getElementById(id);
  if (root && !root.contains(e.target)) dioxus.send(true);
};
document.addEventListener('pointerdown', f, true);
document.addEventListener('focusin', f, true);
```

The book is not in that document. It is in `#reader-frame`, a separate browsing context with its
own `document`, and **DOM events do not cross the frame boundary** — a `pointerdown` in the frame
propagates to *its* document and stops. `e.target` in the host can never be a node inside the
frame, so the host listener is not merely failing the `contains` test, it is never called at all.
Which makes the symptom exactly inverted from how it reads: dismissal works everywhere *except*
the one surface that fills most of the window.

`focusin` is the interesting half. In principle focus moving into a frame fires focus events on
the `iframe` **element** in the parent, which would satisfy the check — the observed behaviour
says it does not reach us here, and a fix that depends on which of two documents a webview
decides to fire focus in is a fix that will rot. The press is the event the primitive actually
wants; the frame can just say when one happened.

**So this is the same shape as every other thing the frame needs to tell the host** —
`ook-link`, `ook-key`, `ook-scroll`, `ook-position`. The frame posts, the host acts. What makes
this one different is where it *ends*: every other message is forwarded into Rust as a
`BridgeMsg` because Rust owns the state it changes. This one changes no Rust state — after 7d
the settings popover has no `open` signal at all, the primitive owns it — so the host's job is
to **re-enter the listener that already exists** rather than to route around it.

### The check — `#[test]`, then `dx serve`

The testable claim is the one the compiler cannot see: two JS files agreeing on a message name,
the repo's standing hazard (`ook-key`, `ook-reflow`, `ook-set-theme` all have this test).

```rust
#[test]
fn a_pointerdown_inside_the_frame_survives_the_hop_back_to_the_host() {
    assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-pointerdown"));
    assert!(BRIDGE_JS.contains("ook-pointerdown"));

    assert!(BRIDGE_JS.contains(FRAME_ID));
}
```

**Watched red** before either half existed:

```
assertion failed: crate::web::assets::INJECTED_ASSETS.contains("ook-pointerdown")
  at src/ui/reader.rs:464:9
```

The third assertion is a *second* name across a gap, and it is new in this step: the host has to
find the frame element to replay the press on, and `"reader-frame"` was a bare literal in two
places in `reader.rs`. It became `const FRAME_ID` so the test can pin the JS to it — rename the
const and this fails, which is the only way that lookup can be caught before it silently returns
`null`.

**What no test can hold**, and what `dx serve` is for:

1. Open the contents panel, click anywhere on the book text. It closes. Same for the settings
   panel.
2. Click a **link** in the text with the panel open — it should both dismiss *and* follow the
   link; the two listeners are independent.
3. **The mutation.** Comment out the `pointerdown` branch in `ook-events-listener.js` and repeat
   (1): the panel stays open over the book. That is the bug, reproduced on demand.
4. Nothing regressed outside the frame: header, nav row and margin clicks still dismiss, and
   Escape still closes.

### The code

`src/web/assets/pointer-listener.js` — new, injected into the frame beside `key-listener.js`:

```js
document.addEventListener("pointerdown", function () {
  window.parent.postMessage({ kind: "ook-pointerdown" }, "*");
});
```

`src/web/assets/ook-events-listener.js` — the host replays it on the frame element:

```js
  if (e.data.kind === "ook-pointerdown") {
    const frame = document.getElementById("reader-frame");
    if (frame) {
      frame.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    }
  }
```

`src/ui/reader.rs` — `const FRAME_ID`, used by the `iframe`'s `id`, by the existing set-page
script, and by the test.

Net: one new file, **13 insertions, 4 deletions**.

### Why it works

**A synthetic event runs the real propagation path.** `dispatchEvent` is not a shortcut that
calls listeners on the target — it computes the full path from `window` down to the target and
back, and every listener on it fires, indistinguishably from a user press. The primitive's
listener sits on `document` with `capture: true`, and `document` is an ancestor of the `iframe`
element, so it is on that path.

**The target is what carries the meaning.** All the primitive asks is
`!root.contains(e.target)`. Dispatching on the `iframe` element makes the answer *true* for
every open popover at once — one event dismisses both panels, and no popover has to know the
frame exists. It is also the honest target: the press really did land on that element as far as
the host document is concerned.

**`bubbles: true` is belt-and-braces, not the mechanism.** Capture-phase listeners fire on
ancestors regardless of whether an event bubbles (this is why `focusin`/`focus` work), so the
`capture: true` registration above would see it either way. It is set because a real
`pointerdown` bubbles, and a synthetic one that does not would quietly skip any bubble-phase
listener added later.

**Why the guard.** `use_bridge`'s `message` listener is registered per mount and is never
removed, so a message can arrive after the reader unmounts and the frame is gone.
`null.dispatchEvent` would throw inside a listener in a webview with no visible console — a
silent failure in the one place this app has no eyes.

**Why it does not fight the `<a href>` bridge.** `link-bridge.js` listens for `click`;
`pointerdown` fires first and on a different event type, so a link click posts both messages.
The dismissal and the navigation are independent and both are wanted.

**Why the frame's listener is unconditional.** It posts on every press whether or not a popover
is open. Telling the frame about popover state would mean a second message kind, a subscription,
and a state machine to keep in sync — all to save one `postMessage` per click, which is already
what `key-listener.js` and `link-bridge.js` cost per key and per click.

### The forks

- **Where to dispatch: the `iframe` element vs `document.body`.** `body` needs no id and would
  pass the `contains` check just as well, but it is a lie about where the press landed, and an
  event dispatched on `body` never traverses `.reader-root` or `#main` — so Dioxus's own
  delegated listeners (which live on `#main`) could never see it. Targeting the frame keeps the
  path truthful for whatever listens later. The price is the `FRAME_ID` coupling, which the test
  now pins.
- **Where the host branch lives: `ook-events-listener.js` vs its own asset.** Every other branch
  in that file ends in `dioxus.send`; this one ends in JS. Splitting it into a `frame-dismiss.js`
  evaluated from `Reader` would keep that file's shape pure, at the cost of a second `message`
  listener on the host, a second `document::eval`, and a `use_hook` to stop it re-registering on
  every render. One branch in the listener that already exists is the smaller thing. The file's
  job is "handle messages from the frame" — that most of them go to Rust is a fact about the
  messages, not a contract.
- **Listening from the host instead of injecting.** The host could reach into
  `frame.contentDocument` and add the listener itself, touching no injected asset. It would have
  to re-register on every chapter load (each chapter is a fresh document from a fresh blob URL)
  and depends on same-origin access to that blob from the host. The established pattern in this
  app is inject-and-post; this follows it.
- **Fixing it in `dioxus-primitives` instead.** `use_outside_dismiss` knowing about iframes is an
  app concern in a library — the same argument 7d made for putting the keyboard guard in the
  wrapper rather than the primitive.

### What to look at hardest

1. **`FRAME_ID` is a widening of the step.** It rewrites two existing lines that were not broken,
   including the `format!` in the set-page effect (which also picked up inline captures for
   `page_number`). The defence is that this step is the one that creates the cross-file
   dependency on that id, so pinning it belongs here — but "the fix needed a const" is exactly
   how scope creep argues for itself, and the assertion could equally have been dropped.
2. **The `focusin` half is left alone.** If a webview *does* fire focus into the host on a frame
   click, the popover now has two independent reasons to dismiss. That is harmless (the second
   `set_open(false)` is a no-op) but it means the fix is not the *only* thing making this work on
   every platform, and the eyeball on one platform cannot tell them apart.
3. **Text selection now dismisses.** Dragging to select in the book closes an open panel, because
   a drag starts with a `pointerdown`. That matches how the host side already behaves — selecting
   host text dismisses too — but it is a behaviour choice, not a neutral one.

### Scope note

**This is dismissal only.** It does not give the frame any other host-level input; a click in the
frame still does not, for instance, blur a focused control in the header.

**The letters after this one are unchanged in content:** 7f is `ul`/`li` list semantics, 7g is
`toc::label_for_spine`, 7h is scroll-the-current-row-into-view.

**Still left alone, carried from 7d:** the settings panel's styling-only wrapper `div` and its
inert `gap` prop, the hand-rolled class merge in `PopoverContent`, the entry `onclick`'s
`stop_propagation` in `toc.rs`, `PopoverRoot`'s unforwarded `props.id`, and the reader's
hand-rolled `button { class: "icon-button" }`. The `merge_attributes` hazard 7d recorded is also
still open.

### What the cleanup pass changed

**One edit, and it was to my own test.** The first draft carried two explanatory comments in
`a_pointerdown_inside_the_frame_survives_the_hop_back_to_the_host`, which `CLAUDE.md` forbids in
agent-written code — the reasoning belongs here and in the handoff, and the comments in the
neighbouring tests are the learner's own. They were removed; the assertions are unchanged.

Reuse, efficiency and altitude produced no edits. Reuse: nothing in the repo already forwards a
frame event to the host DOM, and `link-bridge.js` is a different event for a different purpose.
Efficiency: one `postMessage` and one event construction per press, against a change of state a
human just asked for. Altitude: the two candidates — a separate host asset, and patching the
primitive — are both recorded as forks above rather than taken.

> **Status:** done — committed in `deecc44`, **117 tests green** (116 before), `cargo clippy
> --all-targets` clean, and `reader.rs` is rustfmt-clean. The pre-existing drift in `epub.rs`,
> `web/assets.rs` and `components/popover/mod.rs` is still untouched.
>
> **All three assertions are proved live, by two different instruments.** The first was watched
> red before either half of the hop existed. That red *shadowed* the other two — `assert!` short-
> circuits the test — so at commit time each was inverted in turn and run: misspelling the message
> name fails, and asking the bridge for `"frame-reader"` instead of `FRAME_ID` fails. The file was
> then restored and confirmed byte-identical to its pre-mutation copy with `diff`. The mutation
> was always applied to the *test*, never to the implementation.
>
> **What that last mutation does and does not prove.** Inverting the literal shows the assertion
> runs and that the bridge contains exactly one spelling of the id. It does not directly
> demonstrate the pin — that renaming `FRAME_ID` turns the test red — because that would mean
> editing the implementation. The pin follows from the assertion's *form* rather than from an
> observation: the const's value is the needle, so any new value that the JS does not also carry
> fails.
>
> **No test was owed.** The step planned one and the working tree had it; nothing was written at
> commit time. The learner's own review was the only change between the handoff and the commit,
> and it left the tree byte-identical.
>
> **The `dx serve` gate was confirmed by hand** — the contents panel now dismisses on a press in
> the book text, which is the whole point of the step and the half no test in this crate can hold.
>
> **Not verified, recorded so the next reader does not assume it was:** the mutation described in
> the check section — commenting out the host branch and watching the panel stay open — was not
> run, so the dismissal is confirmed working rather than confirmed *caused* by this diff. Nor were
> the two side-claims eyeballed: that a link click still both dismisses and navigates, and that
> dragging to select text in the book now closes an open panel.

---

## Step 7f — `ul`/`li` list semantics for the contents panel

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Triage item **E**, the one the 7d split left behind: *"the contents list is a `nav` full of bare
`button`s. A screen reader announces eighteen buttons with no sense that they are one list, and
no count."* The smallest item left on the list, and the last one that is purely markup.

### The crux

**A list is a thing you can be lost in, and the browser will only say so if you say `ul`.**

Visually the panel is obviously a list: eighteen rows, stacked, indented by depth. None of that
reaches the accessibility tree. `nav > button × 18` produces eighteen sibling controls; a screen
reader user arrowing through them hears "Preface, button", "Chapter I, button" with nothing that
says how many there are, which one of how many this is, or that they form a single structure.
`ul > li > button` produces "list, 18 items … Preface, button, 1 of 18". The count and the
position are computed by the browser from the markup — there is nothing to maintain, and nothing
to keep in sync with `entries.len()`.

What makes it worth a step rather than a two-character edit is that **`ul`/`li` is not free
layout-wise**. The old markup worked because the eighteen `button`s were themselves the flex
items of a `flex-direction: column` container, so they stretched to full width and the hover
highlight covered the whole row. Insert an `li` between them and the button is no longer the flex
item: it is an `inline-block` inside a block, shrink-to-fit, and the highlight collapses to the
width of the label. The fix is not to re-stretch it but to notice that **the flex column was only
ever simulating block flow**, and block flow is what a list already is.

### The check — `dx serve` + `cargo clippy`

**No red to watch.** The claim is about rendered markup and the accessibility tree, and nothing
inside this crate can see either — the same wall 7c and 7d hit. The gate is a clean build plus
eyes, and the specific things to look at are:

- **The rows still look identical to before.** Same indentation per depth, same padding, and the
  hover highlight still spans the **full width** of the panel, not just the label. This is the
  half the `li` could plausibly have broken.
- **No bullets.** A `ul` renders `::marker` discs by default; if a dot appears to the left of
  every chapter, `list-style: none` did not take.
- **No new gap at the top and bottom of the list.** The UA gives `ul` a `1em` block margin that
  would show up inside the popover's own `1rem` padding.
- **Scrolling still works** — the fixture's ToC is longer than the panel, so the list must still
  scroll inside its `80vh` cap rather than growing the popover.
- **The current chapter is still bold** (`aria-current="page"`), and clicking a row still jumps
  and closes the panel.

If VoiceOver is at hand (`Cmd-F5`), the actual claim of the step is audible: `VO-Right` through
the panel should now announce a list and an *N of 18* on every row.

### The code

`src/ui/toc.rs` — three new levels of nesting, no new logic:

```rust
nav {
    aria_label: "Table of contents",
    ul {
        class: "{Styles::contents_popover__list}",
        for (index, entry) in entries.iter().enumerate() {
            li {
                button { /* unchanged */ }
            }
        }
    }
}
```

`src/ui/toc.css`:

```css
.contents-popover__list {
  max-height: 80vh;
  margin: 0;
  padding: 1rem;
  list-style: none;
  overflow-y: auto;
}

.contents-popover__entry {
  display: block;
  box-sizing: border-box;
  width: 100%;
  /* …unchanged… */
}
```

### Why it works

**`ul` is the count.** `aria-setsize` and `aria-posinset` exist for the cases where you *cannot*
use a list — a virtualized window, a tree assembled from `div`s. Here the whole list is in the
DOM, so the browser derives both, and they stay right when the book changes without a line of
Rust knowing about it.

**`nav` keeps the landmark and finally gets a name.** The element was already there and already
correct — a table of contents is the canonical `<nav>` — but an unnamed landmark is a region a
screen reader announces as "navigation" and nothing else. `aria_label` costs one attribute and
makes the landmark list legible. Note the Dioxus spelling: attributes with hyphens are written
with underscores in `rsx!` (`aria_label`, `aria_current`), and emitted with hyphens.

**The three CSS declarations are each undoing one UA default**, and it is worth naming which:

- `list-style: none` — the `::marker` disc.
- `margin: 0` — `ul`'s `1em` block margin.
- The UA's `padding-inline-start: 40px` needed nothing, because the rule already sets `padding`
  outright. It is only invisible because the shorthand was already there.

**Dropping `display: flex` is the part to think about.** With the `li` in place, the flex
container's items are the list items, which are block-level and full-width on their own — the
column flex is doing nothing that normal block flow does not already do. Two declarations that no
longer earn their place, and removing them is not merely tidy: a `flex-direction: column`
container with a `max-height` gives every item `flex-shrink: 1`, which is the standard way a
scrolling flex list ends up squashing its rows instead of scrolling. Block flow has no such edge.

**`box-sizing: border-box; width: 100%` on the button replaces the stretch that flex was
providing** — and the first draft of this step got that wrong in a way worth keeping in the log,
because the wrong version is the one that reads as obviously correct.

The draft wrote `display: block` alone, on the reasoning that a block-level element's `width:
auto` fills its containing block. That rule does not hold for a `<button>`. Form controls are
sized with **`fit-content`** — `width: auto` on a button means "as wide as your label", `display:
block` or not — so the row hugged its text and the hover highlight stopped after the last letter.
The gate caught it; see the status marker below.

Two consequences worth separating, because they are easy to conflate:

- **The regression came from inserting the `li`, not from dropping the flex.** Before this step
  the button *was* the flex item and `align-items: stretch` gave it full width. Once an `li` sits
  between the list and the button, the `li` is the flex item and the button is just a control
  inside it. Restoring `display: flex; flex-direction: column;` would not have fixed the highlight
  — the fix has to be on the button whatever the list does.
- **`box-sizing: border-box` is not decoration here.** This repo sets no global `border-box`, so
  `width: 100%` on its own would size the *content* box to the full row and push the horizontal
  padding beyond it — and that padding grows with the depth indent, so deeper entries would
  overflow further than shallow ones.

### The forks

**`nav > ul` rather than replacing the `nav` with the `ul`.** The other option is to drop `nav`
and let the `ul` carry the class and the label. It is one element less, and inside a floating
popover a landmark buys less than it does in page chrome. I kept it because removing semantics to
add semantics is a strange trade, and the label makes it a net gain — but if you find the extra
wrapper annoying, deleting the `nav` and moving `aria_label` onto the `ul` is a safe edit.

**A flat `ul` rather than nested `ul`s, or an ARIA `tree`.** The ToC *is* a tree, and the truly
faithful markup is either nested lists or `role="tree"` + `role="treeitem"` + `aria-level`. Both
were rejected, for different reasons. Nested lists would mean un-flattening the `Vec<TocEntry>`
back into a tree at render time, against the phase's stated design decision to flatten once at the
boundary. `role="tree"` is worse than useless done halfway: it is a *widget* role, and a screen
reader that hears "tree" switches to application mode and expects arrow-key navigation, expand and
collapse, and roving `tabindex`. Announcing a contract we do not implement is a regression from
plain buttons. `aria-level` alone on the `li`s is the cheap middle ground and could be added later
if depth turns out to be inaudible in practice.

**No `key` on the `li`.** The `for` loop has no key, matching what the `button` had before it;
`library.rs` keys its book list because that list is re-ordered by last-opened. A book's ToC is
built once per open and never mutates, so the index-position identity Dioxus falls back to is
exactly right. Worth revisiting only if the panel ever filters or searches.

### What to look at hardest

1. **The row's width.** ~~The `display: flex` removal — the only change in the diff that can
   produce a visual regression, and the one I could not test.~~ It did regress, and the handoff
   pointed at the wrong declaration: the cause was the `li` stealing the flex-item role from the
   button, not the flex column coming off the list. Corrected above and in the status marker; the
   three declarations on the button now carry the row's width explicitly.
2. **Whether the `nav` should have survived at all.** See the fork above — this is a taste call
   about how much structure a popover deserves, and you are better placed to make it than I am.
3. **`aria_label: "Table of contents"` is a user-visible string sitting in the markup.** Nothing
   in this app is localized yet and there is no string table to put it in, so it is hardcoded like
   every other label in `src/ui`. Consistent, but it is the first one that exists *only* for
   assistive tech, which makes it the easiest one to forget when a string table does arrive.

### Scope note

Markup and CSS only — no Rust logic changed, no test added, and the panel behaves identically for
a sighted mouse user. It does **not** add `aria-level` for depth, does not make the list a
keyboard-navigable tree widget, and does not touch the `entry onclick`'s `stop_propagation`, which
7d recorded and left alone. Two triage items remain after this one: **7g** (`toc::label_for_spine`)
and **7h** (scroll the current row into view), and 7h is still the one that changes behaviour and
therefore still wants its own commit or a push to the next phase.

### What the cleanup pass changed

**One edit: the flex column came out of `.contents-popover__list`.** The first draft kept
`display: flex; flex-direction: column;` untouched, on the theory that the new `li` layer only
needed `display: block` on the button to fill the row. That is the copy-paste-with-variation
smell — carrying a declaration because it was there rather than because it does something — and
the `flex-shrink` reasoning is in *Why it works* above. The checks were re-run after it:
**117 tests, `cargo clippy --all-targets` clean.**

**The pass was right about the flex and wrong about what replaced it.** Removing the column was
correct and is kept; `display: block` alone was never going to give a `<button>` full width, in
either version of the rule. That is the honest shape of this one: the cleanup did not cause the
regression and did not catch it either, because both drafts shared the same wrong premise about
how form controls size.

Reuse produced no edit: `library.rs` is the repo's other `ul`/`li`, and it shares no styling with
this one (a responsive grid of covers against an indented scroll list), so there is nothing to
hoist. Efficiency: three DOM nodes per entry instead of one, eighteen entries, built once per
panel open. Altitude: the item is markup at the leaf and there is no deeper mechanism to
generalize — the one candidate, a shared "list of buttons in a popover" component, has exactly one
caller.

**Note on how this pass was run.** The `simplify` skill fans out four review agents; this
session's instructions forbid spawning subagents unless the user asks for them, and the diff is
ten lines. The four angles were applied by hand instead, in the same order and with the same
brief.

> **Status:** done — committed in `b92816f`, **117 tests green** (unchanged), `cargo clippy
> --all-targets` clean, and `src/ui/toc.rs` is rustfmt-clean. The pre-existing drift in `epub.rs`,
> `web/assets.rs` and `components/popover/mod.rs` is still untouched.
>
> **No test was owed and none was written.** The step planned none, and it added no logic to pin
> — the diff is three levels of nesting and five CSS declarations. The one test this file already
> carries, `the_depth_variable_is_spelled_the_same_on_both_sides_of_the_css_gap`, was run on its
> own to confirm the 7b pin survived the rule being edited around it, and it passed. Nothing in
> this crate can see rendered markup or the accessibility tree, so the step's actual claim — that
> the panel now announces as a list of eighteen — has no automated instrument at all and is
> recorded as eyeballed, not asserted.
>
> **The eyeball gate failed the first time, which is the useful part of this entry.** The handoff
> shipped `display: block` on the button and claimed it would fill the row. It did not: the hover
> highlight hugged each label, and the screenshot came back before the commit. A `<button>` is a
> form control and sizes to `fit-content` whatever its `display` — the block-level "fill your
> containing block" rule does not apply to it. `box-sizing: border-box; width: 100%` was added and
> the gate re-run.
>
> **The handoff also pointed at the wrong suspect**, and that is worth more than the fix. It
> flagged the `display: flex` removal as the risky change; the cause was the `li` taking the
> flex-item role away from the button, so restoring the flex would not have helped. *Why it works*
> and *What to look at hardest* were both corrected in place rather than left standing, since this
> doc is read later as a record of what was believed at the time.
>
> **Not verified, recorded so the next reader does not assume it was:** the VoiceOver pass. The
> list, its item count and the *N of 18* per row are what the step is *for*, and they were argued
> from the markup rather than heard. The visual checks — full-width highlight at every depth, no
> markers, no new block margin, the `80vh` scroll cap, the bold `aria-current` row, and click
> still jumping and closing — are the ones that were actually run.

---

## Step 7g — `toc::label_for_spine`

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Triage item **H**, carried since Step 4. `entry_index_for_spine` answers "which entry?" with an
*index*, and three of its four callers immediately spend that index on the same lookup —
`entries[index].label` — spelled identically each time. The fourth caller, the contents panel, is
the only one that genuinely wants the number.

### The crux

**The function returns the wrong thing for most of its callers, and "wrong" here means one
indirection too low.**

`entry_index_for_spine` is doing the hard part: the first-match-then-fall-back-to-preceding rule
that Step 3 had to choose and defend. What it hands back is a position into a slice, which is a
*handle* to the answer rather than the answer. Every caller that only wanted the name then
performs the same two-step — index, then index again — and the index leaks out into code that has
no other use for it.

The tell is that the two test closures in `toc.rs` are **byte-identical**, and neither is testing
the indexing; they exist because the assertions want to read `Some("I. A SCANDAL IN BOHEMIA")`
rather than `Some(2)`. When a test has to write a helper to convert the function's output into the
thing it actually means, the function is returning at the wrong altitude.

The fix is not to change `entry_index_for_spine` — the panel needs it as it is, to compare against
its loop counter for `aria-current`. It is to put the common half **above** it, so the index stays
available for the one caller that wants it and stops being the interface for the three that do
not.

### The check — `cargo test`

**No new test.** The two closures the step deletes *are* the check: rewriting them to call the new
function makes the existing assertions exercise it directly, which is the whole point of the item.

```rust
let entries = toc_entries(&epub, &docs);
let label = |spine_index| label_for_spine(&entries, spine_index);

assert_eq!(label(1), Some("The Adventures of Sherlock Holmes"));
assert_eq!(label(2), Some("I. A SCANDAL IN BOHEMIA"));
assert_eq!(label(14), Some("THE FULL PROJECT GUTENBERG™ LICENSE"));
assert_eq!(label(0), None);
```

The red is a compile error, the same shape 7a and 7b opened with:

```
error[E0425]: cannot find function `label_for_spine` in this scope
   --> src/toc.rs:108:35
error[E0425]: cannot find function `label_for_spine` in this scope
   --> src/toc.rs:128:35
```

Both call sites, which is worth noticing: the closure was duplicated, so the red is too.

### The code

`src/toc.rs`:

```rust
pub(crate) fn label_for_spine(entries: &[TocEntry], spine_index: usize) -> Option<&str> {
    entry_index_for_spine(entries, spine_index).map(|index| entries[index].label.as_str())
}
```

`src/ui/reader.rs`:

```rust
fn chapter_label(entries: &[TocEntry], chapter: usize, chapter_count: usize) -> String {
    match toc::label_for_spine(entries, chapter) {
        Some(label) => label.to_string(),
        None => format!("Chapter {} of {}", chapter + 1, chapter_count),
    }
}
```

### Why it works

**`Option<&str>` rather than `Option<String>` is the whole reason this is free.** The label lives
in the `Vec<TocEntry>` the caller already owns and is not going anywhere while the borrow is
alive; returning a `&str` hands out a view of it and allocates nothing. The old `chapter_label`
called `.clone()` on the `String` because it *had* an owned `String` in hand and needed one out;
the new one calls `.to_string()` on a `&str` for the same reason. Same allocation either way —
the win is not fewer bytes, it is that the two callers who never wanted a `String` no longer touch
one.

**The elided lifetime is doing real work here.** Written out, the signature is:

```rust
fn label_for_spine<'a>(entries: &'a [TocEntry], spine_index: usize) -> Option<&'a str>
```

Rust's elision rules pick this without being told: one lifetime in the inputs (`spine_index` is
`usize`, no borrow), so the output borrows from it. That is exactly the guarantee the function
needs — the returned `&str` cannot outlive the slice it points into, so a caller cannot drop the
entries and keep the label. Had there been two reference parameters, elision would have refused
and the lifetime would have to be spelled.

**`entries[index]` cannot panic**, and it is worth being explicit about why, because a bare index
in Rust usually deserves suspicion. `index` came from `position`/`rposition` on *this same slice*
one line earlier, so it is in range by construction. `entries.get(index)` would be defensive
against a bug that the type of the composition already rules out.

**`.map` rather than `?`**: the function is one composition, and `Option::map` says "same shape,
different payload" in a single expression. The first draft bound `let index = …?;` and returned
`Some(…)` on the next line; the cleanup pass took the binding out (see below).

### The forks

**`label_for_spine` rather than `entry_for_spine -> Option<&TocEntry>`.** The more general
function would let a caller reach the depth or the target too, and the label version could be
built on it. I took the narrow one because nothing wants the other fields at a spine index —
the panel iterates all entries and the jump goes through `target` on the entry the user clicked,
not the current one. A `&TocEntry` would also not help the one caller that keeps using the index,
since it needs the *number* to compare against its loop counter. If a "you are here" feature later
wants the depth, widening this is a two-line change.

**`entry_index_for_spine` stays `pub(crate)` and stays as it is.** The alternative is to make it
private and have `ui/toc.rs` use something else, but the panel's need is real and specific:
`aria_current: if Some(index) == current`. Two functions at two altitudes with one caller each is
the honest shape; collapsing them would mean the panel recomputing a position it was just handed.

**`match` rather than `map_or_else` in `chapter_label`.** `toc::label_for_spine(entries, chapter)
.map_or_else(|| format!(…), str::to_string)` is one expression and fits on two lines, but it reads
the arms out of order — the fallback first, the hit second — and `page_label` directly below uses
`match`. Consistency with the neighbour won.

### What to look at hardest

1. **Whether `chapter_label` should have moved into `toc.rs` entirely.** It is now a thin
   `match` over a `toc` function, and the "Chapter N of M" fallback is arguably as much a
   labelling rule as the first-match rule is. I left it in `ui/reader.rs` because the fallback
   is a *presentation* string — the only one of the three that mentions a chapter count the
   model does not track — and because item H asked for the indexing dance to go, not for the
   function to move. Reasonable to disagree.
2. **The name.** `label_for_spine` mirrors `entry_index_for_spine`, but neither says *which*
   label when several entries name one document, which is the fixture's whole complication.
   `current_label` or `label_at` are alternatives; none of them say it either.
3. **`.to_string()` on the `&str`.** It is the same allocation the old `.clone()` made, but it
   now sits in the UI rather than beside the lookup, which makes it easier to see and easier to
   ask whether `chapter_label` should return `Cow<str>` instead. I did not, because the fallback
   arm always allocates anyway and a `Cow` that is owned half the time buys nothing.

### Scope note

Three call sites and one new function. It does **not** move `chapter_label`, does not touch
`entry_index_for_spine`'s rule or signature, and does not change what the reader displays — the
same assertions pass before and after, which is the definition of this kind of refactor going
right. **7h** (scroll the current row into view) is the last item in the triage and the only one
that changes behaviour, so it still wants its own commit or a push to the next phase.

### What the cleanup pass changed

**One edit: the intermediate `let` came out of `label_for_spine`.** The first draft was

```rust
let index = entry_index_for_spine(entries, spine_index)?;

Some(entries[index].label.as_str())
```

which binds a name used once on the very next line — the "intermediate `let` that says nothing"
the simplify brief names explicitly. `.map` says the same thing in one expression, and it is also
the exact shape of the two closures the step deletes, which makes the diff read as *moving* code
rather than rewriting it. The checks were re-run after: **117 tests, `cargo clippy --all-targets`
clean.**

Reuse found nothing to hoist — this step *is* the reuse finding, arriving three sittings after it
was filed. Efficiency: identical, one `position` scan and one allocation either way. Altitude: the
one candidate is fork 1 above, `chapter_label` moving into `toc.rs`, recorded rather than taken.

**Note on how this pass was run.** As in 7f, the `simplify` skill's four review agents were not
spawned — this session's instructions forbid subagents unless the user asks — and the four angles
were applied by hand in the same order.

> **Status:** done — committed in `a3c4ad7`, **117 tests green** (unchanged), `cargo clippy
> --all-targets` clean, and both `src/toc.rs` and `src/ui/reader.rs` are rustfmt-clean. The
> pre-existing drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is still
> untouched.
>
> **No test was written, and none was owed.** The step's check was the two existing tests rewritten
> to call the new function, and both were already in the tree at commit time. The count is
> unchanged because a refactor that adds a test is usually a refactor that changed behaviour.
>
> **The red and the mutation prove two different things, and the step needed both.** The red was a
> compile error at both call sites — `cannot find function label_for_spine in this scope` — which
> proves the tests now *route through* the new function, but says nothing about whether their
> assertions still bite. So at commit time each rewritten test was mutated in turn and run:
> `label(2)`'s expected label replaced with a string the fixture does not contain, and
> `label(3)`'s `Some("One")` — the fall-back-to-preceding case — replaced with `Some("Two")`. Both
> failed, and the failure output shows the real value on the left, so the new function is
> genuinely computing both branches of Step 3's rule. `src/toc.rs` was then restored from a copy
> taken before the mutation and confirmed byte-identical with `diff`. The mutation was applied to
> the *tests* only; the implementation was never touched.
>
> **`dx serve` was run and confirmed by hand**, though this step is the one where it proves least:
> the diff is behaviour-preserving by construction, so the app looking unchanged is what success
> and a no-op both look like. Recorded because it was done, not because it carries the step.
>
> **Fork 1 stayed a fork.** `chapter_label` is still in `ui/reader.rs` rather than moved into
> `toc.rs`. It is now a thin `match` over a `toc` function and the case for moving it is real; it
> was left because the "Chapter N of M" fallback is a presentation string mentioning a count the
> model does not track, and because item H asked for the indexing dance to go, not for the function
> to move.

---

## Step 7h — Scroll the current row into view

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Triage item **G**, carried since Step 5 and the last one on the list. The panel knows which row
you are on — `aria-current="page"` has been on it since 7f — and opens at the top of the list
anyway. On the fixture's 18 entries that costs nothing, because nothing scrolls. On a book whose
ToC does not fit in `80vh`, the one row the panel exists to show you is the one you cannot see.

**This is the phase's only behaviour change since Step 6**, which is why the triage put it last
and on its own commit: a review-and-refactor step that also changes what the app does is two steps
pretending to be one.

### The crux

**The row you want to scroll to does not exist until the moment you want to scroll to it.**

The obvious shape — an effect watching `open()` that finds the current row and scrolls it — races
and loses. `PopoverContent` renders nothing at all while closed (`use_animated_open` returns
`false` and the component early-returns an empty `rsx!`), so at the instant `open` flips to `true`
the row is not in the DOM yet. It arrives one render later, after the primitive's own effect has
run. An effect keyed on `open()` would fire into an empty document.

The fix is to stop asking "when should I scroll?" and let the element answer. `onmounted` fires
exactly once per row, at the moment that row enters the DOM — which, because the content unmounts
on close and remounts on open, is exactly once per panel opening. **The lifecycle event the
framework already gives you is the synchronisation you were about to hand-roll.**

`reader.rs` already uses the same hook for the same reason: `onmounted` + `set_focus(true)` on the
reader root, because focus also cannot be set on an element that is not there yet.

**That was the crux the step was planned around, and it turned out to be only half of it.** The
first draft did all of the above and the panel still opened at the top, because *something else was
already scrolling the list* and winning. See "The second crux" below.

### The second crux — the focus trap scrolls too

**`is_modal` defaults to `true`, and a focus trap is a scroll you did not write.**

`PopoverRootProps::is_modal` is `#[props(default = ReadSignal::new(Signal::new(true)))]`. Nothing in
this repo ever set it, so every popover in the app is modal, and `PopoverContent` runs this on open:

```rust
dialog.trap = window.createFocusTrap(dialog);
```

`FocusTrap`'s constructor ends with `this.focusNext()`, which walks a `TreeWalker` to the **first
focusable descendant** and calls `.focus()` on it. That is the first `<button>` in the list — row
one. And `HTMLElement.focus()` scrolls the focused element into view unless you pass
`preventScroll`, which the trap does not.

So the sequence on every open was: our `onmounted` scrolls the current row into view, then the
trap's `use_effect` focuses row one and scrolls the list straight back to the top. The screenshot
that caught this shows the header reading *4.3 Operators* over a panel sitting on *Notes on Usage*
— the label proves `current` was `Some`, so the guard fired and the scroll ran. It was undone, not
skipped.

**The trap's initial focus is a bug in its own right, not just an obstacle to this step.** On a
book with a two-hundred-entry ToC it puts the keyboard user on row one — the least useful row in
the list, and the one place the panel already scrolls to by default. Nothing was gained by it.

The fix is `is_modal: false` on this popover, which stops the trap being created at all. What that
costs is examined under the forks.

### The check — `dx serve`

**No test, and none is possible in this crate.** The step's claim is "a row is visible after the
panel opens", which is layout in a webview; nothing in the process can see a scroll offset. Same
situation as 7c, 7d and 7f, and the count stays at **117**.

**The gate failed the first time**, on a real book rather than the fixture, and that is how the
focus trap was found. The corrected implementation has not been eyeballed yet.

The gate needs setting up, because **the bundled fixture cannot show this working**. Eighteen rows
at ~1.6rem each is roughly 430px against a `max-height: 80vh` that is ~640px in a normal window —
the list never overflows, so opening the panel looks identical before and after this diff. Two ways
to make it real (a real book with a long ToC, like the C++ one that caught the bug, is a third):

1. **Shrink the window** to about a third of the screen's height and open the panel on a late
   chapter. `80vh` shrinks with it, the list starts scrolling, and the current row should be at
   the top of the visible list rather than the list starting at "The Adventures of Sherlock
   Holmes".
2. **Temporarily** set `.contents-popover__list`'s `max-height` to `10rem` in `src/ui/toc.css`,
   check the same thing, and put `80vh` back.

What to look for, in order:

- Open the panel on chapter 1: nothing moves, the list is already at the top.
- Open it on a late chapter: the bold `aria-current` row is visible without scrolling by hand.
- Nothing else moved — the *page behind the panel* must not have scrolled, and the panel must not
  have jumped position.
- Click a row: it still jumps and still closes. Reopen: it lands on the new current row.

Two checks added by the focus-trap fix, since dropping `is_modal` changes the keyboard story:

- **After picking a chapter, the arrow keys still turn pages.** This is the one thing the trap was
  quietly doing for us — `trap.remove()` restored focus to whatever held it before. Without the
  trap nothing ever took focus away, so it should be identical, but it is the regression to watch.
- **Tab from the list button still walks into the list**, and Escape still closes the panel.

### The code

`src/ui/toc.rs`. On the row `button`:

```rust
onmounted: move |e| async move {
    if Some(index) == current {
        if let Err(err) = e.scroll_to(ScrollBehavior::Instant).await {
            eprintln!("ook: the contents panel did not scroll: {err}");
        }
    }
},
```

and on the `PopoverRoot`:

```rust
is_modal: false,
```

### Why it works

**`onmounted` fires per row, per mount — and the panel remounts every time it opens.**
`PopoverContent` is not hidden with CSS while closed; the primitive drops the subtree. So the
eighteen handlers run on every opening, seventeen of them do nothing, and one scrolls. Nothing
needs to know *when* the panel opened.

**Both captured values are `Copy`, which is why there is no `let` block here.** `index` is a
`usize` from `enumerate` and `current` is the `Option<usize>` computed once above the `rsx!`. The
`move` closure copies both, so unlike the `onclick` directly below it — which has to
`let target = entry.target.clone();` first, because `LinkTarget` is owned data borrowed from a
`Vec` that outlives nothing — this handler needs no preamble. The two handlers sitting next to
each other with different shapes is not inconsistency; it is the difference between capturing four
bytes and capturing a heap allocation.

**The inner `async move` takes ownership of `e`.** `scroll_to` returns a future — the desktop
renderer implements it by evaluating `window.interpreter.scrollTo(id, options)` in the webview and
awaiting the answer, so the call cannot be synchronous. `onmounted` accepts a closure returning a
future, and the `move` on the inner block is what lets `e` outlive the closure body. Same idiom as
`reader.rs`'s `set_focus`.

**The `Err` arm is logged rather than discarded, and the first draft got that wrong.** `scroll_to`
returns `MountedResult<()>`, and the draft wrote `_ = e.scroll_to(…).await` on the argument that
neither failure mode — renderer does not support scrolling, node not found — is recoverable or
worth a line in a handler that runs eighteen times per open. Both halves of that were wrong. The
guard means the call happens **once** per open, not eighteen times, so there is no noise to avoid.
And when the panel then failed to scroll, the discarded `Result` was the reason there was nothing
to go on: the interpreter's `scrollTo` returns `false` when it cannot find the node, which arrives
as `Err(OperationFailed)` and looked exactly like the observed silence. Not the actual cause this
time — but it cost a debugging session to rule out, which is the whole argument for the line.
`eprintln!("ook: …")` matches the bridge's existing warning idiom in `reader.rs`.

**`Instant`, not the default `Smooth`.** The panel is fading in over 200ms at the same time. A
smooth scroll would animate the list from the top while the panel animates from transparent, which
reads as two things arriving late; instant means the panel is simply *already* in the right place
when it becomes visible. Opacity does not affect layout, so the scroll and the fade do not fight.

**What the row does *not* control is where in the viewport it lands.** `MountedData::scroll_to`
fills in `ScrollToOptions { behavior, vertical: Start, horizontal: Center }` — and on desktop the
last two are dead. The struct serialises its fields as `vertical`/`horizontal`, the interpreter
hands the JSON straight to `Element.scrollIntoView`, and that API reads `block`/`inline`. Unknown
keys are ignored, so what actually happens is `scrollIntoView({behavior: "instant"})` — DOM
defaults, `block: "start"`, `inline: "nearest"`. **The current row goes to the top of the list.**
Worth knowing before reaching for `scroll_to_with_options` to ask for `Center`: it would compile,
read as a deliberate choice, and change nothing.

**The document does not scroll along with the list.** `scrollIntoView` walks every scrollable
ancestor, and `.dx-popover-content` is `position: fixed` inside a `body` with `margin: 0` around a
`.reader-root` of `height: 100vh` — the document is exactly the viewport, so there is no second
scroll to perform. That is load-bearing and invisible; the third eyeball check above exists to
catch it if a future layout change breaks it.

### The forks

**`onmounted` on all eighteen rows with a guard, rather than a handler on one row.** The
alternative is `if is_current { button { onmounted: … } } else { button { … } }`, which duplicates
the entire row markup to vary one attribute — clearly worse. rsx has no way to attach an event
handler conditionally, and no way to bind `let is_current = Some(index) == current;` inside the
`for` body either (I tried: `error: expected identifier` at the `=`). So the predicate is spelled
twice in the same element, once for `aria-current` and once for the scroll. It is three tokens and
they are four lines apart; splitting the row into its own component to hoist it would cost more
than it saves. **This is the one bit of the diff I would most expect to be overruled.**

The cost of the chosen shape is seventeen no-op async handlers per panel open. Real, but each is a
`Copy` closure and a comparison, and it is bounded by the number of rows the user is about to look
at.

**No JS asset.** The repo's other DOM manipulations go through `src/web/assets/*.js` and
`document::eval`, but those all talk to the *sandboxed iframe*, which Dioxus does not own. This row
is a node in Dioxus's own tree with an `ElementId` already assigned — going out to
`document.querySelector('[aria-current="page"]')` would re-derive by CSS selector something the
framework hands over for free, and would reintroduce the timing problem the crux describes.

**Scroll on open only, not on chapter change.** If the chapter changes while the panel is open the
rows do not remount and nothing re-scrolls. Reaching that state needs a page turn from inside the
frame while the panel is over it, which 7e made harder by dismissing the panel on a press in the
frame. Not worth an effect and a stored `MountedData` to chase.

**`is_modal: false` rather than out-racing the trap.** Three other ways to get the scroll to stick
were considered and rejected:

- **Scroll after the trap.** Both the trap's `eval` and our query are async scripts queued on the
  webview, and their order is an implementation detail of two effects in two components. Winning a
  race by arranging to be scheduled second is not a fix; it is a fix-shaped coincidence.
- **`tabindex="-1"` on every row but the current one.** This is genuinely clever: the trap's
  `focusable()` check rejects `tabIndex < 0`, so its `TreeWalker` would skip straight to the
  current row and focus *it*, doing our scroll for us with no `scroll_to` at all. It also makes
  every other row unreachable by Tab, which is a real accessibility regression traded for a
  cosmetic win. The standard answer here is a roving tabindex with arrow-key navigation, and that
  is a feature, not a line.
- **Keeping the trap and focusing the current row ourselves.** Same race, plus the trap's
  `TreeWalker` would then be pointing at the wrong node, so the first Tab would jump to row two.

**What dropping the trap costs, honestly.** Focus no longer moves into the panel on open — a
keyboard user Tabs into it instead — and Tab can walk back out while the panel is open. What it
does *not* cost is focus restoration, which is the part that looked expensive: `trap.remove()`
restores focus to `document.activeElement` as it was when the trap was built, which is the trigger
button, and without a trap focus never leaves the trigger button in the first place. The end state
is identical, which is why the arrow-keys-still-work check above should pass. `aria-modal="true"`
also comes off the `role="dialog"`, which is more honest for a panel that never blocked the rest of
the page.

**Only this popover.** `is_modal` is a prop on `PopoverRoot`, so the settings panel keeps its trap.
Whether *it* should is a separate question and a separate diff — it is a short list of controls
where landing on the first one is a reasonable thing to do.

### What to look at hardest

1. **The duplicated `Some(index) == current`.** See the fork above. If it bothers you, the honest
   fix is a `TocRow` component taking `is_current: bool` — which is a real refactor with a real
   argument for it, and a bigger diff than the feature.
2. **`Instant` vs `Smooth`.** I argued the panel should appear already-scrolled, but a short smooth
   scroll is also a way of *showing* the user that the list moved and where. This is a taste call
   about a thing I cannot see; you can.
3. **`block: "start"` putting the current chapter at the very top of the list.** Every entry before
   it is then hidden, which on a deep ToC hides the parent heading the current row sits under. The
   DOM would do `block: "center"` happily; Dioxus's typed API cannot currently ask for it on
   desktop (see *Why it works*), so getting it would mean a raw `eval` or an upstream fix. Worth
   deciding whether `start` is actually what you want before anyone bothers.
4. **`is_modal: false`.** The largest behavioural claim in the diff and the one made under
   diagnosis rather than under a passing test. The mechanism is read off the primitive's source and
   is not in doubt; what is a judgement call is whether a contents panel should trap focus at all.
   If you decide it should, this step needs a different answer, and the `tabindex` one under the
   forks is where I would start.

### Scope note

One file, no test, no CSS. It does **not** scroll on chapter change while open, does not add a
"you are here" affordance beyond the existing bold row, does not touch fragment-level precision,
and does not give the list arrow-key navigation — which is what a roving tabindex would need and
what would let the current row be the only tab stop. It also leaves the settings popover modal.
**This closes the 7-series triage.** With it, every carried item from the Step 5 and Step 6 reviews
is landed, and Phase 8 has nothing left but the phase-closing status marker.

### What the cleanup pass changed

**Nothing.** The pass ran over a five-line diff and produced one candidate — binding
`Some(index) == current` once — which does not compile inside an rsx `for` body. That was tried,
not assumed: the edit was applied, `cargo clippy` returned `error: expected identifier` pointing at
the `=`, and the file was restored from a copy taken beforehand. The checks were re-run after the
restore: **117 tests, `cargo clippy --all-targets` clean**, and `src/ui/toc.rs` is rustfmt-clean.
The pre-existing rustfmt drift in `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is
still untouched.

Reuse: the `onmounted` + await idiom is shared with `reader.rs`'s `set_focus`, but the two calls
have no common body to hoist. Efficiency: seventeen no-op handlers, recorded as the fork's cost
rather than optimised away. Altitude: the `TocRow` component is the one candidate, and it is a
refactor larger than the feature it would tidy — fork 1 above.

**Note on how this pass was run.** As in 7f and 7g, the `simplify` skill's four review agents were
not spawned — this session's instructions forbid subagents unless the user asks — and the four
angles were applied by hand in the same order.

### What the failed eyeball changed

The handoff shipped a five-line diff and claimed it scrolled. It did not, and the screenshot that
proved it also carried the evidence for why: the header read *4.3 Operators* while the panel sat on
*Notes on Usage*, so `current` was `Some` and the guard had fired. A scroll that runs and leaves no
trace is a scroll that something undid.

Three things came out of the diagnosis and all three are in the diff above:

1. **`is_modal: false`** — the actual fix. The focus trap the popover creates by default focuses
   row one on open, and focusing scrolls.
2. **The `Err` arm is logged** rather than `_ =`-ed away. The first handoff argued for discarding
   it and the argument was wrong on both counts; see *Why it works*.
3. **The "why it works" section grew a second crux.** The original one — the row does not exist
   until the panel opens — is still true and still load-bearing. It was just not the only thing
   standing between the step and a working scroll, and a doc that records only the first half reads
   later as if the step were easier than it was.

**Verified at commit time.** The corrected version went through `dx serve` and the panel now opens
on the current chapter. The diagnosis was read off `dioxus-primitives`' `focus-trap.ts` and
`popover.rs` before it was observed, which is worth recording: the fix was argued from the
dependency's source rather than found by trial, and the eyeball confirmed it rather than discovered
it.

> **Status:** done — committed in `75a5cde`, **117 tests green** (unchanged), `cargo clippy
> --all-targets` clean, and `src/ui/toc.rs` is rustfmt-clean. The pre-existing rustfmt drift in
> `epub.rs`, `web/assets.rs` and `components/popover/mod.rs` is still untouched — a stray
> `cargo fmt` picked all three up during the step and was reverted, because reformatting files
> this step never edited is not this step's business.
>
> **No test was written and none was owed.** The step planned none, and neither of its two claims
> has an instrument in this crate: where a list is scrolled and whether a focus trap exists are
> both webview state, and nothing in the process can see either. The count staying at 117 is the
> honest record of that. The one test living in the file this step edited — 7b's `--toc-depth`
> pin — was run on its own to confirm it survived, and passed.
>
> **The gate failed once, and the failure is the most useful thing in this entry.** The first draft
> was five lines and shipped with a handoff claiming it worked. It did not, and the screenshot that
> proved it also carried the evidence for why: the header read *4.3 Operators* over a panel sitting
> on *Notes on Usage*, so `current` was `Some` and the guard had fired. **A scroll that runs and
> leaves no trace is a scroll something undid** — which is a different search than "why did my code
> not run", and the screenshot is what made the difference visible.
>
> **The first handoff also argued for discarding the `Result`**, on the grounds that the failure was
> unrecoverable and the handler ran eighteen times per open. Both halves were wrong: the guard means
> once, not eighteen, and the discarded `Err` was the one instrument that could have distinguished
> "the node was not found" from "something scrolled it back". It is logged now. That reasoning was
> corrected in place in *Why it works* rather than left standing, since this doc is read later as a
> record of what was believed at the time.
>
> **What was not verified, recorded so the next reader does not assume it was:** the keyboard story
> after dropping the trap. The arrow-keys-still-turn-pages check is argued from where focus sits
> (never leaving the trigger button, so there is nothing for `trap.remove()` to restore) rather than
> pressed. Tab-into-the-list and Escape-to-close are argued the same way. The visual check — the
> panel opening on the current chapter on a book whose ToC overflows — is the one that was actually
> run.

---

## Phase closed

Every planned step is landed and the phase doc's checklist is fully ticked. The last code
commit is `75a5cde` (7h); this log closed at `1f9ace0`. Final state: **117 tests passing, 0
failed**, `cargo clippy --all-targets` clean, working tree clean.

### The tally

| | |
|---|---|
| steps planned | 7 |
| steps landed | 15 — Step 7 split into 7a…7h, plus `c5e6d37` ahead of the lettering |
| tests at the phase's start | 108 |
| tests at its close | 117 |
| steps that added no test | 6 — 5, 7c, 7d, 7f, 7g, 7h |
| eyeball gates that failed first | 2 — 7f (`display: block` does not widen a `button`) and 7h (the focus trap scrolls) |
| carried review items dropped | 0 |

**Nine tests for seven steps is the phase's own thesis showing up in the numbers.** Flattening
the ToC at the boundary turned entry→spine, spine→entry, and entry→`LinkTarget` into pure
functions over a `Vec`, and every one of them is pinned by `cargo test`. The six steps that
added none are all on the other side of that line — rendered markup, the accessibility tree,
webview scroll position, whether a focus trap exists — where nothing in this crate can see the
thing being claimed. That split was predicted in the crux and it held.

**The two failed eyeballs are the more useful entry.** Both were steps whose handoff claimed a
working result before anyone had looked, and in both cases the screenshot carried the diagnosis,
not just the verdict. They are the argument for why a visual gate is a gate and not a formality.

### One gate never explicitly closed: 7c

7c's entry still reads *"the eyeball is still outstanding"*, and nothing since has recorded
running it. What is true is weaker than a pass and is worth stating as such: **7f and 7h both
ran `dx serve` afterwards**, on screens carrying all three extracted glyphs — the list icon in
the header, the gear beside it, the ✕ in the panel — and neither reported anything wrong with
them. That is a bystander observation, not the check 7c specified, which was to confirm three
24×24 outline glyphs at unchanged stroke weight. Left open rather than back-dated to a pass.
**If the ✕, the gear, or the list icon ever looks heavier, squared-off, or backed by a filled
box, 7c is where the change came from.**

### What the phase did not do

Unchanged from the phase doc's *Out of scope*, and none of it is scheduled: **bookmarks** (they
need their own table and belong with a later persistence step), `landmarks` and `page-list` —
the other two ToC kinds `rbook` exposes, both absent from the fixture — and fragment-precise
"you are here" highlighting inside the panel, which is the resolve-a-position-in-the-live-DOM
problem Milestone 3 defers for annotations and jump-to-search-hit alike.

Two smaller things were named in a handoff and deliberately left standing, so they do not get
lost: a **`TocRow` component** taking `is_current: bool`, which would delete 7h's duplicated
`Some(index) == current` at the cost of a diff larger than the feature it tidies; and the
contents list having **no arrow-key navigation**, which is what a roving tabindex would buy and
what would make the current row the list's only tab stop.
