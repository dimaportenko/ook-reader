# Phase 4 — Themes & Typography — Build Log

[← Phase doc](phase-4-theming.md) · decision:
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md)

Per-step build log: the crux, the step plan, and for each step the runnable check → minimal
implementation → why it works. The phase doc holds the high-level checklist; this file is the
detailed trail. Newest step appended at the bottom.

> **Prerequisite.** This phase builds on the **served-XHTML renderer** delivered by
> [Phase 3, Step 8](../../02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md):
> the current content document is served through the `/epub/` handler with
> `Content-Type: application/xhtml+xml` and rendered via iframe `src`. That handler is the
> **injection seam** every step below uses. Do Phase 3 Step 8 first.

## The crux

A theme is not "the book's CSS *or* ours." It's a **cascade**: a small reading-system layer
injected *around* the book's untouched CSS, driven by `--USER__*` custom properties on
`:root`. Changing a setting = changing one variable value and re-rendering the document.

The hard part isn't the CSS rules — it's the **injection seam**: getting our stylesheet +
variables *into* a document we render inside an isolated, script-free iframe, on every
settings change. Phase 3 Step 8 already opened that seam by serving the document through the
handler (rather than inlining it in `srcdoc`), so here we inject at serve time and re-serve on
change. Building the injected bytes is **pure Rust** (`cargo test`) — the same Rust/UI split
that kept Phase 3 small.

## Step plan (smallest-first, one idea each)

1. **Model a theme in Rust** — `Theme` enum → `:root { --USER__… }` string. Pure Rust test.
   *(enums, match, the `--USER__` convention)*
2. **Inject the USER layer** — variable block + minimal override sheet, *after* the book CSS.
   *(`rbook` `inject_css`, cascade source-order)*
3. **Add the RS-defaults layer before the book CSS** — completes RS < author < USER.
4. **Theme switcher in the app chrome** — `use_signal` → push the new variable values into
   the frame that is already on screen. *(no reload)*
5. **Typography settings** — split into sittings, because the first one is structural and the
   rest ride on it: **5a** a `Settings` struct that owns the variable list, **5b**
   `--USER__fontSize` + its control, **5c** re-measure and re-anchor after a reflow, **5d**
   `--USER__lineHeight`, **5e** `--USER__pageMargins`, **5f** `--USER__maxLineLength`,
   **5g** `--USER__fontFamily` from a curated list.
6. **Persist the settings** — a `settings` table beside `positions`, loaded before the signal
   exists and written on every change. *(the deferral every step since 4 has been logging)*
7. **Review & refactor.**

> **Dependency.** Every step here serves through the Phase 3 Step 8 handler. Steps 2–3 inject
> into the served document; Step 4 keeps that injection for the *first paint* of a chapter and
> adds a second route — a message — for changes to a document already on screen.

## Reconciliation, before Step 1 (2026-08-06)

This plan was written before Phases 5–7. The seam it describes is still the right one, but
it is no longer spelled the way the sketches below spell it. Read this first; the step
entries have not been rewritten, so where they disagree with this section, this section wins.

- **There is no `rbook` `inject_css` call to add to.** The renderer has its own injection
  helper: `serve_epub_resource` (`src/epub.rs:46`) runs
  `insert_before_head_close(&xhtml, INJECTED_ASSETS)`, where `INJECTED_ASSETS`
  (`src/web/assets.rs:21`) is a `concat!` of seven wrapped files — `pagination.css` plus the
  six JS assets that pagination, links, the page count, fragment scroll, and the position
  bridge grew out of. `EpubRewriteOptions` is still used, but only for `rewrite_paths`.
- **The USER layer cannot join `INJECTED_ASSETS`.** That const is built at compile time and
  is the same bytes for every request; a theme changes at *runtime*. So the theme block is a
  second, dynamic string appended after the static one — which is also exactly the right
  cascade position, since "after everything else in `<head>`" is what makes the USER layer
  win at equal specificity.
- **The real obstacle moved to Step 4, and it is worth knowing about now.**
  `serve_epub_resource` takes no theme, and its caller is a `use_asset_handler` closure
  registered once at mount. Getting "the theme the user just picked" into a long-lived
  handler — and getting the already-rendered iframe to pick up the change — is the actual
  work of this phase. Step 1 does not touch it, which is the point of doing Step 1 first.
- **`--ook-page` is a precedent worth copying.** `pagination.css` already drives layout from
  a custom property that something outside the stylesheet sets. The theme layer is the same
  trick with a different writer.

---

## Step 1 — model a theme in Rust

> **Status:** done — committed in `27e1d86`, together with Step 2 (**54 tests green**,
> `cargo clippy --all-targets -- -D warnings` clean). Suggested 2026-08-06, the first step of
> Phase 4; baseline to beat was **49 green**.
>
> **It did not land alone.** The plan had this step end with nothing calling `Theme` and a
> `dead_code` warning standing as the honest signal of a half-built seam. In the event Step 2
> was written on top of it before either was committed, so the two share a commit and the
> warning never became a real state of the tree — only `Night` stayed unconstructed, and it
> carries an `#[allow(dead_code)]` until Step 4's switcher. Predicted 49 → 50; actual 49 → 54,
> the extra three being Step 2's (see below).
>
> Two things shifted from the sketch below, both under review. `vars()` was split so a
> private `declarations()` owns the variable names and values, leaving `vars()` to wrap them
> in `:root { … }` — Step 5 adds four more variables and this is the one place they go.
> And the tests live in `web/assets.rs`'s `mod test` rather than a new one in `theme.rs`,
> next to the injection they are really about.

A theme is a set of `--USER__*` values, so model it as data and render it to a CSS string —
pure Rust, fully testable, before any of it touches the webview. Nothing calls it at the end
of this step; that is deliberate, and it is the same data-first shape Phase 7 opened with
(the `positions` table existed for three steps before anything wrote to it).

**Where it goes: a new `src/web/theme.rs`**, declared with `pub mod theme;` in
`src/web/mod.rs` (today a single line). It belongs under `web/` rather than at the crate root
because its output is *injected CSS* — the same job `web/assets.rs` already has, and the
module it will sit next to when Step 2 wires them together. A top-level `src/theme.rs` would
read as "the app's theme," which is a different thing: this is the theme of the **book
document**, not the chrome around it.

### Runnable check (`cargo test`)

A `#[cfg(test)] mod test` at the bottom of the new file, matching how `web/assets.rs` and
`library.rs` write theirs. Watch it fail as a **compile error** first — `Theme` doesn't
exist yet — which is the honest kind of red for a step that introduces a type.

```rust
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn every_theme_sets_both_user_colour_variables() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let css = theme.vars();

            // The USER layer drives colour through these two, by Readium convention.
            assert!(css.contains("--USER__backgroundColor"), "{theme:?} has no background");
            assert!(css.contains("--USER__textColor"), "{theme:?} has no text colour");
            // Step 2 injects this into a document that already has a `<style>`; it has to be
            // a self-contained rule, not a bare declaration list.
            assert!(css.starts_with(":root {"), "{theme:?} is not a :root rule");
        }
    }

    #[test]
    fn the_three_themes_are_actually_different() {
        assert_ne!(Theme::Day.vars(), Theme::Night.vars());
        assert_ne!(Theme::Day.vars(), Theme::Sepia.vars());
        assert_ne!(Theme::Sepia.vars(), Theme::Night.vars());
    }
}
```

The second test looks like it is asserting the obvious, and it is the one that will actually
catch something: three near-identical `match` arms full of hex literals is exactly the shape
where a copy-paste leaves two arms identical, and nothing else in the phase would notice —
Step 4 would just render a theme switcher where one button appears to do nothing.

### Minimal implementation (sketch)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Theme {
    #[default]
    Day,
    Sepia,
    Night,
}

impl Theme {
    pub(crate) fn vars(self) -> String {
        let (background, text) = match self {
            Theme::Day => ("#ffffff", "#121212"),
            Theme::Sepia => ("#faf4e8", "#5b4636"),
            Theme::Night => ("#121212", "#cfcfcf"),
        };

        format!(":root {{ --USER__backgroundColor: {background}; --USER__textColor: {text}; }}")
    }
}
```

Plus the one line in `src/web/mod.rs`. Expect a `dead_code` warning until Step 2 — the same
knowing exception Phase 7 Step 3 recorded, for the same reason: the point of the step is
that nothing consumes it yet.

### Why it works

- **A theme is just data → CSS.** Modelling it as an enum keeps day/sepia/night exhaustive
  (the compiler flags a missing arm) and makes custom themes a later "another set of values."
- **`--USER__` is the prefix that wins the cascade** (Step 2 wires it after the book CSS).
- **No webview here** — this is the testable half, deliberately first.
- **`Copy` and `Default` are not decoration.** `Copy` because `Theme` is two bits of data and
  Step 4 will read it out of a `Signal` on every render — a `Signal<Theme>` of a `Copy` type
  hands back the value with no clone and no borrow to hold. `Default` because the reader has
  to open *somewhere* before anyone has chosen, and `#[default]` on the `Day` arm says which
  arm that is in the type itself rather than in whichever call site happens to construct one
  first.
- **`fn vars(self)`, not `fn vars(&self)`.** For a `Copy` enum, taking `self` by value is
  the idiom: there is nothing cheaper about a reference to two bits than the two bits, and by
  value the method reads as a transformation of the value rather than an inspection of a
  borrow. It also means `theme.vars()` works on a `Theme` read straight out of a signal
  without a `&` at the call site.
- **A method rather than a free `theme_vars(theme)` function** (as an earlier draft of this
  step had it): `Theme::vars` keeps the data and the one thing you do with it in the same
  namespace, and when Step 5 adds typography the sibling — `Theme::typography` or a second
  settings type — has an obvious place to land.

### Scope note

Colour only, and only the two colour variables — no image filters (`darkenImages` /
`invertImages`), no typography, no persistence of the chosen theme, and nothing on screen.
Injection is Step 2, the RS-defaults layer under the book's CSS is Step 3, and the switcher
that makes any of it reachable is Step 4. The `dead_code` warning is the honest signal that
this step is one half of a seam.

---

## Step 2 — inject the USER layer

> **Status:** done — committed in `27e1d86` alongside Step 1 (**54 tests green**, 49 → 54;
> `cargo clippy --all-targets -- -D warnings` clean). The sketch below was written before
> Phases 5–7 and is kept for the record; what was actually built differs on the one point
> that matters, and the ✅ notes say where.

Step 1 produced the variables; now they must reach the served document *and* a tiny override
sheet must actually *use* them (a variable alone styles nothing). Inject both **after** the
book's CSS so they win at equal specificity.

### Runnable check

- `cargo test`: the served document string contains the `--USER__…` block **after** the book's
  `<link rel="stylesheet">` (assert the `<style>` index is greater than the `<link>` index).
- `dx serve`: with Night injected, the page background goes dark and text light **inside** the
  iframe; the book's structural styling (drop cap, headings) still renders.

> ✅ **As built.** `serving_a_chapter_injects_the_theme_after_every_other_layer` in
> `epub.rs` serves a real chapter from the fixture under `Theme::Night` and asserts the
> `--USER__backgroundColor` offset is greater than *both* `pgepub.css` (the book's own sheet)
> and `--ook-page: 0` (pagination.css), and still inside `<head>`. Two more in `assets.rs`
> cover the pieces underneath: `the_injected_layer_applies_the_variables_it_declares` checks
> that every variable declared is also read by a rule — a declaration nothing consumes styles
> nothing — and `wrapped_css_is_a_cdata_style_element` pins the CDATA wrapper.
>
> Worth recording: all three passed on their first run. They pin behaviour rather than drive
> it, because the implementation was already written when they were added. A test that never
> went red has proved less than one that did.

### Minimal implementation (sketch)

The Phase 3 Step 8 handler serves the content document; have it inject the layer at serve time.
`rbook`'s rewrite can inject CSS just before `</head>` — i.e. *after* the book's `<link>`s,
exactly the USER-after slot:

```rust
let layer = format!(
    "{vars}\nbody {{ background: var(--USER__backgroundColor) !important; \
                     color: var(--USER__textColor) !important; }}",
    vars = theme_vars(theme),
);
let rewrite = EpubRewriteOptions::default().inject_css(&layer); // confirm builder name vs your rbook
// serve manifest_entry.read_str_with(&rewrite) as application/xhtml+xml
```

> ✅ **As built — and this is where the sketch is wrong.** `inject_css` *was* used first, and
> it puts the layer in the wrong place. It writes at end-of-head **during the rewrite**, so
> its output is already in the string by the time `insert_before_head_close` runs; the static
> assets then land *after* it. Head order came out `book CSS → theme → pagination.css`, and
> since every `body` rule in pagination.css is `!important`, the theme lost every tie. Colour
> survived only because pagination.css sets no colours — Step 5's margins would not have.
> There is no ordering knob that fixes this: the two injections happen at different stages
> and the later stage always wins. So `inject_css` is gone, and the theme is concatenated
> onto `INJECTED_ASSETS` before the single `insert_before_head_close` call:
>
> ```rust
> let inject_css = format!("{INJECTED_ASSETS}{}", wrap_css_str(&theme.user_layer()));
> let with_assets = insert_before_head_close(&xhtml, &inject_css);
> ```
>
> Owning the injection means owning the `<style>` wrapper that `inject_css` was providing.
> Chapters are served as `application/xhtml+xml`, so that element's body is parsed as **XML**:
> `wrap_css_str` (`web/assets.rs`) wraps it in `/*<![CDATA[*/ … /*]]>*/`, the same thing the
> `wrap_css!` macro does for pagination.css. The macro takes an `include_str!` path and cannot
> wrap a runtime `String`, hence the function beside it. Nothing in today's CSS needs the
> escape; a `body > p` selector in Step 5 would abort the entire document without it.

### Why it works

- **Source order breaks the tie.** Same specificity, later wins — injecting after the book's
  `<link>` is what lets USER beat the author.
- **Minimal, scoped `!important`.** Only on the few properties the theme must enforce — the
  Readium discipline that avoids fighting embedded fonts / author `!important` wholesale.
- **`var(--USER__…)` indirection** is why Step 4's switch is cheap: re-serve with different
  variable values and every rule that reads them updates.

> ✅ **One addition.** "After the book's CSS" was the plan's tie-breaker; the real tie is with
> **pagination.css**, which is ours and `!important` throughout. The USER layer must come
> after that too, which makes the injected head a three-layer stack already — author, then
> reading-system, then user — before Step 3 adds the RS-defaults layer *underneath* the book.

### Scope note

This switches the Step 8 handler from serving raw doc bytes to serving an injected string for
content documents. `inject_css` writes at end-of-head only — fine for the USER (after) layer;
the **RS (before)** layer in Step 3 needs the *start* of `<head>`, which `inject_css` can't do.
Decide that there, not here.

> ✅ **As built, plus what leaked in.** `inject_css` is no longer used at all, so Step 3's
> question is now simply "where does `insert_before_head_close`'s sibling go" — a second
> helper that writes after `<head>` rather than before `</head>`.
>
> Two things landed here that this step's scope did not call for, both deliberate, both
> unfinished:
>
> - **The app shell is themed too.** `Theme::inline_styles()` on the reader's root `div`
>   (`ui/reader.rs`), so the chrome around the iframe matches the page inside it rather than
>   framing a sepia book in white. Without `!important`, unlike the book document — the shell
>   has no publisher CSS to beat, and `assets/main.css` should stay able to override it.
> - **The theme reaches the handler through context**, not a signal. `App` provides a plain
>   `Theme` and `use_register_asset_handler` copies it into a closure registered once at
>   mount, so nothing can observe a change yet — and it is pinned to `Sepia`, overriding the
>   `#[default] Day` the enum derives. That is Step 4's whole job, and it will have to
>   re-thread both the context and the handler, not just flip a `Signal`.

---

## Step 3 — the RS-defaults layer, before the book's CSS

> **Status:** done — committed in `1fe0398` (**58 tests green**, 54 → 58 as predicted;
> `cargo clippy --all-targets -- -D warnings` clean). Suggested 2026-08-07.
>
> Landed as sketched, which is worth noting after Steps 1–2 both drifted. The
> `<header>` test did its job: it is the one that fails against the obvious
> `find("<head")` implementation, and it is why the guard on the following byte is
> in the helper rather than discovered later by a book that happens to have a
> headerless chapter.

Steps 1–2 built the *top* of the cascade. This one builds the bottom: a sheet injected at the
**start** of `<head>`, so the book's own `<link>`s come after it and win. That completes
ADR-0003's three tiers — **RS < author < USER** — and it is the last structural piece; Steps
4–5 only add variables to a stack that already sorts itself.

**ADR-0003 reserved a `rbook` fork for exactly this step, and it is no longer needed.** The
ADR flagged the RS-before injection point as "the spot most likely to reopen the fork
question," because `rbook`'s `inject_css` writes only at end-of-head. Step 2 stopped using
`inject_css` at all, so head surgery is already ours: `insert_before_head_close` is nine lines
in `epub.rs`. The fork question closes not because we solved it but because Step 2 walked past
it. Worth recording in the ADR when Step 6 re-reads it.

### The crux: the closing tag is a constant, the opening tag is not

`insert_before_head_close` can be a one-line `replacen("</head>", …)` because `</head>` is the
only spelling of a closing tag. An *opening* tag is a family: `<head>`, `<head profile="…">`,
`<head\n  xmlns="…">`. The mirror of the existing helper does not exist. You have to find
where the tag *ends*, which means finding `<head` and then the next `>`.

And `"<head"` is a prefix of `"<header>"`. A document with no `<head>` but a `<header>` in its
body — perfectly legal, and EPUB3 content is XHTML5 where `<header>` is a normal sectioning
element — would match, and the sheet would land inside the body. That is the bug this step's
tests exist to catch, and it is the reason the step is worth writing rather than pattern-
matching off the helper next to it.

### Runnable check (`cargo test`)

Three unit tests on the new helper, in `epub.rs`'s `mod test` beside
`insert_before_head_close_is_a_noop_without_a_head`, plus one on the assembled document.

```rust
#[test]
fn insert_after_head_open_writes_inside_a_head_that_has_attributes() {
    let xhtml = r#"<html><head profile="http://example.org/p"><title>T</title></head><body/></html>"#;

    let out = insert_after_head_open(xhtml, "<style/>");

    assert!(out.contains(r#"<head profile="http://example.org/p"><style/><title>T</title>"#));
}

#[test]
fn insert_after_head_open_is_a_noop_without_a_head() {
    let out = insert_after_head_open("<html><body>x</body></html>", "<style/>");

    assert_eq!(out, "<html><body>x</body></html>");
}

#[test]
fn insert_after_head_open_does_not_mistake_a_header_for_a_head() {
    // `<head` is a prefix of `<header>`, which is ordinary XHTML5 sectioning content.
    let xhtml = "<html><body><header>Chapter</header><p>x</p></body></html>";

    let out = insert_after_head_open(xhtml, "<style/>");

    assert_eq!(out, xhtml);
}

#[test]
fn the_three_cascade_layers_are_served_in_priority_order() {
    let epub = Epub::open(crate::TEST_BOOK).expect("open fixture book");
    let hrefs = spine_hrefs(&epub).expect("fixture spine");

    let href = hrefs.get(2).expect("3d item in spine exists");
    let served = serve_epub_resource(&epub, &format!("/{href}"), Theme::Night)
        .expect("a spine document is reachable by its href");
    let xhtml = String::from_utf8(served.body).expect("chapters are utf-8");

    let rs = xhtml.find("--RS__").expect("the reading-system defaults");
    let author = xhtml.find("pgepub.css").expect("the book's own stylesheet");
    let user = xhtml.find("--USER__").expect("the theme layer");

    assert!(rs < author, "RS defaults must lose to the book's CSS");
    assert!(author < user, "the book's CSS must lose to the USER layer");
}
```

The third test is the one to watch fail. Write the helper the obvious way — `find("<head")` —
and the first two pass while it quietly injects into a `<header>`.

### Minimal implementation (sketch)

**A new `src/web/assets/reading-system.css`:**

```css
:root {
  --RS__maxMediaWidth: 100%;
}

img,
svg,
video {
  max-width: var(--RS__maxMediaWidth);
  height: auto;
}
```

**In `web/assets.rs`, beside `INJECTED_ASSETS`:**

```rust
pub(crate) const READING_SYSTEM_DEFAULTS: &str = wrap_css!("./assets/reading-system.css");
```

**In `epub.rs`, a sibling to `insert_before_head_close`:**

```rust
pub(crate) fn insert_after_head_open(xhtml: &str, snippet: &str) -> String {
    let Some(start) = xhtml.find("<head") else {
        return xhtml.to_owned();
    };

    let rest = &xhtml[start + "<head".len()..];
    if !rest.starts_with('>') && !rest.starts_with(char::is_whitespace) {
        return xhtml.to_owned();
    }

    let Some(end) = rest.find('>') else {
        return xhtml.to_owned();
    };

    let at = start + "<head".len() + end + 1;

    format!("{}{snippet}{}", &xhtml[..at], &xhtml[at..])
}
```

**And in `serve_epub_resource`, one line before the existing injection:**

```rust
let with_defaults = insert_after_head_open(&xhtml, READING_SYSTEM_DEFAULTS);
let with_assets = insert_before_head_close(&with_defaults, &inject_css);
```

### Why it works

- **Source order is the whole mechanism, again.** Nothing in `reading-system.css` is
  `!important` and nothing is specific — a book that sets its own `img` width simply comes
  later and wins. That is the definition of a *defaults* layer: it is what applies when the
  book says nothing.
- **`--RS__maxMediaWidth` is a real variable, not decoration.** It gives the sheet a
  variable in the prefix convention the ADR names, gives the ordering test a marker to find,
  and makes the eventual "fit image to page" setting a value change rather than a rule change.
- **The image rule earns its place here specifically.** `pagination.css` lays the body out in
  columns of `calc(100vw - 48px)`; an image wider than the column silently breaks the page
  geometry the whole reader depends on. Containing media is a reading-system concern, and
  it is the one default that is *already* load-bearing in this codebase.
- **`let … else` for the misses.** Three ways to find no head — no tag, a `<header>` match, an
  unterminated tag — and each returns the input untouched. `let … else` keeps the happy path
  unindented and at the bottom, which is the shape `serve_epub_resource` already uses with `?`.
- **The byte arithmetic is safe, and it is worth knowing why.** `find` returns *byte* offsets
  and slicing at a non-character boundary panics — a real hazard in a document full of
  em-dashes and curly quotes. It is fine here because every offset lands on an ASCII byte:
  `start` is at `<`, and `at` is one past `>`. A boundary is only ever in the middle of a
  multi-byte character, and ASCII bytes are never in the middle of one.
- **Why the RS layer sets no colours,** though Readium's does: Readium applies its user theme
  only when one is selected, so its RS layer supplies the fallback. Ours is unconditional and
  `!important` — an `--RS__backgroundColor` would be dead code the day it was written.

### Scope note

String surgery, not parsing — an attribute value containing a literal `>` (`<head
profile="a>b">`) would defeat the scan. Not worth a parser: the input is already-rewritten
XHTML from `rbook`, and the failure mode is a misplaced `<style>`, not a corrupt document.
Also deferred: a real normalize (margins, `widows`/`orphans`, hyphenation) — this step lands
the *seam*, and defaults can be appended to the file for free once the seam is proven. The
switcher that makes any of this reachable is still Step 4.

---

## Step 4 — the switcher, and pushing the theme into the live frame

> **Status:** done — committed in `ed1df0d` (**61 tests green**, 58 → 61 as predicted;
> `cargo clippy --all-targets -- -D warnings` clean). Suggested 2026-08-07, **revised twice
> the same day**, and landed as Revision 2 sketched it — the message route, the reverted
> one-argument `chapter_url`, no `Pending::Page`. The no-blink switch, the held page
> position, and the already-themed next chapter were confirmed by `dx serve`; no unit test
> reaches any of them.
>
> **Two departures from the sketch, both deliberate.** `use_signal(|| Theme::Sepia)` stays
> rather than becoming `use_signal(Theme::default)` — the opening theme is a persistence
> question, and it gets settled when the chosen theme is saved to the database, not by
> changing which hard-coded value the reader starts on. And the reader chrome was
> restructured while the picker went in: Close moved out of its absolutely-positioned corner
> into a flex header row opposite the picker, with the title absolute between them. Known
> nit, left standing: that title anchors to the viewport rather than to its flex parent,
> since the root `div` is not `position: relative`. Identical result at `height: 100vh`,
> fragile if that ever changes.
>
> **The three tests were written after the implementation**, so — as in Step 2 — they pin
> behaviour rather than drive it. The pairing test was checked against a deliberate break
> before the commit: adding `var(--USER__fontSize)` to `user_layer()` without adding it to
> `css_vars` fails with *"the layer reads --USER__fontSize, which the message never sets."*
> That is the one whose value is entirely in Step 5, so it was worth watching go red once.
>
> **Revision 1** folded in restoring the page across the reload: a theme change did not
> *reset* the page, it put Rust and the frame **out of sync** — label reading "Page 4 of N"
> over page 1, and Next jumping to 5.
>
> **Revision 2 — the current one — takes the reload back out, and the restore with it.**
> Running the built step showed the cost: every theme click tears down the document, refetches
> it, re-measures it, and hides the frame behind the `invisible` class while it settles. A
> visible blink, for a change of two colour values. The restore was never the feature; it was
> damage control for a reload that should not happen. A colour change is a **repaint**, not a
> reflow — the document on screen is already correct in every respect except two custom
> property values, and those can simply be written into it. So Revision 2 keeps the switcher
> (built, good), keeps the serve-time injection (it is what makes the *first* paint right),
> and replaces the refetch-and-restore machinery with one message.

### What is already done, and why that changes the step

The reconciliation section at the top of this doc called this step "the actual work of this
phase," on the reading that `serve_epub_resource` takes no theme and its caller is a
closure registered once at mount. Both halves of that have since been overtaken:

- **The theme is already a `Signal<Theme>`.** `App` holds `use_signal(|| Theme::Sepia)` and
  provides it; `Reader` takes `use_context::<Signal<Theme>>()`. Landed quietly inside
  `27e1d86`.
- **The handler is already reactive, for a reason worth knowing.** `use_asset_handler` does
  not store your closure — it passes it to `use_callback`, and `use_callback` *replaces its
  inner closure on every render* (`dioxus-hooks-0.7.9/src/use_callback.rs:22`). So the
  closure `use_register_asset_handler` builds each render, capturing that render's
  `theme()`, is the one the next request runs. "Registered once at mount" describes the
  *handler slot*, not the code in it. Reading `theme()` in `Reader`'s body subscribes
  `Reader` to the signal, which is what makes the re-render happen at all.
- **The switcher is built, and better than the original sketch.** A `<select>` in its own
  `src/ui/theme.rs` — which is why `Theme::from_slug` exists, since the change event hands
  back a string and the enum needs a way home from its own slug. It sits on *both* screens,
  the reader chrome and the library, the latter wrapped in a themed `div`. A `<select>` also
  *states* the current theme, which a row of buttons cannot without extra styling. Keep all
  of it; `slug` and `from_slug` both stay even though the URL no longer uses them.

So the plumbing and the control are done. What is left is the delivery.

### The crux: a colour change is not a new document

The old entry framed this as "the frame never asks again," and set out to make it ask. That
framing is what produced the blink. Look at what actually differs between the document on
screen and the document a refetch would return:

```
:root { --USER__backgroundColor: #faf4e8; --USER__textColor: #5b4636; }   ← two values
body  { background: var(--USER__backgroundColor) !important; … }          ← identical
…every other byte…                                                       ← identical
```

Two values. Every rule that consumes them is already in the document, unchanged, because
`user_layer()` emits the same rule for every theme. Nothing about the layout depends on a
colour, so every page boundary is exactly where it was and the page count cannot change.
Refetching to deliver two strings throws away a parsed document, a layout, and a scroll
position, then rebuilds all three — and the restore machinery exists purely to undo that.

The document already runs our JavaScript (`allow-scripts` on the iframe, six injected assets)
and already listens for `--ook-page` over `postMessage`. The theme is the same trick with two
more variables: **`page-listener.js` is the precedent, not `chapter-loader.js`.**

### The second crux: which declaration wins, and why nothing needs re-injecting

The message sets the variables as an **inline style on `documentElement`**:

```js
document.documentElement.style.setProperty("--USER__backgroundColor", "#121212");
```

Three facts make that enough on its own:

1. `:root` **is** `documentElement`, so the injected `:root { … }` block and the inline style
   are two declarations for the same property on the same element.
2. An inline declaration outranks any selector-based one. No `!important`, no specificity
   arithmetic, no re-injection — the served block simply loses, which is exactly what you
   want for the one it is meant to be a default for.
3. Custom properties **inherit**. `body { background: var(--USER__backgroundColor) }` reads
   whatever the root currently holds, so re-pointing the root re-paints the body with no rule
   anywhere being rewritten.

That is why the served-time injection stays: it is the value a document is *born* with, and
the message is the value it is *changed* to. Same two variables, two routes, chosen by whether
the document exists yet.

### Runnable check

**`cargo test`**, three new tests.

The first is the one that will still be earning its keep in Step 5 — it pins the two routes
to each other, in both directions:

```rust
// src/web/assets.rs
#[test]
fn the_pushed_vars_and_the_injected_layer_name_the_same_variables() {
    for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
        let layer = theme.user_layer();

        // Nothing pushed that the served layer never declares or never applies …
        for (name, value) in theme.css_vars() {
            assert!(
                layer.contains(&format!("{name}: {value};")),
                "{theme:?} pushes {name}, which the injected layer never declares",
            );
            assert!(
                layer.contains(&format!("var({name})")),
                "{theme:?} declares {name} and no rule reads it",
            );
        }

        // … and nothing read that no message will ever set.
        for reference in layer.split("var(").skip(1) {
            let name = reference.split(')').next().expect("var( … ) closes");
            assert!(
                theme.css_vars().iter().any(|(pushed, _)| *pushed == name),
                "the layer reads {name}, which the message never sets — \
                 that variable would only ever update on a chapter turn",
            );
        }
    }
}
```

The second direction is the interesting one. Add `--USER__fontSize` to the stylesheet in
Step 5 and forget to push it, and the theme half-updates live — colours move, size doesn't,
until you turn the page. No runtime error, nothing in a log. This catches it at `cargo test`.

The second test is the injection, mirroring `injects_page_listener_before_head_close`:

```rust
// src/epub.rs
#[test]
fn injects_a_theme_listener_before_head_close() {
    let xhtml = r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Hi</p></body></html>"#;

    let out = insert_before_head_close(xhtml, INJECTED_ASSETS);

    assert!(out.contains("ook-set-theme"));
    assert!(out.find("ook-set-theme").unwrap() < out.find("</head>").unwrap());
    assert!(out.contains("<p>Hi</p>"));
}
```

The third pins the two halves of the channel together, the way
`the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives` already does for the blob URL:

```rust
// src/ui/reader.rs
#[test]
fn the_theme_push_and_the_chapter_listener_agree_on_the_message_kind() {
    // Two files, one message name, no compiler between them. Rename it on one side
    // and the theme silently stops arriving — nothing errors, the colours just stop.
    assert!(THEME_PUSH_JS.contains("ook-set-theme"));
    assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-set-theme"));
}
```

And two tests come back **without being touched**: reverting `chapter_url` to one argument
makes `the_chapter_url_is_the_route_plus_the_zip_path` and
`the_chapter_url_encodes_spaces_but_keeps_path_separators` pass again as originally written.
That they were failing was the first sign the query string was fighting the design.

One test goes away with the code it covered: nothing sets `Pending::Page` any more.

**`dx serve`** for the rest, because no unit test reaches it:

- Click Night → the page inside the frame goes dark **without blinking**. No white flash, no
  blank frame, no reflow. This is the whole step; if you see a flash, the URL is still busted.
- Click back to Day, then Night again. (The second click catches a URL that changes only once
  — less relevant now, but it also catches a listener that only fires the first time.)
- The book's structural styling survives — drop caps, headings, the `<hr>` rules.
- **Turn to page 3, switch themes: you are still on page 3**, and you should be able to see
  that nothing moved, because nothing re-laid-out.
- Turn to the next chapter *after* switching → it arrives already in the new theme, from the
  served bytes. No first-frame flash of the old colours.
- Reopen a book saved mid-chapter: it still lands on the saved position. Removing the
  `Pending::Page` effect must not have disturbed the `Pending::Fragment` path.

### Minimal implementation (sketch)

**One source of truth for the variable list** (`web/theme.rs`) — the pairs, with the CSS
string derived from them rather than written twice:

```rust
pub(crate) fn css_vars(self) -> [(&'static str, &'static str); 2] {
    let (background, text) = self.colors();

    [
        ("--USER__backgroundColor", background),
        ("--USER__textColor", text),
    ]
}

fn declarations(self) -> String {
    self.css_vars()
        .iter()
        .map(|(name, value)| format!("{name}: {value};"))
        .collect::<Vec<_>>()
        .join(" ")
}
```

**`theme-listener.js`** — new, joins `INJECTED_ASSETS` beside `page-listener.js`:

```js
window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-theme") {
    return;
  }
  for (const [name, value] of e.data.vars) {
    document.documentElement.style.setProperty(name, value);
  }
});
```

**`theme-push.js`** — new, the parent side, `include_str!`d in `reader.rs` next to
`CHAPTER_LOADER_JS`:

```js
const vars = await dioxus.recv();
const frame = document.getElementById("reader-frame");

frame?.contentWindow?.postMessage({ kind: "ook-set-theme", vars }, "*");
```

**The effect in `Reader`** — this *replaces* the `Pending::Page` effect at `ui/reader.rs:65`:

```rust
use_effect(move || {
    let push = document::eval(THEME_PUSH_JS);
    _ = push.send(theme().css_vars());
});
```

**Revert `chapter_url`** to the one-argument form it had before, and the loader effect with
it:

```rust
let url = epub::chapter_url(&docs_for_iframe[chapter()]);
```

**Delete**, because nothing constructs them once the reload is gone — and `Pending::Page`
would fail `-D warnings` as a never-constructed variant if you left it:

- `Pending::Page(usize)` (`nav.rs:24`)
- `restored_page` (`nav.rs:148`)
- `on_pages` goes back to `if matches!(pending(), Pending::LastPage)`

**Keep**, though the URL no longer needs them: `slug` (the `<select>`'s option values and
labels) and `from_slug` (the change event's round trip).

Two cleanups this step still earns: `App`'s `use_signal(|| Theme::Sepia)` becomes
`use_signal(Theme::default)`, and the `#[allow(dead_code)]` on `Night` comes off — the
switcher constructs it, which is what the attribute was waiting for.

### Why it works

- **Repaint, not reflow — and that is why the restore machinery leaves.** A custom property
  that only feeds `background` and `color` invalidates paint, not layout. Column boxes,
  `scrollWidth`, page boundaries: all unchanged. `Pending::Page` was never about themes; it
  was about a *reload*, and once the reload goes it has nothing to restore. Deleting working,
  tested code is the right move when the thing it repaired no longer happens.
- **The frame stops blanking for free.** `class: if pending().is_settling() { "invisible" }`
  keys off `Pending != Nothing`. With no `Pending::Page` being set, a theme change never
  enters a settling state, so the class never applies. The blink had two sources — the
  document teardown and the deliberate hide — and one change removes both.
- **The URL goes back to naming a chapter.** The old entry argued the query was "the identity,
  not a cache-buster," and that reading is defensible: two themes really are two renderings.
  But `dataset.chapterUrl` is the only cache that reads it, and that guard exists precisely to
  say "you already have this document." A theme change *should* hit that early return. Making
  the URL differ bought correctness for one consumer at the price of a full teardown — the
  right fix is to stop needing new bytes at all.
- **Serve-time injection still earns its place, and now it is the only thing that does.** The
  handler is `use_callback`-refreshed, so a chapter fetched after a switch already carries the
  new colours in its bytes. Without it every chapter turn would paint the old theme for one
  frame and then correct — the same flash, just smaller. Bytes for a document that does not
  exist yet; a message for one that does.
- **Send data, not source.** The page effect at `ui/reader.rs:81` `format!`s its number into a
  script body; `chapter-loader.js` takes its arguments over `dioxus.recv()`. Follow the
  loader. With two hex colours the difference is invisible, but Step 5 pushes a font-family
  string, and building JavaScript source by concatenating values is a quoting bug on a timer.
- **An array of pairs is why there is no `serde_json`.** `[(&'static str, &'static str); 2]`
  serialises straight to a JSON array of two-element arrays, which JS destructures with
  `for (const [name, value] of vars)`. No new dependency, no hand-rolled JSON escaping, and
  the length is the only thing Step 5 has to change.
- **`css_vars` first, `declarations` derived — the ordering matters.** Write the pairs as the
  primary form and the CSS string as a fold over them, and the two routes cannot disagree
  about *values*. The pairing test then covers the other half: that they do not disagree about
  *which variables exist*.
- **Reading a signal inside `use_effect` is still the subscription.** `theme()` in the effect
  body registers it as a subscriber, so `.set` re-runs it and it pushes. Same lesson the old
  entry drew for the URL effect — now with one effect instead of two, and no `peek()`
  bookkeeping, because this effect writes nothing.
- **`Copy` on `Theme` is what makes `theme()` cheap here.** The effect reads the signal by
  value on every run; Step 1's derive is what lets that be a copy rather than a borrow held
  across the `send`.
- **The listener knows nothing about colours.** It loops a name/value list. Step 5 adds
  `--USER__fontSize` and friends by extending `css_vars` and the stylesheet — the JavaScript
  does not change at all.

### Scope note

Deferred, unchanged: persisting the chosen theme across launches (the reader opens on the
default every time), and any control over the app shell's colours beyond following the book.

**One race, named and accepted.** Switch themes while a chapter fetch is still in flight and
the push lands in the outgoing document, while the incoming bytes may have been served under
the old theme — that chapter stays stale until the next change. The window is a single fetch
from an in-memory zip, and it needs a theme click landing inside a chapter turn. If it ever
bites, the fix is the shape this codebase already uses twice: re-push when `pages:N` arrives,
letting the fresh document's own announcement that it exists be the trigger, exactly as
`Pending::LastPage` does. Not worth building on speculation.

**Step 5 gets cheaper, but not free.** Font-size and line-length *do* reflow, so they do need
a re-anchor — and the earlier note still stands: do it by the `ook-sel:` selector
`restored_data` already uses, not by a page number, because a page index means nothing across
a reflow. What Step 5 no longer needs is a transport: this channel carries those variables
with no new mechanism, and the pairing test above will fail the moment a new variable is added
to only one end.

---

## Step 5 — typography: the sub-plan

Step 5 was written as one line ("font-size, line-height, line-length, margins, fonts"). It is
five sittings, and the first one is not a typography feature at all.

### The crux: a theme is a palette, not a settings set

Everything Steps 1–4 built hangs off `Theme`. `Theme` produces the variable pairs, the
`:root` block, the USER layer, the app-chrome inline styles; `Signal<Theme>` is the context;
`serve_epub_resource` takes a `Theme`. That was right while every user setting *was* a colour.

Font size is not a colour, and it is not a property of night mode — you want 125% in all three
themes. So the first sitting has nothing to show on screen: it moves the ownership of "the set
of `--USER__*` variables" off `Theme` and onto a **`Settings`** struct that *has* a theme.
`Theme` goes back to being what its name says — a palette, plus the slugs the picker round-
trips through.

Two type changes carry the idea, and they are the reason this can't be deferred:

```rust
[( &'static str, &'static str ); 2]   →   Vec<(&'static str, String)>
```

- **The length stops being a compile-time constant.** Today the pushed array is exactly two
  long because a theme has exactly two colours. A settings set grows one variable per sitting,
  and every one of them has to reach both routes.
- **The values stop being literals.** `"#faf4e8"` is a `&'static str` baked into the binary;
  `"125%"` is *computed* from a number the user changed. The moment one value has to be built
  at runtime, every value in the collection has to be an owned `String`.

Both routes — the serve-time `:root` block and the `postMessage` push — already read from one
list (`css_vars`), and the pairing test from Step 4 already fails if the two ends disagree
about which variables exist. That is the machinery 5a is generalising, and once it is
generalised, 5b–5e each add a field, a rule, and a control.

### The sub-plan

- **5a — `Settings` owns the variable list.** Plumbing only, nothing visible changes.
- **5b — `--USER__fontSize` + its control.** Text resizes live. The page *count* goes stale —
  named and deferred, deliberately, so 5c is motivated by something you have watched break.
- **5c — re-measure and re-anchor after a reflow.** A colour change repaints; a size change
  **re-columns the document**, so the page you are on is a different piece of text than it was.
  `page-count.js` only re-reports on `resize`, which a font-size change is not. This sitting
  re-reports the count and lands you back on the same *words*, via the `ook-sel:` selector
  `restored_data` already uses.
- **5d — `--USER__lineHeight`.** Nearly free once 5c exists: a field, a rule, a control. (Was
  written as "line-height, then the margin / line-length pair"; split when 5d was laid out,
  because line-length changes the page *geometry* and the other two do not.)
- **5e — the margin / line-length pair.** The one that touches `pagination.css`'s
  `column-width`.
- **5f — `--USER__fontFamily` from a curated list.** The one with an ethics clause: a book
  that ships embedded fonts chose them, and Readium gates the aggressive overrides behind a
  flag rather than always winning. Curated names, not a free picker.

---

## Step 5a — a `Settings` struct that owns the variable list

> **Status:** done — committed in `df7e4f0` (62 tests green).

This is a refactor step in the middle of a phase, which the repo usually saves for the
phase-ending review. It is here instead because 5b cannot be written honestly without it:
adding `font_size` to `Theme` would say that 125% is a property of sepia.

### Runnable check

**`cargo test`.** The suite is the safety net: **61 tests green before, 62 after**, and no
existing assertion changes what it claims — only the type it calls it on. Behaviour must be
byte-identical; if a served document differs by one character, the step went wrong.

Write the new test **first**, in a file that does not exist yet. It won't compile — that's
the Rust version of watching a test fail, and the error message (`unresolved import
crate::web::settings`) is the to-do list:

```rust
// src/web/settings.rs
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_settings_variable_list_carries_the_whole_palette() {
        for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
            let settings = Settings { theme };
            let vars = settings.css_vars();

            for (name, value) in theme.css_vars() {
                assert!(
                    vars.contains(&(name, value.to_string())),
                    "{theme:?} declares {name}, and the settings list drops it",
                );
            }

            // 5b adds a variable that belongs to no theme. This is the assertion
            // that will have to change then — and it should, visibly, in that step.
            assert_eq!(vars.len(), theme.css_vars().len());
        }
    }
}
```

Then move Step 1–4's theme tests out of `web/assets.rs` and into this module, retargeted at
`Settings`. They were never about assets; they were about the layer, and the layer now lives
here. Four move (`every_theme_sets_both_user_colour_variables`,
`the_injected_layer_applies_the_variables_it_declares`,
`the_pushed_vars_and_the_injected_layer_name_the_same_variables`,
`the_three_themes_are_actually_different`); the two that really are about asset wrapping
(`the_page_formula_is_defined_once_across_the_injected_assets`,
`wrapped_css_is_a_cdata_style_element`) stay.

**`cargo clippy --all-targets`** — clean today, and the one new lint to expect is
`clippy::useless_conversion` or a needless `.clone()` around the `&'static str` → `String`
hop.

**`dx serve`**, thirty seconds, because the point is that nothing moved: the picker still
switches all three themes on both screens, live, with no blink; a chapter turn still arrives
in the current theme.

### Minimal implementation (sketch)

**New — `src/web/settings.rs`.** Everything that describes *the whole USER layer* moves here
from `Theme`: `declarations`, `vars`, `user_layer`, `inline_styles`.

```rust
use crate::web::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
}

impl Settings {
    pub(crate) fn css_vars(self) -> Vec<(&'static str, String)> {
        self.theme
            .css_vars()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect()
    }

    fn declarations(self) -> String {
        self.css_vars()
            .iter()
            .map(|(name, value)| format!("{name}: {value};"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn vars(self) -> String {
        format!(":root {{ {} }}", self.declarations())
    }

    pub(crate) fn user_layer(self) -> String {
        format!(
            "{}\nbody {{ background: var(--USER__backgroundColor) !important; \
                color: var(--USER__textColor) !important; }}",
            self.vars()
        )
    }

    pub(crate) fn inline_styles(self) -> String {
        format!(
            "{} background-color: var(--USER__backgroundColor); color: var(--USER__textColor)",
            self.declarations()
        )
    }
}
```

**`src/web/theme.rs`** keeps `colors`, `css_vars`, `slug`, `from_slug` — and loses the four
methods above. **`src/web/mod.rs`** gains `pub mod settings;`.

**`src/epub.rs`** — the parameter changes type and name in two signatures and one body:

```rust
pub(crate) fn serve_epub_resource(epub: &Epub, path: &str, settings: Settings) -> Option<Served>
pub(crate) fn use_register_asset_handler(epub: Rc<Epub>, settings: Settings)
```

with `wrap_css_str(&settings.user_layer())` at `epub.rs:65`. The ten test call sites become
`Settings::default()` and `Settings { theme: Theme::Night }`.

**`src/main.rs:39`** — `use_signal(Settings::default)`, provided as `Signal<Settings>`, and
`settings().inline_styles()` at line 75.

**`src/ui/reader.rs:45`** — `use_context::<Signal<Settings>>()`, `settings()` to the handler
at line 46, `settings().css_vars()` in the push effect at line 68.

**`src/ui/theme.rs`** — the picker reads and writes *through* settings:

```rust
let mut settings = use_context::<Signal<Settings>>();
…
onchange: move |event| {
    let theme = Theme::from_slug(&event.data.value());
    settings.write().theme = theme;
},
…
selected: settings().theme == opt,
```

### Why it works

- **`Vec` instead of `[_; 2]`, and why the array had to go.** A fixed-size array is a
  *promise about the count*, and `[(&str, &str); 2]` promises the reader has exactly two
  settings. Every sitting after this one breaks that promise, and each break would be an edit
  to a type signature, its two call sites, and the tests — for nothing. `Vec` gives up
  stack allocation and compile-time length in exchange for a collection that grows by
  changing one function body.
- **`&'static str` → `String` is the *runtime-value* boundary, not a style choice.** The
  colours can be `&'static str` because they are string literals living in the binary for the
  whole program. `format!("{}%", self.font_size)` in 5b produces a `String` that is allocated
  when the user clicks — it has no `'static` lifetime to hand out. A collection is one type,
  so one runtime-computed value forces every element to be owned. Doing that hop now, on
  values that are still literals, means 5b touches one line instead of a signature.
- **`.to_string()` on a `&'static str` really does copy — and that is fine.** It heap-
  allocates and memcpys seven bytes, once per settings change. The alternative that avoids it
  is `Cow<'static, str>`, which is the right tool when the copy is measurable; here you'd be
  paying an enum's worth of extra reading to save a handful of bytes on a click. Reach for
  `Cow` when a profiler asks.
- **`css_vars` stays the single source of truth, one level up.** `declarations` folds over it,
  so the served `:root` block and the pushed pairs still cannot disagree about values. The
  pairing test covers the other half — that they cannot disagree about which variables *exist*
  — and it is about to become the most valuable test in the phase, because 5b–5e each add a
  variable that has to reach both ends.
- **`Settings { theme }` is composition, and the borrow checker rewards it.** `Settings` has-a
  `Theme` rather than replacing it, so `Theme` keeps a job (a palette + its slugs) and the
  picker keeps its round-trip. `Copy` survives because every field is `Copy` — which is what
  lets `settings()` in the effect be a cheap read rather than a borrow held across the `send`.
- **`settings.write().theme = theme` — order of evaluation is load-bearing.** `write()` hands
  back a guard that marks the signal dirty when it drops; while it is alive, any read of the
  same signal panics at runtime. Computing `Theme::from_slug(...)` into a local *before* the
  assignment is what keeps a read out of that window. The old code could `set` because the
  whole value was the theme; now it is a field, so it is a read-modify-write.
- **`selected: settings().theme == opt` still derives from state.** Nothing here changes the
  Dioxus lesson from Step 4: the `<select>` renders from the signal, so it cannot drift out of
  sync with what the frame is showing.

### Scope note

No new variable, no new control, nothing visible on screen — that is the definition of done
here, and the suite staying at the same assertions is how you prove it. `--USER__fontSize`
lands in 5b. Persisting settings across launches is still deferred (5a is where that will
eventually hook in, since a `Settings` struct is the thing you'd serialise, but nothing about
this sitting anticipates it).

**Not covered by any test:** that the served bytes are *identical* before and after. The
existing `serving_a_chapter_injects_the_theme_after_every_other_layer` checks the layer's
content and its position, which is strong enough — a golden-file test of a whole served
document would fail on every future sitting for the right reason and the wrong cost.

### What actually landed

Two departures from the sketch above, both deliberate:

- **The library screen lost its theme picker** and the themed wrapper `div` around it, so
  the app chrome outside the reader is now unstyled. The sketch said "nothing visible
  changes"; this is the one thing that did. The reader keeps its own picker, which is the
  one that matters while the settings set is growing — the library screen gets its own
  entry point back when there is more than a palette to put in it.
- **The default theme is `Day`, not `Sepia`.** `use_signal(Settings::default)` inherits
  `Theme`'s `#[default]`, where the old `use_signal(|| Theme::Sepia)` named its choice.
  Not worth a field default of its own until settings are persisted.

Also worth noting for **Step 6**, and not a 5a regression — it predates this sitting:
`use_register_asset_handler` is handed a `settings()` *snapshot* at mount, and
`use_asset_handler` registers once. A theme change repaints the live frame through the
`postMessage` push, but the handler keeps serving the theme the reader mounted with, so a
chapter turn after a change is served the stale palette and then corrected by the push. It
is invisible today; it stops being invisible when a setting changes *layout* rather than
colour, which is exactly 5b.

---

## Step 5b — `--USER__fontSize` and its control

> **Status:** done — committed in `1529fc4` (65 tests green, 62 → 65; clippy clean).

The first setting that is not a colour, end to end: a field, a rule, a control. 5a built the
machinery for exactly this, so the interesting part of this sitting is not the plumbing —
it is that **two tests you already have go red before you write a line of implementation**,
each for a different right reason. That is the whole payoff of the last sitting, and it is
worth watching happen before you fix it.

### The crux: a percentage is a scale factor, and `Default` is a trap

Two things make this bigger than "add a field".

**`Settings` can no longer derive `Default`.** `u16`'s default is `0`, so a derived
`Settings::default()` would serve `font-size: 0%` — and `Settings::default()` is the value
behind eight `epub.rs` test call sites and the app's initial signal. The derive would
compile, the type would be correct, and the reader would render nothing. Deriving `Default`
is only right when *every field's* zero value is the sensible default; the moment one field
has a meaningful starting point, the derive silently lies. So it goes, replaced by a
hand-written `impl Default for Settings`.

**The value is a percentage, not a pixel size** — and that choice is doing real work.
`font-size: 125%` on the root means *"1.25× whatever the user agent's default is"*, so every
`em` and `rem` in the book's own stylesheet keeps its relationship to everything else: a
chapter heading the author set at `2em` stays twice the body text at every setting. A pixel
value would flatten that — you would be picking the body size *and* silently overriding the
author's typographic hierarchy. This is also why the variable belongs on `html` (the `:root`
that `theme-listener.js` already writes to) rather than on `body`: the root is where the
whole `em` cascade is anchored.

### Runnable check

**`cargo test`.** **62 tests before, 65 after** — but the order matters here. Add the field
*first*, before touching `user_layer`, and watch two existing tests fail:

1. `the_settings_variable_list_carries_the_whole_palette` — the tripwire planted in 5a. Its
   `assert_eq!(vars.len(), theme.css_vars().len())` was written to fail exactly now. Change
   it to `theme.css_vars().len() + 1` and say why in the message.
2. `the_pushed_vars_and_the_injected_layer_name_the_same_variables` — this one you did *not*
   write for this step, and it still catches you:

   ```
   Day declares --USER__fontSize and no rule reads it
   ```

   The variable reaches the `:root` block for free (`declarations` folds over `css_vars`),
   but nothing *applies* it, so the number would change and the text would not. This is the
   test earning its keep — it is the only thing standing between you and a control that
   looks like it works.

Then the three new ones, in `src/web/settings.rs`:

```rust
#[test]
fn the_default_font_size_is_100_percent() {
    // Not a style preference: a derived `Default` gives `0`, which serves
    // `font-size: 0%` to every caller of `Settings::default()`.
    assert_eq!(Settings::default().font_size, 100);
}

#[test]
fn the_font_size_steps_and_clamps() {
    let mut settings = Settings {
        font_size: 150,
        ..Settings::default()
    };

    settings.zoom_out();
    assert_eq!(settings.font_size, 150 - FONT_SIZE_STEP);
    settings.zoom_in();
    assert_eq!(settings.font_size, 150);

    for _ in 0..20 {
        settings.zoom_out();
    }
    assert_eq!(
        settings.font_size, FONT_SIZE_MIN,
        "zooming out past the floor must clamp, not underflow",
    );

    for _ in 0..20 {
        settings.zoom_in();
    }
    assert_eq!(
        settings.font_size, FONT_SIZE_MAX,
        "zooming in past the ceiling must clamp, not overflow",
    );
}

#[test]
fn the_font_size_reaches_the_layer_as_a_percentage() {
    let settings = Settings {
        font_size: 125,
        ..Settings::default()
    };

    assert!(settings
        .css_vars()
        .contains(&("--USER__fontSize", "125%".to_string())));

    let layer = settings.user_layer();

    assert!(
        layer.contains("--USER__fontSize: 125%;"),
        "the chosen size never reached the :root block",
    );
    assert!(
        layer.contains("font-size: var(--USER__fontSize)"),
        "the layer declares a size it never applies — the number would move \
         and the text would not",
    );
}
```

**`cargo clippy --all-targets`** — the lint to expect is on the arithmetic: reach for
`saturating_add`/`saturating_sub` rather than `+`/`-` and the question doesn't come up.

**`dx serve`** — the eyeball check, and the one that motivates 5c. Open a chapter and press
`A+` twice. The text should resize *live*, with no reload and no blink, in all three themes.
Then look at the page label: **it still says the old count**, and the page you are on now
shows different words than it did. That is the bug 5c fixes, and it is much easier to fix
something you have watched happen.

### Minimal implementation (sketch)

**`src/web/settings.rs`** — the field, the bounds, the steppers, and one new rule:

```rust
pub(crate) const FONT_SIZE_MIN: u16 = 75;
pub(crate) const FONT_SIZE_MAX: u16 = 250;
pub(crate) const FONT_SIZE_STEP: u16 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_size: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            font_size: 100,
        }
    }
}

impl Settings {
    pub(crate) fn zoom_in(&mut self) {
        self.font_size = self.font_size.saturating_add(FONT_SIZE_STEP).min(FONT_SIZE_MAX);
    }

    pub(crate) fn zoom_out(&mut self) {
        self.font_size = self.font_size.saturating_sub(FONT_SIZE_STEP).max(FONT_SIZE_MIN);
    }

    pub(crate) fn css_vars(self) -> Vec<(&'static str, String)> {
        let mut vars: Vec<(&'static str, String)> = self
            .theme
            .css_vars()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect();

        vars.push(("--USER__fontSize", format!("{}%", self.font_size)));

        vars
    }
```

with `user_layer` gaining the rule that reads it:

```rust
    pub(crate) fn user_layer(self) -> String {
        format!(
            "{}\nhtml {{ font-size: var(--USER__fontSize) !important; }}\
             \nbody {{ background: var(--USER__backgroundColor) !important; \
                color: var(--USER__textColor) !important; }}",
            self.vars()
        )
    }
```

**New — `src/ui/settings.rs`**, mirroring `web/settings.rs`:

```rust
use dioxus::prelude::*;

use crate::web::settings::{Settings, FONT_SIZE_MAX, FONT_SIZE_MIN};

#[component]
pub(crate) fn FontSizeControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();

    rsx! {
        div {
            button {
                disabled: settings().font_size <= FONT_SIZE_MIN,
                onclick: move |_| settings.write().zoom_out(),
                "A-"
            }
            span {
                style: "padding: 0 0.5rem;",
                "{settings().font_size}%"
            }
            button {
                disabled: settings().font_size >= FONT_SIZE_MAX,
                onclick: move |_| settings.write().zoom_in(),
                "A+"
            }
        }
    }
}
```

**`src/ui/mod.rs`** gains `pub mod settings;`. **`src/ui/reader.rs:117`** mounts it beside
the picker, in the same right-hand chrome `div`:

```rust
div {
    style: "display: flex; gap: 1rem; padding: 0.5rem 1rem; z-index: 1;",
    FontSizeControl {}
    ThemePicker {}
}
```

Nothing else changes. In particular **no JavaScript changes at all** — which is the point.

### Why it works

- **The push route is already done, and that is 5a's dividend.** `use_effect` sends
  `settings().css_vars()`, `theme-listener.js` loops over whatever pairs arrive and calls
  `setProperty` for each. Neither end knows or cares that the list grew from two to three. If
  `css_vars` had still returned `[(&str, &str); 2]`, this step would have had to change the
  return type, both call sites, and the tests before it could add anything.
- **`format!("{}%", self.font_size)` is why the values had to become owned.** This `String`
  is built when the user clicks; there is no `'static` lifetime to hand out for it. That hop
  cost one line here because it was paid in 5a — it would have cost a signature change today.
- **`saturating_add(STEP).min(MAX)` and the mirror for down.** The saturate handles the type's
  bounds (`u16` can't wrap to a tiny number), the `min`/`max` handles *your* bounds. Two
  different failures, two different guards, and the plain `+`/`-` form gives you neither —
  in release builds an overflow wraps silently rather than panicking.
- **`Settings { font_size: 125, ..Settings::default() }` — struct update syntax.** It fills
  the fields you didn't name from another value of the same type. Worth adopting in tests
  now: every later sitting adds a field, and the tests written this way don't need touching.
- **`settings.write().zoom_out()` is a read-modify-write through the guard.** `write()` hands
  back a `WriteSignal` guard that marks the signal dirty when it drops at the end of the
  statement; `zoom_out` takes `&mut self` *through* that guard. The rule from 5a still holds
  — no read of the same signal while the guard is alive — and here it is satisfied because
  the whole computation happens inside `zoom_out`, on `&mut self`, never touching the signal.
- **`disabled:` and the readout both derive from the signal.** Reading `settings()` during
  render subscribes the component, so the buttons grey out at the bounds and the `%` label
  moves without any separate bookkeeping. And `Signal` is `Copy`, which is what lets the same
  `settings` handle be captured by two `move` closures without a `.clone()` in sight.
- **Why `!important` on the root rule.** Not a specificity contest with the book's
  `body { font-size: … }` — that targets a different element and simply *inherits* the root
  size, which is the whole mechanism. The `!important` is there for the narrower case of a
  book that sets `html`/`:root` font-size itself, with `!important` of its own; being served
  last only wins ties between declarations of equal weight. Readium's font-size module marks
  its override the same way, for the same reason.

### Scope note

Four things this step deliberately does not do:

- **The page count goes stale, and the page you are on drifts.** A font-size change re-columns
  the document, but `page-count.js` only re-reports on `load` and `resize`, and a variable
  change is neither. **5c** fixes both halves — re-report the count, and re-anchor on the same
  *words* via the `ook-sel:` selector.
- **The app chrome does not scale.** `inline_styles` will carry `--USER__fontSize` into the
  wrapper's `style` attribute, where nothing reads it. The setting is about the book's text,
  not the reader's buttons; leave it that way unless it looks wrong.
- **Books that set text sizes in absolute `px` will not respond.** Their author CSS is not
  relative to the root, so scaling the root does nothing for them. Readium solves this with a
  much more aggressive override that also rewrites author declarations; that trade-off belongs
  with **5e**'s embedded-font question, not here.
- **Nothing is persisted.** A relaunch is back to 100% and Day, same as the theme.

**One naming smell to carry into Step 6:** the bridge message is still called
`ook-set-theme`, and it now carries typography. The mechanism is right and the name has
stopped describing it.

### What actually happened

**The tripwire fired, and so did the pairing test — both as written.** Adding the field
before touching `user_layer` broke the suite in two places for two different reasons: the
`assert_eq!(vars.len(), …)` planted in 5a, which only needed its `+ 1`, and
`the_pushed_vars_and_the_injected_layer_name_the_same_variables` with
`Day declares --USER__fontSize and no rule reads it`. The second is the one worth keeping
score of, because nobody wrote it for this step. A variable that reaches `:root` and no
rule reads is a control that moves a number and nothing else, and that test is the only
thing that would have caught it.

**One test call site outside `web::settings` had to move too.** `epub.rs`'s
`serving_a_chapter_injects_the_theme_after_every_other_layer` built a `Settings { theme }`
literal, which stopped compiling the moment the struct grew. It took `..Settings::default()`
rather than a second literal field — struct update syntax is the shape that survives 5c–5e,
each of which adds a field. The five literals inside `web::settings` still name `font_size`
by hand; they are worth converting the next time that file is opened.

**The departure that mattered: `html` versus `body`.** The first pass put `font-size` on
the existing `body` rule, which works — `body` inherits from the root, so 125% is still
1.25× — but leaves `rem` resolving against an unscaled `html`, so any author rule using
`rem` would not move. The correction over-shot in the other direction and moved the *whole*
block, colours included, to `html`. That is the version to remember, because it looks
strictly tidier and is a regression:

> `!important` does not survive being moved up a level, because **inheritance is not
> specificity**. `html { color: … !important }` wins the cascade *for `html`*; `body` then
> merely inherits it, and an inherited value loses to any declaration matching `body`
> directly. A book with a plain `body { color: #000 }` — common — takes the night palette
> back to black text on a dark ground, with no `!important` of its own required. Background
> fails the same way: `html`'s propagates to the canvas, and the book's `body` background
> paints a box over it.

So the layer ends up as two rules rather than one, and the split is load-bearing: the size
belongs on the root *because* `body` inherits it, and the colours belong on `body` *because*
inheritance is exactly what would defeat them.

Worth noting that the suite was green for every one of those three arrangements.
`the_injected_layer_applies_the_variables_it_declares` asserts the layer declares each
variable and reads it somewhere; it never asserts *which element* the rule targets. The
tests pinned the plumbing and had nothing to say about the cascade — which is the honest
limit of a string-matching test on CSS, and the reason the `dx serve` check is a gate here
and not a courtesy.

**The three new tests were written after the implementation**, as in steps 2 and 4.

---

## Step 5c — Re-measure and re-anchor after a reflow

> **Status:** done — committed in `2a9e181` (67 tests green, 65 → 67 as predicted; clippy
> clean; the four `dx serve` checks confirmed by hand).

5b ended by having you watch two things break. Press `A+` mid-chapter and the page label
still reads the old count, and the words in front of you are not the words that were there
a moment ago. Both have the same cause and neither has the same fix, which is what makes
this step worth its own sitting.

### The crux: a reflow invalidates two derived values, and neither knows it

`page_count` and `page` are not state — they are *measurements* of a layout. Everything the
reader knows about where you are is derived from a column count that only exists once the
browser has laid the document out:

```
page_count = ceil(body.scrollWidth / innerWidth)      ← how many columns fit
page       = round(el.offsetLeft / innerWidth)        ← which column an element is in
```

Change the font size and both denominators stay put while the numerators move. The document
re-columns, and the two numbers that describe it are now describing a layout that no longer
exists. `page-count.js` re-reports on `load` and `resize`, because those were the only two
ways the layout could change when it was written. A CSS variable write is neither, so
nothing re-reports and nothing re-anchors.

The insight that makes the fix small: **the anchor already exists.** `page-position.js`
computes `selectorFor(firstElementOnPage(page))` on every page turn, to persist where you
are. That is precisely "the words I am looking at, named in a way that survives a relayout"
— the same `ook-sel:` selector `restored_data` feeds back in on launch. A reflow is a
tiny, in-memory version of quitting and reopening the book: note the selector, let the
layout change, find the selector again, go to whatever page it landed on.

### Runnable check

**Two Rust tripwire tests plus a `dx serve` eyeball.** The behaviour lives in JavaScript, so
the tests can only guard the seams between the files — that is the same bargain
`the_loader_and_the_cleanup_agree_on_where_the_blob_url_lives` and
`the_fragment_prefix_matches_the_one_the_asset_looks_for` already struck, and the reason the
eyeball check is a real gate here rather than a courtesy. **65 tests before, 67 after.**

In `src/ui/reader.rs`:

```rust
#[test]
fn the_reflow_message_survives_all_three_hops() {
    // theme-listener.js posts it, ook-events-listener.js forwards it, `parse` reads
    // it back. Three files, one name, and no compiler between any two of them —
    // rename it in one and the count goes stale again, silently.
    assert!(crate::web::assets::INJECTED_ASSETS.contains("ook-reflow"));
    assert!(BRIDGE_JS.contains("ook-reflow"));

    assert_eq!(BridgeMsg::parse("reflow:7"), Some(BridgeMsg::Reflow(7)));
    assert_eq!(BridgeMsg::parse("reflow:notanumber"), None);
}
```

In `src/web/assets.rs`:

```rust
#[test]
fn the_reflow_handler_reuses_the_position_helpers() {
    // The anchor has to be the same notion of "where I am" that page-position.js
    // persists, which means the same code and not a second copy of it. A copy would
    // drift the first time one of the two is fixed.
    assert_eq!(INJECTED_ASSETS.matches("function selectorFor").count(), 1);
    assert_eq!(
        INJECTED_ASSETS
            .matches("function firstElementOnPage")
            .count(),
        1
    );
    assert_eq!(INJECTED_ASSETS.matches("const report =").count(), 1);
}
```

**`cargo clippy --all-targets`**, as always.

**`dx serve`** — the check that actually proves the step:

1. Open a chapter and page forward until you are somewhere in the middle. Note the first
   few words on screen and the `Page X of N` label.
2. Press `A+`. **`N` changes** (more columns at a bigger size) and **the words stay put**.
   `X` will usually change, and should — you are on a different column of a different
   layout, looking at the same sentence.
3. Press `A-` twice, then `A+` twice. You should land back on the same words each time,
   not drift a little further each press.
4. Switch theme. **Nothing jumps and `N` does not change** — a colour change does not
   re-column, so this path has to be a no-op for it.

### Minimal implementation (sketch)

**`src/web/assets/theme-listener.js`** — the whole step, really. Capture, apply, re-measure,
re-anchor:

```js
window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-theme") {
    return;
  }

  const before = currentPage();
  const anchorEl = firstElementOnPage(before);
  const anchor = anchorEl && selectorFor(anchorEl);

  for (const [name, value] of e.data.vars) {
    document.documentElement.style.setProperty(name, value);
  }

  report();

  const moved = anchor && document.querySelector(anchor);
  if (!moved) {
    return;
  }

  const page = pageOf(moved);
  if (page !== before) {
    window.parent.postMessage({ kind: "ook-reflow", page: page }, "*");
  }
});
```

**`src/web/assets/ook-events-listener.js`** gains one more forward, beside the other four:

```js
  if (e.data.kind === "ook-reflow") {
    dioxus.send("reflow:" + e.data.page);
  }
```

**`src/ui/reader.rs`** — a variant, a parse arm, a dispatch arm:

```rust
pub(crate) enum BridgeMsg {
    Link(String),
    Scroll(usize),
    Pages(usize),
    Position(String),
    Reflow(usize),
}
```

```rust
        } else if let Some(page) = msg.strip_prefix("reflow:") {
            page.parse().ok().map(BridgeMsg::Reflow)
```

```rust
                    Some(BridgeMsg::Reflow(page)) => state.on_reflow(page),
```

**`src/nav.rs`** — the smallest method in the file, and the point of the whole exercise:

```rust
    pub(crate) fn on_reflow(self, p: usize) {
        self.data.page().set(p);
    }
```

### Why it works

- **Reading a layout property flushes the layout.** `setProperty` does not recompute
  anything; it invalidates and returns. But `report()` reads `document.body.scrollWidth`,
  and `pageOf` reads `el.offsetLeft`, and both are *layout-forcing reads* — the browser must
  bring style and layout up to date before it can answer. So the code above needs no
  `requestAnimationFrame` and no `setTimeout(0)`: the read on the line after the write is
  already looking at the new layout. This is the same "forced synchronous layout" that
  profilers flag as an anti-pattern in a loop; here it is exactly the tool for the job,
  because we write once and read once.
- **`transform` does not move `offsetLeft`.** `pagination.css` pages by
  `translateX(calc(var(--ook-page) * -100vw))`, and transforms are a paint-time effect that
  leaves layout geometry untouched. That is why `pageOf` can be asked "which column is this
  element in" at any time without first scrolling anywhere, and why the anchor works
  identically on page 0 and page 40.
- **Capture before, resolve after — and the guard order matters.** `selectorFor(null)`
  does *not* return `null`; its `while (el && …)` loop simply never runs and it hands back
  the string `"body"`. Feed that to `querySelector` and you get the body element, whose
  `pageOf` is 0, and every settings change on a not-yet-populated document would yank you to
  page 0. `page-position.js` guards this by checking the element before building the
  selector (`if (!el) return;`), and `anchorEl && selectorFor(anchorEl)` is the same guard
  in expression form.
- **`report` is reachable from another file, but not through `window`.** `const report =
  …` at the top level of a classic script creates a binding in the *global lexical
  environment*, which every later classic script in that document can see — and
  `theme-listener.js` is last in `INJECTED_ASSETS`, after `page-count.js` and
  `page-position.js`. It is not a property of `window`, though, so `window.report()` is
  `undefined`. Same for `pageOf` and `currentPage`, which are `function` declarations and
  *are* on `window`; the inconsistency is real and worth knowing rather than working around.
- **`ook-reflow` rather than reusing `ook-scroll`.** The tempting shortcut is to post
  `ook-scroll`, which already means "the document decided which page you are on" and already
  sets `page`. The cost is hidden in `on_scroll`: it also clears `Pending::Fragment`. That
  clearing is load-bearing for `fragment-scroll.js` — it is what un-hides the frame once a
  restored position has settled — so borrowing the message means a settings change that
  lands mid-settle would drop the restore and strand you at the top of the chapter. Two
  events that produce the same *state change* are still two events when one of them carries
  an extra meaning. `on_reflow` sets the page and says nothing about pending, which is the
  whole difference.
- **The count goes out before the page, and that order is free.** `postMessage` delivers in
  order and the bridge's `recv` loop consumes in order, so `report()` before the
  `ook-reflow` post means `page_count` is updated before `page` reads against it. Reverse
  them and the label flickers `Page 12 of 8` for a frame.
- **The saved position updates itself.** `on_reflow` writes `page`, which re-runs the
  `ook-set-page` effect, which makes `page-position.js` report a fresh `ook-position`, which
  the bridge persists. Nothing new to write — the existing loop closes over the new layout.
  And there is no cycle, because `ook-position` is a different message that never sets the
  page.
- **Posting only when the page actually moved** is what keeps a colour change silent. Day →
  Night does not re-column, so the anchor resolves to the same column, `page === before`, and
  no message is sent. Without the guard every theme click would push a redundant page write
  through the whole loop.

### Scope note

- **`resize` still re-reports without re-anchoring.** Now that a reflow handler exists,
  pointing the `resize` listener at it is nearly free — but resizing the window is a
  different user action with its own feel, and it is a pre-existing behaviour rather than
  something this step breaks. Left for **Step 6** to fold in deliberately.
- **Only the live frame.** A chapter that has not loaded yet still gets its settings baked
  into the served bytes, which is the other half of the two-route design from Step 4 and
  unchanged here.
- **`theme-listener.js` is now badly named**, for the second step running — it listens for
  settings, measures a layout and re-anchors a reading position. Step 6 renames it and the
  `ook-set-theme` message with it.
- **5d and 5e ride this for free.** Line-height, margins and font-family all reflow, and all
  three arrive through the same `ook-set-theme` push. Once this handler is right, those steps
  are a field, a rule and a control again — with no positioning work of their own.

### Departures and notes from the sitting

- **The step landed as sketched.** Both tripwires went green on the first run, 65 → 67 as
  predicted, and all four `dx serve` checks passed — including the one that matters most,
  that a theme switch leaves `N` alone and nothing jumps.
- **`INJECTED_ASSETS` now has a load-bearing *order*, and no test guards it.**
  `theme-listener.js` reaches for `report`, `selectorFor`, `firstElementOnPage`, `pageOf`
  and `currentPage` across four other files, and it only sees them because it is **last** in
  the `concat!`. `const report = …` is a binding in the global lexical environment, visible
  only to scripts that run *after* it — move `theme-listener.js` up the list and the handler
  dies with a TDZ `ReferenceError` the first time a setting changes. The new
  `the_reflow_handler_reuses_the_position_helpers` counts occurrences, not positions, so it
  would stay green through that. **Step 6** should either assert the order or say so in a
  comment on the `concat!`; today it is true by accident of how the list grew.
- **A narrow `Pending::LastPage` interleave, recorded and left alone.** `report()` fires
  before the `ook-reflow` post, so `on_pages` runs first — and if `Pending::LastPage` is set
  it snaps `page` to the last page and clears pending, after which `on_reflow` overwrites it
  with the anchor's page. Only reachable if a settings change lands mid chapter-prev-settle,
  and the fix would be to give `on_reflow` an opinion about `pending`, which is exactly the
  entanglement this step avoided by not reusing `ook-scroll`. Known, not fixed.

---

## Step 5d — `--USER__lineHeight`

> **Status:** done — committed in `eb92bb6` (72 tests green, 67 → 72; clippy clean).

The sub-plan wrote 5d as "line-height, **then** the margin / line-length pair" — three
settings in one line. Line-height is one idea and the pair is another (line-length is the one
that reaches into `pagination.css`'s `column-width`), so this sitting is **line-height alone**
and the pair becomes **5e**; font-family slides to **5f**. Same work, one idea each.

This is the first step that gets to cash 5c's cheque. There is no positioning work in it, no
new message, no new JS: a field, a rule, a control. What it *does* have is two small traps
worth the sitting — the unit and the selector.

### Runnable check first

Two of these are already written and will go **red before you touch anything else**, which is
the nicest kind of failing test — the ones from earlier steps that told you they would.

**`cargo test`**, in `src/web/settings.rs`'s `mod test`:

The existing count assertion breaks the moment `css_vars` grows, and it says so:

```rust
            assert_eq!(
                vars.len(),
                theme.css_vars().len() + 1,
                "the palette plus --USER__fontSize — bump this when a setting is added",
            );
```

Bump it to `+ 2` and fix the message. `the_pushed_vars_and_the_injected_layer_name_the_same_variables`
also goes red on its own, and needs **no** edit — it walks whatever `css_vars` returns and
demands the layer both *declare* and *read* each name. That is 5a's design paying out: add a
variable and the pairing test starts guarding it for free.

Then the new ones:

```rust
    #[test]
    fn the_line_height_reaches_the_layer_unitless() {
        let settings = Settings {
            line_height: 140,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__lineHeight", "1.40".to_string())));

        let layer = settings.user_layer();

        assert!(
            layer.contains("--USER__lineHeight: 1.40;"),
            "the chosen leading never reached the :root block",
        );
        assert!(
            layer.contains("line-height: var(--USER__lineHeight)"),
            "the layer declares a leading it never applies — the number would move \
             and the lines would not",
        );
    }

    #[test]
    fn a_line_height_below_a_tenth_keeps_its_leading_zero() {
        // 105 hundredths is 1.05, not 1.5. `{}.{}` prints the latter, and the gap
        // between "a hair looser" and "half again as loose" is invisible in the
        // source and obvious on screen.
        let settings = Settings {
            line_height: 105,
            ..Settings::default()
        };

        assert!(settings
            .css_vars()
            .contains(&("--USER__lineHeight", "1.05".to_string())));
    }

    #[test]
    fn the_line_height_rule_reaches_the_elements_the_book_styles() {
        // A rule on `body` alone only supplies an *inherited* value, and inheritance
        // fills in what nothing else declares. A book that says `p { line-height: 1.2 }`
        // has declared it, so the paragraphs — the only text you actually read — would
        // ignore the setting entirely.
        assert!(Settings::default().user_layer().contains("body *"));
    }

    #[test]
    fn the_line_height_steps_and_clamps() {
        let mut settings = Settings {
            line_height: 150,
            ..Settings::default()
        };

        settings.tighter();
        assert_eq!(settings.line_height, 150 - LINE_HEIGHT_STEP);
        settings.looser();
        assert_eq!(settings.line_height, 150);

        for _ in 0..20 {
            settings.tighter();
        }
        assert_eq!(settings.line_height, LINE_HEIGHT_MIN);

        for _ in 0..20 {
            settings.looser();
        }
        assert_eq!(settings.line_height, LINE_HEIGHT_MAX);
    }
```

67 → 71. (72 as committed — review added a fifth test; see the departure note below.)

**`cargo clippy --all-targets`**, as always.

**`dx serve`** — the part the tests cannot see:

1. Open a chapter, page into the middle, note the first words and `Page X of N`.
2. Press the leading `+`. The lines open up, **`N` changes**, and **the words stay put** —
   5c's handler is doing this with nothing new written for it.
3. Find a chapter with a heading in it. The heading's leading should scale *with* its own
   size, not inherit the body's — a unitless value is what buys that (see below).
4. Switch theme afterwards. Still no jump, still no change to `N`.

### Minimal implementation (sketch)

**`src/web/settings.rs`** — the constants, beside the font-size ones:

```rust
pub(crate) const LINE_HEIGHT_MIN: u16 = 100;
pub(crate) const LINE_HEIGHT_MAX: u16 = 200;
pub(crate) const LINE_HEIGHT_STEP: u16 = 10;
```

the field, and a default that is a real typographic choice rather than a round number:

```rust
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) font_size: u16,
    pub(crate) line_height: u16,
}
```

```rust
            font_size: 100,
            line_height: 140,
```

the two steppers, the same shape as `zoom_in` / `zoom_out`:

```rust
    pub(crate) fn looser(&mut self) {
        self.line_height = self
            .line_height
            .saturating_add(LINE_HEIGHT_STEP)
            .min(LINE_HEIGHT_MAX);
    }

    pub(crate) fn tighter(&mut self) {
        self.line_height = self
            .line_height
            .saturating_sub(LINE_HEIGHT_STEP)
            .max(LINE_HEIGHT_MIN);
    }
```

one formatter, called from both the variable list and the control's label so the two can
never disagree:

```rust
    pub(crate) fn line_height_css(self) -> String {
        format!("{}.{:02}", self.line_height / 100, self.line_height % 100)
    }
```

```rust
        vars.push(("--USER__lineHeight", self.line_height_css()));
```

and the rule, appended to `user_layer`:

```rust
                \nbody, body * {{ line-height: var(--USER__lineHeight) !important; }}",
```

**`src/ui/settings.rs`** — a second control, deliberately a near-copy of the first:

```rust
#[component]
pub(crate) fn LineHeightControl() -> Element {
    let mut settings = use_context::<Signal<Settings>>();
    let leading = settings().line_height_css();

    rsx! {
        div {
            button {
                disabled: settings().line_height <= LINE_HEIGHT_MIN,
                onclick: move |_| settings.write().tighter(),
                "\u{2195}-"
            }
            span {
                style: "padding: 0 0.5rem",
                "{leading}"
            }
            button {
                disabled: settings().line_height >= LINE_HEIGHT_MAX,
                onclick: move |_| settings.write().looser(),
                "\u{2195}+"
            }
        }
    }
}
```

**`src/ui/reader.rs`** — one import and one line in the chrome, next to `FontSizeControl {}`.

That is the whole step. Nothing in `epub.rs`, nothing in the JS, nothing in `nav.rs`.

### Why it works

- **Unitless, not a percentage.** `line-height: 1.4` and `line-height: 140%` compute to the
  same pixels on the element that declares them and then behave *differently on every
  descendant*. A percentage is resolved to a length **where it is declared** and that length
  is what inherits; a unitless number inherits as a *number* and each element multiplies it by
  its **own** font-size. Put `140%` on `body` at 16px and every child inherits `22.4px` of
  leading — including an `<h1>` at 32px, whose lines then overlap. That is why the CSS spec
  recommends unitless line-height on anything that inherits, and why `--USER__fontSize` gets
  to be a `%` while `--USER__lineHeight` must not be. It is also why the integer field is in
  *hundredths* and gets formatted rather than pushed as a number.
- **`{:02}` is the whole reason a formatter exists.** `140 / 100` and `140 % 100` give `1` and
  `40`, and `"{}.{}"` prints `1.40` — correct by luck. `105` gives `1` and `5`, and the same
  format string prints `1.5`, which is a *different setting* that CSS will accept without
  complaint. `{:02}` pads the remainder to two digits, and the test for `105` is there because
  `140` can never catch it. (`format!("{:.2}", f64::from(self.line_height) / 100.0)` is the
  same output through floats; the integer-division form keeps the type honest — the field is
  a count of hundredths, not a measurement.)
- **`body *` beats inheritance, and beats the author too.** Inheritance is the weakest way a
  property gets a value: it only applies when *no* declaration matched the element. Books
  declare `line-height` on `p` constantly, so a rule on `body` alone is silently ignored
  exactly where it matters. `body *` matches the paragraph itself, which puts us in the
  cascade instead of underneath it. Against `p { line-height: 1.2 !important }` the two
  declarations tie on origin (both author), tie on `!important`, and tie on specificity
  (`body *` and `p` are both one element selector — `*` contributes nothing) — so it comes
  down to **document order**, and the USER layer is injected last, before `</head>`, after the
  book's stylesheets *and* after `pagination.css`. That ordering was Step 2's real finding and
  this is the first step to lean on it for something other than colour.
- **The live route needed no work.** `css_vars` is the one list both routes read, so the
  serve-time `:root` block and the `ook-set-theme` push both pick the new variable up from the
  single `vars.push`. 5c's handler then re-measures and re-anchors because it reacts to *any*
  variable change, not to font-size specifically. A field, a rule, a control — that was the
  claim in the sub-plan, and this is it holding.
- **`let leading = …` before the `rsx!`, not inside it.** Reading `settings()` in the
  component body subscribes this component to the signal, so the label re-renders when the
  value changes either way. Binding it first also sidesteps calling a method inside an `rsx!`
  format hole, which is a different (and fussier) piece of macro machinery than a plain
  identifier.

### Scope note

- **Margins and line-length are 5e**, font-family 5f. Line-length is the one that has to
  reach `column-width` in `pagination.css` — it changes the *page geometry*, not just the
  text inside it — so it earns its own sitting rather than riding along here.
- **Headings get the setting too.** `body *` is a hammer; Readium excludes a handful of
  elements from its line-height override to protect deliberate display type. Unitless keeps
  that from being *broken*, but it is still an override the author did not ask for. If it
  looks wrong on a real book, that is 5f's "respect author intent" conversation arriving
  early, and worth noting rather than fixing here.
- **Two controls that are the same control.** `FontSizeControl` and `LineHeightControl` differ
  in four identifiers. Leave the duplication standing for this sitting — a shared `Stepper`
  wants to see both concrete cases first, and **Step 6** is where it gets written.
- **Settings still live in a `use_signal` in `main.rs`** and reset on every launch. Persisting
  them is real work (a second table beside `positions`) and belongs to its own step, not to
  the sitting that adds the third one.

### Departures and notes from the sitting

- **The colour rule got widened before it got split back.** The first implementation folded
  the leading into the existing colour rule and changed its selector to `body, body *`
  — one rule instead of two, which reads as the tidier edit. It is not: `background` is a
  *shorthand* and it was already `!important`, so every element in the book started painting
  an opaque theme ground and dropping its own `background-image` on the way past. A `<mark>`
  highlight, a striped table, a tinted `<blockquote>` or code block all flatten into the page
  with no declaration the author can write to win them back. The leading is the only one of
  the three properties that needs to reach descendants, because it is the only one whose
  inherited value loses to a rule books actually write. Split into three rules, which is what
  the sketch above had.
- **That is the second time in this phase the same mistake wore the same disguise.** 5b moved
  the colour rule up to `html` alongside the size and lost its `!important` to inheritance;
  5d pushed it down to every descendant and beat declarations it should not have. Both edits
  looked like removing a duplicated selector. The rule to take forward: in this layer the
  selector *is* the design, and two rules that share declarations are not evidence they should
  share a selector.
- **The suite was green for both arrangements, again.** The layer tests assert each variable
  is declared and read *somewhere*; they have never asserted which elements a rule reaches.
  That is the same honest limit 5b recorded, and this time it got a test rather than a note:
  `the_background_rule_stops_at_the_body` walks the layer rule by rule and fails any
  descendant selector that also declares a background. It was confirmed **red** against the
  merged form before being kept — a tripwire nobody has seen fail is a guess about what the
  code does.
- **`!important` on the leading was in the sketch and missing from the first pass.** Without
  it the rule still wins the common case, since `body *` and `p` tie on specificity and the
  layer is injected last — which is exactly what makes the omission hard to see. It loses
  only to an author `!important`, on the books most likely to have opinions about leading.
- **The `dx serve` walk was the real gate and passed by hand**: the lines opened, `N` changed,
  the words held, headings scaled their own leading, and a theme switch afterwards still moved
  nothing. 5c's handler did all of that without a line written for this step, which was the
  claim the sub-plan made about 5d and the reason it was worth checking rather than assuming.

---

## Interlude — the settings popover

> **Status:** done — committed in `b0e10db` (72 tests green, unchanged; clippy clean).
> **Not a planned step.** It is chrome, not cascade: no variable was added, no rule changed,
> and the test count did not move. Recorded here so the build log has no gap between 5d and 5e.

Three controls sitting side by side in the reader header — leading, size, theme — had grown
wider than the page they were meant to leave room for. They moved behind a single gear button
built on `dioxus-primitives`' popover, wrapped in `src/components/popover` so the class names
attach in one place. The close button lost its word for an icon at the same size.

Worth carrying forward into **Step 6**: `.icon-button` in `assets/main.css` and
`.dx-popover-trigger` in the popover's sheet are now the same forty-pixel circle declared
twice, because one is a primitive's trigger and one is not. And the gear's SVG writes
`"view-box"` as a raw string attribute where the close icon writes `view_box:` — the raw form
is emitted literally and SVG ignores it, so that icon has no viewport and renders correctly
only because its `width`/`height` happen to match its path coordinates.

---

## Step 5e — `--USER__pageMargins`

> **Status:** done — committed in `18b42c2` (**75 tests green**, 72 → 75; clippy clean).
> Suggested 2026-08-08 after `b0e10db`; predicted 72 → **76**, and the arithmetic was off by
> one — of the "four tests" below only two are *new*, the other two being amendments to
> existing ones. The third new test (`the_page_margins_step_and_clamp`) was added by analogy
> with 5b and 5d, which is what makes it 75 rather than 74.
>
> **A test the step did not anticipate went red:**
> `injects_pagination_css_before_head_close` asserted the literal old spelling
> `column-width: calc(100vw`. It is a Phase 3-era test whose real subject is "the sheet
> reaches the head," so it was retargeted to `var(--ook-column)` rather than taught the new
> expression — the geometry itself is tested next door, and pinning it twice would mean two
> places to edit at 5f.
>
> **The `dx serve` walk was the gate and passed by hand.** The tests prove the numbers derive
> from one variable; only the walk proves the page still lands where it should, and the drift
> this step is designed around only becomes visible some distance into a chapter.
>
> **Written differently from every step before it:** the implementation was written by Claude
> at the user's explicit request, not by hand, so the usual split does not apply — the tests
> and the code have the same author this once. Two of the tests were watched failing on their
> assertions before the CSS existed (the geometry test, and the pairing test with *"Day
> declares --USER__pageMargins and no rule reads it"*); the other three were only ever seen as
> compile errors, so their assertions were confirmed live by mutating the expected values and
> watching them go red.

The first setting that changes the *shape of the page* rather than the text on it. It is also
the first one whose value is consumed by a stylesheet that is not `user_layer()` — which is
what makes it more than a fourth copy of 5b.

### The crux: three numbers that are secretly one

`pagination.css` today hard-codes the page geometry:

```css
body {
  padding: 24px !important;
  column-width: calc(100vw - 48px) !important;
  column-gap: 48px !important;
  transform: translateX(calc(var(--ook-page) * -100vw));
}
```

Those are not three independent numbers. Paging works because **one column plus one gap
advances exactly `100vw`** — `(100vw - 48px) + 48px` — which is the step the `transform`
moves by and the unit `pageOf()` divides by:

```js
function pageOf(el) {
  return Math.round(el.offsetLeft / window.innerWidth);
}
```

Change the margin naively — bump the padding to `36px` and stop — and the content box narrows
while the gap does not, so the advance stops being `100vw`. The transform then under-shoots a
little more on every page, and `pageOf` disagrees with it by a growing amount. The bug does not
show up on page 1; it shows up on page 20, which is the worst kind.

So the step is not "make the padding a variable." It is: **derive all three from one column
width**, and let the invariant be the thing that is written down.

```
column   = 100vw - 2·gutter·margins
padding  = (100vw - column) / 2        ← half the leftover, per side
gap      = 100vw - column              ← all of the leftover
advance  = column + gap = 100vw        ← unchanged, by construction
```

### The second crux: the variable is read by a sheet that does not declare it

Every `--USER__*` variable so far has been declared by `user_layer()` and applied by a rule in
`user_layer()`. This one is declared by the layer and read by `pagination.css` — a *different*
stylesheet, injected *earlier*, as part of the compile-time `INJECTED_ASSETS` const.

That works, and it is worth being clear about why: source order decides which **declaration**
of a custom property wins, but it does not limit who may **read** one. `pagination.css` sits
ahead of the layer, so its `:root` loses the value — which is exactly right, since the layer's
`:root` is the one carrying the user's choice. Substitution then happens at computed-value
time on whatever won. This is the `--ook-page` trick from Phase 5 pointed the other way:
there, JS writes and CSS reads; here, the served layer writes and CSS reads.

It also means **no new message and no new JavaScript**. `theme-listener.js` already writes
every pushed pair onto `documentElement` as an inline style, re-reports the count, and posts
`ook-reflow` if the anchor moved column. A margin change re-columns the document exactly like
a font-size change does, so 5c's handler carries this one for free — the third time that step
has paid out.

### Runnable check (`cargo test`)

Four tests. The interesting one is the third, because it is **an existing test that this step
breaks**, and understanding why it breaks is most of the lesson.

**1 — the factor reaches the layer bare.** In `src/web/settings.rs`'s `mod test`:

```rust
#[test]
fn the_page_margins_reach_the_layer_as_a_bare_factor() {
    let settings = Settings {
        page_margins: 150,
        ..Settings::default()
    };

    assert!(settings
        .css_vars()
        .contains(&("--USER__pageMargins", "1.50".to_string())));

    // No unit. The value is a multiplicand inside `calc(2 * 24px * m)`; give it a
    // unit and that product is an area, the calc is invalid, and the declaration
    // falls back to its initial value — which for `padding` is 0, i.e. no margin
    // at all on the setting that exists to add one.
    assert!(
        settings.vars().contains("--USER__pageMargins: 1.50;"),
        "the chosen margin never reached the :root block",
    );
}
```

**2 — the geometry stays in step.** In `src/web/assets.rs`'s `mod test`, next to the other
`INJECTED_ASSETS` tripwires:

```rust
#[test]
fn the_page_geometry_derives_from_one_column_width() {
    // padding, column-width and column-gap are one number wearing three hats: the
    // column plus one gap has to advance exactly 100vw, because that is the step
    // `translateX(calc(var(--ook-page) * -100vw))` moves by and the unit `pageOf`
    // divides by. Deriving all three from `--ook-column` is what keeps them in step
    // when the margin moves; three separate expressions would drift silently.
    assert!(
        INJECTED_ASSETS.contains("--ook-column:"),
        "no derived column width — the geometry is still three loose numbers",
    );
    assert_eq!(
        INJECTED_ASSETS.matches("var(--ook-column)").count(),
        3,
        "padding, column-width and column-gap each derive from the column, \
         or one of them is still hard-coded",
    );
}
```

**3 — the pairing test has to learn about the second reader.** This one is already in
`src/web/settings.rs` and it will go **red** the moment `--USER__pageMargins` is pushed. Its
forward half demands that every pushed variable is read by `user_layer()`, and this variable
is read by `pagination.css`. Widen the *reader set*, and only that half:

```rust
#[test]
fn the_pushed_vars_and_the_injected_layer_name_the_same_variables() {
    for theme in [Theme::Day, Theme::Sepia, Theme::Night] {
        let settings = Settings {
            theme,
            ..Settings::default()
        };
        let layer = settings.user_layer();
        // A pushed variable must be read by *something the document gets*, which is no
        // longer only the layer: the geometry rules live in pagination.css, served
        // ahead of the layer and reading the value the layer sets.
        let readers = format!("{layer}{INJECTED_ASSETS}");

        // Nothing pushed that the served layer never declares …
        for (name, value) in settings.css_vars() {
            assert!(
                layer.contains(&format!("{name}: {value};")),
                "{theme:?} pushes {name}, which the injected layer never declares",
            );
            assert!(
                readers.contains(&format!("var({name})"))
                    || readers.contains(&format!("var({name},")),
                "{theme:?} declares {name} and no rule reads it",
            );
        }

        // … and nothing the *layer* reads that no message will ever set. This half
        // stays narrow on purpose — see below.
        for reference in layer.split("var(").skip(1) {
            let name = reference.split(')').next().expect("var( … ) closes");
            assert!(
                settings
                    .css_vars()
                    .iter()
                    .any(|(pushed, _)| *pushed == name),
                "the layer reads {name}, which the message never sets — \
                 that variable would only ever update on a chapter turn",
            );
        }
    }
}
```

Two details in that diff earn their keep:

- **The `var({name},` arm.** `pagination.css` writes `var(--USER__pageMargins, 1)` — a
  fallback, for the reason in the implementation below — so the exact substring `var(NAME)`
  is not present. Without this arm the test fails for a reason that has nothing to do with
  what it is checking.
- **The reverse half must *not* be widened.** It scans `layer` only, and it should keep doing
  so. `INJECTED_ASSETS` reads `--ook-page`, `--RS__pageGutter` and `--ook-column`, none of
  which the message sets and none of which should be — they are internal, not settings. Point
  the reverse scan at `readers` and the test starts demanding the app push variables it has no
  business owning.

**4 — bump the count.** `the_settings_variable_list_carries_the_whole_palette` asserts
`theme.css_vars().len() + 2`; it becomes `+ 3`, and its message grows a name. That assertion
is deliberately annoying: it is the thing that makes you come back and read this list.

Run it: **1, 2 and 4 fail to compile** (no `page_margins` field), **3 fails on the assert**
once the field exists. Then write the implementation.

### Minimal implementation (sketch)

**`src/web/settings.rs`** — the same shape as `line_height`, hundredths again:

```rust
pub(crate) const PAGE_MARGINS_MIN: u16 = 50;
pub(crate) const PAGE_MARGINS_MAX: u16 = 200;
pub(crate) const PAGE_MARGINS_STEP: u16 = 25;
```

a `page_margins: u16` field defaulting to `100`, a `wider`/`narrower` pair matching
`looser`/`tighter`, one more `vars.push` in `css_vars`, and a formatter that is
`line_height_css` with a different field behind it.

**`src/web/assets/reading-system.css`** — the base gutter is a reading-system default, so it
belongs in the RS layer beside `--RS__maxMediaWidth`:

```css
:root {
  --RS__maxMediaWidth: 100%;
  --RS__pageGutter: 24px;
}
```

**`src/web/assets/pagination.css`** — the body rule stops carrying numbers:

```css
:root {
  --ook-page: 0;
  --ook-column: calc(100vw - 2 * var(--RS__pageGutter) * var(--USER__pageMargins, 1));
}
body {
  padding: var(--RS__pageGutter) calc((100vw - var(--ook-column)) / 2) !important;
  column-width: var(--ook-column) !important;
  column-gap: calc(100vw - var(--ook-column)) !important;
}
```

(the other declarations in that rule are unchanged)

**`src/ui/settings.rs`** — a `PageMarginsControl` alongside the other two, added to the
popover's column.

### Why it works

- **The invariant is now a consequence, not a coincidence.** `column-gap` is *defined* as the
  leftover and `padding` as half of it, so `column + gap` reduces to `100vw` for any margin
  the control can produce. There is no value of `--USER__pageMargins` that can put the
  transform and `pageOf` out of agreement, because neither of them is mentioned in the change.

- **`var(--USER__pageMargins, 1)` is a fuse, not a default.** A `var()` naming an undefined
  custom property makes the whole declaration *invalid at computed-value time* — not ignored,
  which would leave the previous cascade winner, but reset to the property's initial value.
  For `column-width` that is `auto` and for `padding` it is `0`: pagination collapses into one
  tall scrolling column. The real default lives in `Settings::default()`, and the layer is
  always injected, so the fallback should never fire — which is exactly why it is worth
  writing, since the failure it guards is total and silent.

- **The gutter is `--RS__*` and the factor is `--USER__*`, and that split is the point.** The
  reading system says how wide a comfortable margin is *once*; the reader says how much of one
  they want. Multiplying them means the setting is device-independent — the same "1.5" is 36px
  everywhere — and it keeps a single place to change if the base ever becomes responsive.

- **Vertical padding stays fixed at the gutter.** `--USER__pageMargins` is about line length,
  which is a horizontal quantity; Readium scopes it the same way. Scaling the vertical margin
  too would work and would re-anchor fine, but it changes the column *height* and therefore
  how many lines fit a page, which is a different setting wearing this one's name.

- **`Math.round` in `pageOf` absorbs the padding, and there is a limit to that.** `offsetLeft`
  for content in column *n* is roughly `padding-left + n·100vw`, so the ratio is
  `n + padding/innerWidth` and rounding recovers `n` only while that fraction stays under a
  half. At the `200` ceiling on a 375px phone the padding is 48px — about 13%, comfortably
  clear. It is the reason the ceiling is a small number rather than "as wide as you like."

### Scope note

- **Line length is 5f, not this step.** Once `--ook-column` exists, capping the measure is one
  `min()` inside its definition and everything downstream follows — which is precisely why it
  deserves its own sitting rather than being free-ridden here: its real content is the *unit*
  (`ch` couples it to `--USER__fontSize`; `rem` does not) and what happens on a wide desktop
  window, where a bare margin factor leaves an unreadably long line. Splitting the pair pushes
  font-family to **5g**.
- **A third `{}.{:02}` formatter.** `line_height_css` and `page_margins_css` will differ in one
  field name. Same call as 5d made about the two controls: leave it, and let **Step 6** see
  three concrete cases before it writes the shared helper.
- **No new JavaScript, and that is a claim to check rather than assume.** 5d made the same
  claim about 5c's handler and it held; verify it again under `dx serve` before believing it.
- **`dx serve` is the real gate here.** The tests can prove the numbers derive from one
  variable; they cannot prove the page still lands where it should. Walk it: change the margin
  mid-chapter and confirm the words hold, `N` updates, then page forward ten or twenty pages
  and confirm the text is still framed rather than creeping off the edge — the drift bug this
  step is designed around only becomes visible some distance in.

---

## Step 5f — `--USER__maxLineLength`

> **Status:** done — committed in `fb7304f` (**78 tests green**, 75 → 78 as predicted;
> clippy clean). Suggested 2026-08-09 after `18b42c2`, and landed exactly as sketched — the
> first step of this phase where the plan and the build agreed on every point.
>
> **The prediction that mattered held.**
> `the_page_geometry_derives_from_one_column_width` stayed green **without being touched**.
> Capping changed what `--ook-column` holds, not how many rules derive from it, and that it
> needed no edit is the cheapest available proof that 5e's design absorbed this step.
>
> **The unit correction was the step's real content**, and it was found by checking the
> plan's premise against MDN rather than building on it. `rem` *is* the root font-size —
> exactly what `--USER__fontSize` sets — so the plan's stated reason for preferring `ch` did
> not survive contact. `ch` still wins, on the font-*family* argument the plan never made.
>
> **The `dx serve` walk was the gate and passed by hand**, wide window included: the text
> stopped growing and centred, twenty pages forward held their advance with the gap larger
> than it has ever been, a narrow window was unchanged, and a font-size change kept the
> characters per line rather than the physical width. 5c's handler re-anchored it with no new
> JavaScript, for the fourth time — verified rather than assumed, as 5d and 5e both insisted.
>
> **Implementation written by Claude at the user's request**, as with 5e, so the usual
> learner-writes-the-code split does not apply. Three of the four tests were watched failing
> on real assertions before the implementation existed (`the_measure_caps_the_column_alone`,
> `the_measure_reaches_the_layer_in_characters`, and the `+ 3` → `+ 4` tripwire firing on
> schedule for the third time); `the_measure_steps_and_clamps` went green on its first run,
> so its assertion was confirmed live by inverting the expected value and watching it fail.

5e made the page geometry derive from one number. This step puts a **ceiling** on that number,
and the whole implementation is one `min()` in one place. The interesting content is not the
change — it is the **unit**, and a claim in the plan that turns out to be wrong.

### The crux: a margin factor cannot fix a wide window

`--USER__pageMargins` scales a *fixed* 24px gutter. On a 1400px-wide window at the `200`
ceiling the column is still `1400 - 96 = 1304px` — about 160 characters at a normal size,
roughly twice a readable measure. The margin control cannot reach it, because it was never
about the measure: it multiplies a constant, and the constant is small.

So the cap has to be an **absolute** quantity, not a proportion of the viewport. That is a
different kind of setting from every one before it, and it is why 5e's note said this deserved
its own sitting rather than a free ride.

### The second crux: the plan's premise about the unit is wrong

The phase doc has said since 5e that the content of this step is the unit, because "`ch`
couples it to `--USER__fontSize`, `rem` does not." Checked against MDN, **that is not true**,
and the real distinction is more interesting.

- **`rem` is the root element's font-size.** `--USER__fontSize` is applied as
  `html { font-size: var(--USER__fontSize) !important; }` — that *is* the root's font-size, so
  `rem` tracks it exactly. Set 125% and `1rem` goes from 16px to 20px.
- **`ch` is the width of the `0` glyph in *the element's font*** — a *local* font-relative
  length, in the same family as `em` and `ex`, not a root-relative one like `rem`.

So both units scale with font size, and the plan's stated reason for preferring `ch`
evaporates. `ch` is still the right choice, for a reason the plan did not name: it is
sensitive to the **font family**, and `rem` is blind to it. A condensed face fits more
characters into the same width; `66ch` narrows to match, `41rem` does not. `ch` holds the
measure constant in *characters* — which is the thing a measure is actually specified in —
across both settings that can change it, and the second of those settings is **5g**.

There is a loose end here, and it should be named rather than assumed. `ch` resolves against
"the element's font," and `--ook-column` is *declared* on `:root` but *read* by
`column-width` on `body`. Which element's font a `ch` inside a custom property resolves
against is exactly the sort of thing to confirm on screen rather than reason about — and it
does not become observable until `body` and `html` have *different* fonts, which is 5g's
doing, not this step's. Both elements carry the same font today, so 5f cannot tell the
difference. **Recorded as 5g's problem**, with a note there to re-check it.

### Runnable check (`cargo test`)

Three new tests and one tripwire bump: **75 → 78**.

**1 — the measure reaches the layer in characters.** In `src/web/settings.rs`'s `mod test`:

```rust
#[test]
fn the_measure_reaches_the_layer_in_characters() {
    let settings = Settings {
        max_line_length: 66,
        ..Settings::default()
    };

    assert!(settings
        .css_vars()
        .contains(&("--USER__maxLineLength", "66ch".to_string())));
    assert!(
        settings.vars().contains("--USER__maxLineLength: 66ch;"),
        "the chosen measure never reached the :root block",
    );

    // The unit is the whole decision. `px` would pin the measure to a physical
    // width, so raising the font size would cut the characters per line. `rem`
    // tracks the root font-size — so it survives a size change — but it is blind
    // to the font *family*, which 5g is about to make user-settable. `ch` is the
    // width of a `0` in the font actually in use, so it is the only one of the
    // three that keeps the measure constant in characters under both settings.
    let (_, value) = settings
        .css_vars()
        .into_iter()
        .find(|(name, _)| *name == "--USER__maxLineLength")
        .expect("the measure is pushed");

    assert!(value.ends_with("ch"), "the measure is in {value}, not characters");
}
```

**2 — the cap enters through the column and nowhere else.** In `src/web/assets.rs`, next to
`the_page_geometry_derives_from_one_column_width`. This is the test that carries the step:

```rust
#[test]
fn the_measure_caps_the_column_alone() {
    let column = INJECTED_ASSETS
        .split_once("--ook-column:")
        .and_then(|(_, rest)| rest.split_once(';'))
        .map(|(value, _)| value)
        .expect("pagination.css declares the derived column");

    assert!(
        column.contains("min("),
        "the measure has to cap the column, and a cap is a min()",
    );
    assert!(
        column.contains("var(--USER__maxLineLength"),
        "the column ignores the measure entirely",
    );

    // One reference, in the column's own definition. Cap the padding or the gap
    // separately and they stop being "the leftover" — the advance stops being
    // 100vw and the transform drifts from `pageOf` again, which is the exact bug
    // 5e was built to make unreachable.
    assert_eq!(
        INJECTED_ASSETS.matches("var(--USER__maxLineLength").count(),
        1,
        "the cap belongs in one place — every other number derives from it",
    );
}
```

**3 — the measure steps and clamps**, the same shape as `the_page_margins_step_and_clamp`.

**4 — the tripwire.** `the_settings_variable_list_carries_the_whole_palette` goes from
`theme.css_vars().len() + 3` to `+ 4`, and its message grows a name. Third time it has fired
on schedule.

**And one prediction worth checking:** `the_page_geometry_derives_from_one_column_width` must
stay green **without being touched**. It counts three `var(--ook-column)` references, and
capping the column changes what that variable *holds*, not how many rules derive from it. If
that test needs editing, the cap went somewhere it should not have. That it survives is the
payoff of 5e's design, and it is the cheapest possible proof of it.

### Minimal implementation (sketch)

**`src/web/settings.rs`** — the same shape as the last three settings, but the field is a
plain character count, so there is no `{}.{:02}` formatter this time:

```rust
pub(crate) const MAX_LINE_LENGTH_MIN: u16 = 45;
pub(crate) const MAX_LINE_LENGTH_MAX: u16 = 100;
pub(crate) const MAX_LINE_LENGTH_STEP: u16 = 5;
```

a `max_line_length: u16` field defaulting to **70**, a `longer`/`shorter` pair, and one more
`vars.push` with `format!("{}ch", self.max_line_length)`.

**`src/web/assets/pagination.css`** — the only change, and it is one line:

```css
:root {
  --ook-page: 0;
  --ook-column: min(
    100vw - 2 * var(--RS__pageGutter) * var(--USER__pageMargins, 1),
    var(--USER__maxLineLength, 100ch)
  );
}
```

`min()` is itself a math function, so its arguments are calc-expressions already — the inner
`calc()` wrapper is optional and dropping it reads better. **Nothing else in the file
changes**, which is the point.

**`src/ui/settings.rs`** — a `MaxLineLengthControl` beside the other three.

### Why it works

- **`min()` is the cap, and the leftover machinery does the rest.** `padding` is defined as
  half of `100vw - column` and `column-gap` as all of it, so shrinking the column *widens both
  automatically* and `column + gap` is still exactly `100vw`. On a wide window the extra space
  becomes symmetric margin and the text sits centred — with no rule anywhere mentioning
  centring. This is what "derive everything from one number" bought, and 5f is where it gets
  collected.
- **A cap, not a width.** `min()` means the setting only ever *binds* — on a phone the
  viewport branch is smaller and wins, so narrow screens are untouched and a default of `70`
  changes nothing there. A setting that can only remove space is much harder to misuse than
  one that sets it outright.
- **`var(--USER__maxLineLength, 100ch)` is a fuse, same as 5e's.** An undefined custom
  property invalidates the whole declaration at computed-value time, which would reset
  `column-width` to `auto` and collapse pagination into one tall scroll. The fallback is
  deliberately *generous* rather than sensible: it should never fire, and if it does, a wide
  column is a far better failure than a broken pager.
- **Characters are the unit a measure is specified in.** Typographic advice — 45–75
  characters — is in characters, so a control in characters is a control in the same units
  as the recommendation, and the numbers on screen mean something. The `u16` is the count;
  the `ch` is only how CSS is told about it.
- **Why 70 by default.** Inside the classic 45–75 band, and wide enough that a laptop window
  is capped noticeably while a tablet in portrait mostly is not — so the setting announces
  itself on the device where the problem is real.
- **Nothing new in the transport, for the fourth time.** `theme-listener.js` writes every
  pushed pair onto `documentElement`, re-reports the count and re-anchors. A cap re-columns
  the document exactly like a margin change, so 5c carries this too — a claim to *verify*
  under `dx serve`, not assume, which is what 5d and 5e both said and both were right about.

### Scope note

- **No "off" switch.** The ceiling can be raised to 100 characters, which on any real window
  is not binding; a tri-state control (on / off / value) is a UI question, not a cascade one.
- **Horizontal only**, like 5e. Nothing here touches how many lines fit a page.
- **The `ch`-resolution question is 5g's**, as above — flagged there, not answered here.
- **`dx serve` is the gate, and this one needs a wide window.** Widen the window past roughly
  a laptop's width: the text should stop growing and centre instead. Then page forward twenty
  pages *in that wide window* — with the cap binding, the gap is much larger than it has ever
  been, so if the advance invariant were going to break, this is the configuration that would
  show it. Then check a narrow window is unchanged, and that a font-size change keeps roughly
  the same number of characters per line rather than the same physical width.

---

## Step 5g — `--USER__fontFamily` from a curated list

> **Status:** done — committed in `cb03a4b` **together with 5h** (**89 tests green**,
> 78 → 83 → 89; clippy clean). **The plan's single `5g` was split here into two steps**,
> because it was carrying two unrelated ideas: a font stack is a value like every setting
> before it, but *respecting the publisher's font* is a rule that has to appear and disappear,
> and a variable push cannot carry a rule. 5g is the value; [5h](#step-5h--respect-the-publishers-font-the-gate)
> is the gate.
>
> **One commit for two steps, the way Steps 1 and 2 landed in `27e1d86`.** 5g on its own
> ships the deliberate regression its scope note describes — every book's font overridden,
> embedded faces included — and 5h exists to undo it. Both were written in one sitting, so
> there was no green intermediate worth preserving in the history. 5g was run on its own
> first (**83 tests green**, the predicted count exactly) before 5h was started.
>
> **The five planned tests landed unchanged**, and the `+ 4` → `+ 5` tripwire fired on
> schedule for the fourth time.
>
> **The `dx serve` walk was confirmed by the user.** The plan's contingency — "if the physical
> column width does not move between Old Style and Sans, add `html` to the font rule's
> selector" — was not triggered, so the selector stayed at `body` and 5f's reasoning about
> `ch` resolving at the *use* site stands unrefuted.
>
> **Implementation written by Claude at the user's request**, as with 5e and 5f, so the usual
> learner-writes-the-code split does not apply. The tests were verified by mutating the
> implementation — the `:not(pre)` exclusion removed, the `+ 5` reverted to `+ 4` — and
> watching each test go red, then restoring from backup and re-confirming 89 green.

Five settings in, the shape is familiar: a field, a variable, a rule, a control. This one is
familiar right up to the point where it isn't. Two things are new — the value is a **list with
internal punctuation** rather than a number with a unit, and the setting is the first that
changes what a `ch` *is*, which is the loose end 5f left tied to this step.

### The crux: a font stack is a fallback chain, and the last link is the only guaranteed one

`font-family` does not name a font, it names a *sequence* of candidates, and the browser walks
it until one is installed. `Iowan Old Style` exists on macOS and nowhere else; `Charter` ships
with some systems and not others. If every named face misses, the declaration still applies —
it just resolves to whatever the UA default is, which on this reader is the same font the book
was already showing. The setting would appear to do nothing, on exactly the machines you did
not test on.

So each stack ends in a **generic family** — `serif` or `sans-serif`. That is the link that
cannot miss, and it is what makes "Sans" mean *sans* on a machine that has none of the four
faces you named. Curating a font list is really curating four fallback chains, and the
interesting design is at the end of each one, not the start.

### The second crux: `ch` is the width of a `0` in *the element's* font

5f chose `ch` for the measure over `rem` on the grounds that it tracks the font *family* as
well as the size, and then flagged the thing it could not check: `--ook-column` is declared on
`:root`, and `column-width: var(--ook-column)` is read on `body`. If a `ch` inside a custom
property resolved where the property is **declared**, the measure would track `html`'s font;
if it resolves where the property is **used**, it tracks `body`'s.

Until now both elements carried the same font, so the two readings were indistinguishable.
This step makes them differ — the override lands on `body` and its descendants, and `html`
keeps the UA font — which is what finally makes the question observable. The spec's answer is
that an unregistered custom property's value is a *token sequence*: `70ch` is not a length
while it sits in the variable, it becomes one only when substituted into `column-width` on
`body`. So it should track `body`. **That is a prediction, and this step's `dx serve` walk is
where it gets checked** — see the scope note for what to do if it comes back the other way.

### Runnable check (`cargo test`)

Five new tests, and one existing tripwire that fires on schedule for the fourth time. Write
them in `src/web/settings.rs`'s `mod test` (and the enum's own round-trip beside it), watch
them fail, then implement.

```rust
#[test]
fn the_font_family_reaches_the_layer_as_a_stack() {
    let settings = Settings {
        font_family: FontFamily::Sans,
        ..Settings::default()
    };

    let stack = FontFamily::Sans.stack();

    assert!(settings
        .css_vars()
        .contains(&("--USER__fontFamily", stack.to_string())));

    let layer = settings.user_layer();

    assert!(
        layer.contains(&format!("--USER__fontFamily: {stack};")),
        "the chosen face never reached the :root block",
    );
    assert!(
        layer.contains("font-family: var(--USER__fontFamily)"),
        "the layer declares a family it never applies — the picker would move \
         and the text would not",
    );
}

#[test]
fn every_stack_ends_in_a_generic_family() {
    // The only link in the chain that cannot miss. Without it, a machine with none
    // of the named faces installed falls back to the UA default — which is the font
    // the book was already showing, so the setting silently does nothing there.
    for family in FontFamily::ALL {
        let stack = family.stack();
        let last = stack.rsplit(',').next().expect("a stack is non-empty").trim();

        assert!(
            matches!(last, "serif" | "sans-serif" | "monospace"),
            "{family:?} ends in `{last}`, which is a face and might not exist",
        );
    }
}

#[test]
fn no_stack_quotes_a_family_with_double_quotes() {
    // The stack does not only travel as CSS. `inline_styles()` puts it in a `style="…"`
    // attribute on the reader's own chrome, where a `"` closes the attribute early and
    // takes the rest of the declarations with it. Single quotes are legal CSS and have
    // no such second job.
    for family in FontFamily::ALL {
        assert!(
            !family.stack().contains('"'),
            "{family:?} quotes with `\"`, which cannot survive an HTML attribute",
        );
    }
}

#[test]
fn the_monospace_elements_keep_their_own_font() {
    // `code`, `kbd`, `samp` and `pre` are monospace because the *content* is
    // column-aligned, not because the author had a taste in fonts. Overriding them
    // with a proportional face is how a code sample or an ASCII table stops being
    // readable — a change nobody asked for and nobody can undo.
    let layer = Settings::default().user_layer();
    let rule = layer
        .split('\n')
        .find(|rule| rule.contains("font-family:"))
        .expect("the layer applies the family");

    for tag in ["code", "kbd", "samp", "pre", "var"] {
        assert!(
            rule.contains(&format!(":not({tag})")),
            "the family lands on <{tag}>, whose font is structural",
        );
    }
}

#[test]
fn a_font_family_survives_a_slug_round_trip() {
    // Step 6 stores the choice as this slug. A variant that does not come back is a
    // setting that silently resets to the default on the next launch.
    for family in FontFamily::ALL {
        assert_eq!(FontFamily::from_slug(family.slug()), family);
    }

    assert_eq!(FontFamily::from_slug("comic-sans"), FontFamily::default());
}
```

**The tripwire that must fire.** `the_settings_variable_list_carries_the_whole_palette`
asserts `theme.css_vars().len() + 4`. It goes to `+ 5`, and — as in 5b, 5d, 5e and 5f — you
should watch it *fail first*. It is the only thing standing between "I added a field" and "I
added a field and forgot to push it".

**The tripwire that must *not* fire.**
`the_pushed_vars_and_the_injected_layer_name_the_same_variables` should stay green untouched:
the new variable is pushed, declared in the `:root` block, and read by a rule in the layer, so
both halves of it are already satisfied. If it goes red, the field reached `css_vars()` and
never reached `user_layer()` — which is precisely the bug it exists to catch.

### Minimal implementation (sketch)

**`src/web/font.rs`** — a new module, sibling to `theme.rs`, for the same reason `Theme` got
one: a closed set of named choices with a rendering and a slug is its own thing, and
`settings.rs` is already the longest file in the phase.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FontFamily {
    #[default]
    OldStyle,
    Modern,
    Sans,
    Humanist,
}

impl FontFamily {
    pub(crate) const ALL: [FontFamily; 4] = [
        FontFamily::OldStyle,
        FontFamily::Modern,
        FontFamily::Sans,
        FontFamily::Humanist,
    ];

    pub(crate) fn stack(self) -> &'static str {
        match self {
            FontFamily::OldStyle => "'Iowan Old Style', 'Sitka Text', Palatino, Georgia, serif",
            FontFamily::Modern => "Athelas, Charter, 'Bitstream Charter', Cambria, serif",
            FontFamily::Sans => "Seravek, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif",
            FontFamily::Humanist => "Frutiger, Calibri, 'Gill Sans', 'Lucida Grande', sans-serif",
        }
    }

    pub(crate) fn slug(self) -> &'static str { … }      // "old-style", "modern", …
    pub(crate) fn from_slug(slug: &str) -> FontFamily { … }   // unknown → default
}
```

**`src/web/settings.rs`** — a `font_family: FontFamily` field, `FontFamily::default()` in
`Default`, one more push, and one more rule:

```rust
vars.push(("--USER__fontFamily", self.font_family.stack().to_string()));
```

```rust
"…\nbody, body *:not(code):not(kbd):not(samp):not(pre):not(var) \
   {{ font-family: var(--USER__fontFamily) !important; }}"
```

**`src/ui/settings.rs`** — a `FontFamilyPicker`, and it is deliberately a near-copy of
`ThemePicker`: a `<select>`, an option per variant, `selected:` comparing to the current
value, `from_slug` on change. Two `<select>`s that differ only in their enum is exactly the
duplication Step 7 should collapse; write the copy now and let the refactor step see two
instances rather than guess at one.

No change to `pagination.css`, `theme-listener.js` or the push. For the fifth time, the
transport already carries this.

### Why it works

- **`!important` on a descendant selector is what makes it land.** A book that says
  `p { font-family: 'Whatever' }` has declared the property on the paragraph, so an inherited
  value from `body` never reaches it — the same trap 5d hit with `line-height`, and the same
  fix. The specificity is worth reading: each `:not()` argument contributes its own weight, so
  `body *:not(code):not(kbd):not(samp):not(pre):not(var)` is a *heavier* selector than
  anything a typical stylesheet writes, on top of being `!important` and later in document
  order.
- **The exclusions are a statement about content, not taste.** Everything in that `:not()`
  chain is monospace because the characters line up in columns. This is the first sliver of
  "respect author intent" — and it is deliberately the easy half, since the hard half
  (embedded faces, author `!important`) needs the gate that 5h builds.
- **Single quotes, because the value has two destinations.** `inline_styles()` interpolates
  the declarations into a Dioxus `style:` attribute on the reader's chrome; the served
  `:root` block goes into a CDATA-wrapped `<style>`. CSS accepts either quote, HTML attributes
  do not, and picking the one that works everywhere costs nothing. The test is cheaper than
  remembering.
- **A closed enum, not a string.** `from_slug` folding an unknown value to the default is the
  same shape `Theme::from_slug` has had since Step 4, and it is what lets Step 6 store a slug
  without the reader ever being able to boot into a font that no longer exists.

### Scope note

- **No publisher's-font option yet, and that is a real regression for one step.** Every book
  gets its face overridden, embedded fonts included — a book whose alphabet *is* the embedded
  font will render as garbage. This is the same deliberate arrangement as 5b leaving the page
  count stale for 5c to fix: you watch the aggressive override misbehave, then build the gate
  that Readium built for the same reason. **5h is the fix, and it should be the next sitting,
  not a later one.**
- **Faces only — no weight, no `font-variant`, no `text-align`, no hyphenation.** The
  language-sensitive settings the phase doc parks stay parked.
- **`dx serve` is the gate, and it has two things to look at.** First, obviously: switch
  between the four and the text changes face. Second, and this is 5f's loose end — at a fixed
  measure, switch between **Old Style and Sans** and watch the *physical* column width. If it
  moves, `ch` is resolving against `body`, the prediction held, and the measure is genuinely in
  characters under both settings. If it does not move, `ch` is resolving against `:root` — the
  fix is to add `html` to the font rule's selector, and the note in 5f about `ch` beating `rem`
  needs the correction, not the code.
- **A book with an embedded icon font is the interesting one to open.** Not required for this
  step; required to *believe* 5h.

---

## Step 5h — respect the publisher's font (the gate)

> **Status:** done — committed in `cb03a4b` together with 5g (**89 tests green**, 83 → 89;
> clippy clean). Laid out in full at the user's request before 5g was written, and landed
> exactly as sketched — the recommended option was taken at both forks (the serve-time
> bootstrap over the ungated rule, and the empty-value push over omitting the variable).
>
> **The design question inside the tripwire was answered before typing, and the answer held.**
> `declarations()` filters empty values, so the `:root` block simply has no `--USER__fontFamily`
> line under Publisher; `the_pushed_vars_and_the_injected_layer_name_the_same_variables` grew
> the empty-value case rather than losing an assertion, and its second half needed no change —
> the layer still reads `var(--USER__fontFamily)` from inside a rule that does not match.
>
> **`FontFamily::Publisher => ""` was the load-bearing choice.** Every downstream branch —
> the filter in `declarations()`, the early return in `bootstrap_js()`, the skip in
> `every_stack_ends_in_a_generic_family` — falls out of one `is_empty()` rather than a `match`
> in three places.
>
> **All four state transitions were reasoned through before the `dx serve` walk** and the walk
> confirmed them: served-Publisher and served-Sans, each switched live in both directions. The
> one that could have gone wrong is served-Sans → live-Publisher, which is the case the
> rejected "ungated rule at serve time" option would have got wrong.
>
> **Implementation written by Claude at the user's request.** Four of the six tests were
> verified by mutating the implementation and watching them go red — the gate dropped from the
> second selector only, `Publisher` given a real stack, the `declarations()` filter removed,
> and `removeProperty` swapped back to `setProperty` — then restoring from backup and
> re-confirming 89 green.

The half of the plan's `5g` that is not about fonts at all. It is about a limit in the
transport that five settings in a row have been able to ignore.

### The crux: you cannot push a rule through a variable

Every setting so far is always *on*; only its value moves. That is why
`theme-listener.js` has never needed a change — writing a new value onto
`documentElement.style` is enough, because the rule that reads it is already in the served
document and should always apply.

A `Publisher` option breaks that. "Use the book's own font" is not a value of
`font-family` — there is no CSS keyword meaning *the author's declaration*. `revert` rolls
back past the author origin entirely, `unset` on an inherited property means `inherit`, and an
undefined custom property makes the declaration invalid at computed-value time, which is also
`inherit`. All three give you *some other font*, not the book's. The rule genuinely has to
stop matching.

### Readium's answer: gate on the *presence* of the variable

```css
:root[style*='--USER__fontFamily'] body,
:root[style*='--USER__fontFamily'] body *:not(code):not(kbd):not(samp):not(pre):not(var) {
  font-family: var(--USER__fontFamily) !important;
}
```

The rule matches only when that name literally appears in the root element's **inline `style`
attribute** — which is exactly and only what the push writes to. Choose a face, the property
is set and the rule switches on; choose Publisher, the push removes it and the rule switches
off, with the book's own cascade untouched underneath because nothing ever overwrote it.

Two details make that selector work, and neither is obvious:

- **CSSOM writes are reflected into the attribute.** `el.style.setProperty('--x', 'y')`
  does not just mutate an internal object — the `style` *content attribute* re-serializes, so
  `[style*='--x']` starts matching immediately. Custom properties serialize like any other
  declaration. This is the whole reason the trick works at all, and it is the one assumption in
  the step worth confirming in devtools rather than trusting.
- **A substring match is a substring match.** `[style*='--USER__fontFamily']` is safe because
  no other variable name contains it. Had the gate been written on `--USER__font`, it would
  also match `--USER__fontSize`, which is pushed on every launch — the gate would be welded
  open and nothing would ever look wrong until a book with an embedded alphabet turned up.

### The part that will actually bite: the serve path writes to the wrong place

The gate reads the `style` **attribute**; serve-time injection writes a `:root { … }`
**rule**. They are not the same place, so a chapter that has not loaded yet comes up with the
gate closed and the reader's chosen face ignored until the next push. Two candidate fixes:

- **Emit the rule ungated at serve time** when a face is selected. One `if` in Rust, no new
  JavaScript — but a chapter *served* under `Sans` keeps that ungated rule, so switching **to**
  Publisher would not take effect until the chapter turns. A stale-until-you-page bug.
- **Have the served document set its own inline style before paint** — a serve-time
  `<script>` next to the serve-time `<style>`, writing the chosen stack onto
  `documentElement.style` while the head is still parsing. Both routes then write to the same
  place, which is what makes the gate honest.

**Take the second.** The first is smaller and it is the one that will be tempting; it also
re-introduces exactly the class of "the document and the reader disagree until you turn a page"
bug that 5c and 5f went to some trouble to eliminate.

### The design question inside the tripwire

`the_pushed_vars_and_the_injected_layer_name_the_same_variables` has two halves, and Publisher
puts pressure on both. Answer this *before* typing, because both naive options fail:

- **Omit `--USER__fontFamily` from `css_vars()` under Publisher.** Half two — "every `var()`
  the layer reads is something a message can set" — goes red, correctly: the gated rule reads
  it, and if no message can ever set it, choosing a face on a loaded chapter would do nothing
  until the chapter turned.
- **Push it with an empty value and render it anyway.** Half one — "every pushed var is
  declared in the layer" — passes only if the `:root` block emits `--USER__fontFamily: ;`,
  which is not a declaration; the parser drops it, and the served CSS now contains a lie.

The answer is **push the pair always with an empty value, and let an empty value mean "no
declaration" on both sides**: `declarations()` skips it, so the `:root` block simply has no
such line, and `theme-listener.js` calls `removeProperty` instead of `setProperty`. The
tripwire then grows a case rather than losing one — non-empty vars must be declared, empty
vars must be *absent*. That the test needs an edit here is the signal that the transport
gained a genuinely new capability, not that the test was wrong.

### Runnable check (`cargo test`)

Five new tests plus that edit. Watch them fail first — in particular the two that fail for
*interesting* reasons: the gate test fails because 5g's rule is ungated, and the tripwire fails
the moment `Publisher` becomes the default.

```rust
#[test]
fn the_publisher_is_the_default_font() {
    // The whole point of the gate. An override that is on by default is not a gate,
    // it is 5g — and 5g clobbers embedded fonts, which is the regression this closes.
    assert_eq!(Settings::default().font_family, FontFamily::Publisher);
}

#[test]
fn the_publisher_pushes_an_empty_value_and_declares_nothing() {
    let settings = Settings {
        font_family: FontFamily::Publisher,
        ..Settings::default()
    };

    // Pushed, so a *live* switch back to the book's own face reaches the frame …
    assert!(settings
        .css_vars()
        .contains(&("--USER__fontFamily", String::new())));

    // … and not declared, because `--USER__fontFamily: ;` is not a declaration.
    assert!(
        !settings.vars().contains("--USER__fontFamily"),
        "the :root block declares a face under Publisher — the gate would open \
         the moment anything copied that block into an inline style",
    );
}

#[test]
fn every_selector_on_the_font_rule_carries_the_gate() {
    // A selector *list* is the trap: prefixing the gate onto the first selector and
    // not the second leaves `body *` matching unconditionally, and the bug shows up
    // only on the descendants — i.e. on every paragraph, which is all the text there
    // is. Check each comma-separated selector, not the rule.
    let layer = Settings::default().user_layer();
    let rule = layer
        .split('\n')
        .find(|rule| rule.contains("font-family:"))
        .expect("the layer applies the family");
    let (selectors, _) = rule.split_once('{').expect("a rule has a block");

    for selector in selectors.split(',') {
        assert!(
            selector.contains("[style*='--USER__fontFamily']"),
            "`{}` overrides the font unconditionally",
            selector.trim(),
        );
    }
}

#[test]
fn a_chosen_face_reaches_a_chapter_that_has_not_loaded_yet() {
    // The gate reads the inline style; serve-time injection writes a stylesheet rule.
    // Without this bootstrap the first paint of every new chapter ignores the setting.
    let settings = Settings {
        font_family: FontFamily::Sans,
        ..Settings::default()
    };
    let bootstrap = settings.bootstrap_js();

    assert!(bootstrap.contains("setProperty"));
    assert!(bootstrap.contains("--USER__fontFamily"));
    assert!(bootstrap.contains(FontFamily::Sans.stack()));

    // The stack is interpolated into a JavaScript string literal here, which is the
    // second job of 5g's `no_stack_quotes_a_family_with_double_quotes`: a `"` in a
    // stack would close the literal and turn the document into a syntax error.
    assert!(!FontFamily::Sans.stack().contains('"'));
}

#[test]
fn the_publisher_bootstraps_nothing() {
    // Not "sets it to empty" — emits no script at all. There is nothing to undo on a
    // document that was born without the property.
    assert!(Settings::default().bootstrap_js().is_empty());
}
```

And the cross-language tripwire, in `ui/reader.rs`'s `mod test` beside the three that already
do this job:

```rust
#[test]
fn an_empty_pushed_value_removes_the_property() {
    // `setProperty(name, "")` is a no-op, not a removal — the gate would stay open and
    // Publisher would silently do nothing. Rust decides empty means remove; only this
    // string in a JS file honours it, and no compiler sees both.
    assert!(crate::web::assets::INJECTED_ASSETS.contains("removeProperty"));
}
```

**The edit.** In `the_pushed_vars_and_the_injected_layer_name_the_same_variables`, half one
splits on the value:

```rust
for (name, value) in settings.css_vars() {
    if value.is_empty() {
        assert!(!layer.contains(name), "{name} is unset and still declared");
    } else {
        assert!(layer.contains(&format!("{name}: {value};")), …);
        assert!(readers.contains(&format!("var({name})")) || …, …);
    }
}
```

Note the `var()` reachability check moves inside the non-empty branch: under Publisher the
layer still reads `var(--USER__fontFamily)` from inside a rule that does not match, which is
the correct state and not a dangling reference.

Expect **~83 → ~88**.

### Minimal implementation (sketch)

**`src/web/font.rs`** — one more variant, and it takes the `#[default]`:

```rust
pub(crate) enum FontFamily {
    #[default]
    Publisher,
    OldStyle,
    …
}
```

`stack()` returns `""` for it, `slug()` returns `"publisher"`, and `ALL` grows to five. The
empty stack is what makes every downstream decision fall out of one `is_empty()` rather than a
`match` in four places — 5g's `every_stack_ends_in_a_generic_family` needs to skip it, and
saying so with `.filter(|f| !f.stack().is_empty())` reads better than an exception list.

**`src/web/settings.rs`** — three changes:

```rust
fn declarations(self) -> String {
    self.css_vars()
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| format!("{name}: {value};"))
        …
}
```

the gate on the font rule (both selectors), and a new method:

```rust
pub(crate) fn bootstrap_js(self) -> String {
    let stack = self.font_family.stack();
    if stack.is_empty() {
        return String::new();
    }
    format!(
        "document.documentElement.style.setProperty(\"--USER__fontFamily\", \"{stack}\");"
    )
}
```

**`src/web/assets.rs`** — `wrap_js_str`, the sibling `wrap_css_str` has been waiting for since
Step 2. Same shape, `//<![CDATA[` instead of `/*<![CDATA[*/`.

**`src/epub.rs`** — one more piece concatenated at the injection site:

```rust
let inject = format!(
    "{INJECTED_ASSETS}{}{}",
    wrap_css_str(&settings.user_layer()),
    wrap_js_str(&settings.bootstrap_js()),
);
```

**`src/web/assets/theme-listener.js`** — the first change to this file since 5c:

```js
for (const [name, value] of e.data.vars) {
  if (value) {
    document.documentElement.style.setProperty(name, value);
  } else {
    document.documentElement.style.removeProperty(name);
  }
}
```

**`src/ui/settings.rs`** — nothing, if 5g's picker iterates `FontFamily::ALL`. A new variant
appearing in the `<select>` for free is the payoff of the `ALL` constant.

### Why it works

- **The gate turns a rule into a value.** The transport can only carry name/value pairs, so the
  trick is to make the *presence* of a pair be the on/off state and let the selector read it.
  Nothing new crosses the boundary; CSS does the branching. This is why Readium ships it, and
  it is a pattern worth keeping in mind well beyond fonts.
- **Both routes write to the same place, so there is only one state.** After the bootstrap,
  "is the override on?" has exactly one answer — whether that property is on the root's inline
  style — and both the serve path and the push path set it the same way. The rejected option
  had two answers that could disagree, which is the definition of a stale-state bug.
- **The bootstrap runs in `<head>`, so there is no flash.** `document.documentElement` exists
  as soon as the parser has opened `<html>`; the property is set before `<body>` has any boxes
  to lay out, so the chapter's first paint is already correct rather than repainting into it.
  Same reasoning as Step 6's "read the settings before the signal is created".
- **The gate is "the user asked", not "the book has `@font-face`".** It deliberately does not
  sniff for embedded fonts. A reader who picks Sans on a book with an embedded face gets Sans —
  that is their call, explicitly made. What the gate protects is the *default*, which is the
  only state a reader never consciously chose.
- **`removeProperty`, not `setProperty(name, "")`.** The empty string is a legal value to
  assign and is treated as a no-op by CSSOM, so the wrong one of the two is silent rather than
  wrong-looking — the worst failure shape there is.

### Scope note

- **Only the font family is gated.** Readium puts font-size, leading and text-align behind an
  advanced-settings flag too; here they stay unconditional. Reversing a colour or a size is
  something the reader can see and undo, and none of them can make text *unreadable* the way
  substituting a face over an embedded alphabet can. Revisit if a book proves otherwise.
- **Author `!important` still loses.** The phase's known-constraints line mentions respecting it;
  this step respects it only in the sense that the default never fights it. A book that says
  `p { font-family: X !important }` is still overridden once the reader picks a face, because
  our selector is heavier and later. That is the intended order — the reader is the last word
  when they have spoken.
- **This is the last setting.** `Settings` is finished after this step, which is what Step 6
  has been waiting for.
- **A Step 7 candidate, noted not done:** with the bootstrap in place, `--USER__fontFamily`
  lives in two places in the served document — the `:root` block *and* the inline style. The
  inline copy wins and is the one the gate reads, so the block copy is only a fallback for a
  document where scripts did not run. Collapsing *all* the variables into the bootstrap would
  make the two routes literally one mechanism and delete `vars()`; it would also make theming
  depend on JavaScript. Worth weighing in the refactor, not worth doing mid-phase.
- **`dx serve` is the gate, and it needs the right book.** Open one with an embedded font.
  Pick a face — it overrides. Pick Publisher — the book's own face comes back **on the page
  you are already on**, not after a chapter turn. Then turn a chapter under each of the two
  states and confirm the fresh document comes up matching. Finally, open devtools on the frame
  and look at `<html>`'s `style` attribute across a switch: the property should appear and
  disappear, which is the assumption the whole step rests on.

---

## Step 6 — persist the settings (sketched)

Every step since 4 has ended by writing down the same deferral — "the reader opens on the
default every time" (Step 4 scope note), "5a is where that will eventually hook in" (5a), "a
relaunch is back to 100% and Day" (5b), "settings still live in a `use_signal` in `main.rs`"
(the interlude). This is the step that closes it, and it is deliberately **after 5h**: the
settings set stops growing at 5h, so persistence is written once against a finished struct
instead of being extended by every sitting in between.

### The crux: persist the *choice*, not the CSS

`Settings` already knows how to turn itself into a name/value list — `css_vars()` — and it
is tempting to store that, since it is already a list of strings. It is the wrong thing to store.
`("--USER__fontSize", "125%")` is the *rendered* form; the state is `font_size: 125`. Storing
the rendering means parsing `"125%"` and `"1.40"` back into `u16`s on every launch, and it
welds the on-disk format to a CSS convention that Step 7 might want to change. Persist the
struct's fields; let `css_vars()` stay a pure function of them.

The second decision is the **table shape**, and it is the one worth thinking about before
typing:

- **A one-row typed table** — `theme TEXT, font_size INTEGER, line_height INTEGER, …`,
  `CHECK (id = 1)` so there can only ever be one row. Matches how `positions` stores a
  `Locator`: typed columns, one `query_row`, one struct out. Every future setting is an
  `ALTER TABLE ADD COLUMN`.
- **A key/value table** — `key TEXT PRIMARY KEY, value TEXT`. No migration ever, but every
  value is a string that has to be parsed and defaulted individually, and a field you forget
  to write is silently missing rather than a compile error.

Recommendation: **the typed one-row table**, because the settings set is finished by the time
this step runs and because it is the shape the repo already reads fluently. The migration cost
that key/value buys off is a cost this phase no longer has.

### The thing that will actually bite: load order

`App` today builds the settings signal *before* the library:

```rust
let settings = use_signal(Settings::default);          // main.rs:40
let library = use_hook(|| Rc::new(Library::open_default()));  // main.rs:41
```

Persistence inverts that — the stored settings are an input to the signal's initial value, so
the library has to open first. And the save side is a `use_effect` that reads `settings()`;
its **first run happens at mount**, writing back the row it was just loaded from. Harmless,
but know it is happening rather than discover it in the SQLite file.

### Runnable check (`cargo test`, sketch)

The one test that carries this step is a **round-trip on a settings value where every field
differs from the default** — in `library.rs`'s `mod test`, beside
`position_round_trips_and_latest_save_wins`:

```rust
let saved = Settings {
    theme: Theme::Night,
    font_size: 125,
    line_height: 170,
    // …every other field, each different from Settings::default()
};
library.save_settings(&saved).expect("save");
assert_eq!(library.settings().expect("read"), Some(saved));
```

Why "every field differs" is the whole design of the test: a field dropped from the `INSERT`
comes back as its default, and if the test value *is* the default the assertion still passes.
This is the same class of tripwire as
`the_pushed_vars_and_the_injected_layer_name_the_same_variables` — the compiler cannot see
across the SQL string, so a test has to.

Two more to expect: an empty table reads as `None` (a first launch, and the reason the load is
`unwrap_or_default()` rather than an `expect`), and a stored theme slug that no longer names a
variant falls back to `Theme::default()` instead of failing the whole read — `from_slug`
already does exactly that, which is the second job it has been waiting for since Step 4 took
the theme out of the URL.

**`dx serve`:** change all the settings, quit, relaunch — the reader comes back as you left
it, and the *first paint* is already correct rather than flickering through the default,
because the settings are read before the first render rather than in an effect after it.

### Scope note

- **Settings are global, not per-book.** `positions` is keyed by `book_id` because a position
  is about one book; a font size is about your eyes. One row, no foreign key.
- **No debounce.** One `UPDATE` of one row per click on a local SQLite file. Revisit only if a
  future control is a drag-slider rather than a stepper.
- **The stale-handler note from 5a is not this step's problem** and should not be fixed here.

---

## Step 7 — review & refactor (sketched)

The repo's phase-ending step (commit `b09d6c9`): fold duplication in the serve/inject path,
confirm the cascade order, re-read against ADR-0003. By then `Settings` will have six fields
and the `user_layer` string will be a `format!` with six holes — plus the two near-identical
`<select>` pickers 5g leaves standing (`ThemePicker` and `FontFamilyPicker` differ only in
their enum) — that is the shape to look
hardest at, along with the three near-identical `{}.{:02}` formatters 5e left standing on
purpose and the two chrome nits the popover interlude recorded (the forty-pixel circle
declared twice, and the gear icon's raw `"view-box"` attribute).
</content>
