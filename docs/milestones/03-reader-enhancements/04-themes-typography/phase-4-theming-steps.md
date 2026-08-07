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
5. **Typography settings (later)** — font-size, line-height, line-length, margins, fonts.
6. **Review & refactor.**

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

## Steps 5–6 — sketched
- **Step 5 — typography (later, one at a time).** `--USER__fontSize` (75–250%),
  `--USER__lineHeight` (1–2), `--USER__lineLength`, page margins, then `--USER__fontFamily`
  from a *curated* list. Each: a variable + a control + a `cargo test` on the rendered string.
  Respect embedded-font / author-`!important` intent.
- **Step 6 — review & refactor.** The repo's phase-ending step (commit `b09d6c9`): fold
  duplication in the serve/inject path, confirm the cascade order, re-read against ADR-0003.
</content>
