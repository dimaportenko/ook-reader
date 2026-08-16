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
3. **Name the current chapter** — `entry_for_spine`, the reverse lookup, with its chosen
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
