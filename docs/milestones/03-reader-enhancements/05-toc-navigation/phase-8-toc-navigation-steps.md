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
