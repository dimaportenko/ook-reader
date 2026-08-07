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
4. **Theme switcher in the app chrome** — `use_signal` → reload the frame.
5. **Typography settings (later)** — font-size, line-height, line-length, margins, fonts.
6. **Review & refactor.**

> **Dependency.** Every step here serves through the Phase 3 Step 8 handler. Steps 2–3 inject
> into the served document; Step 4 re-serves it on a settings change.

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

## Steps 3–6 — sketched

- **Step 3 — RS-defaults layer (before book CSS).** A normalize/`--RS__*` defaults sheet at
  the *start* of `<head>`, completing RS < author < USER. Either a small head-rewrite in our
  serve path or (if awkward) the `rbook` tweak ADR-0003 reserves — the one realistic
  fork-trigger. Test: ordering of the three layers in the served string.
- **Step 4 — theme switcher.** `let mut theme = use_signal(|| Theme::Day);` in the chrome;
  Day/Sepia/Night controls `.set` it; the iframe `src` reload picks up the new injection.
  Eyeball: click Night → page goes dark.
- **Step 5 — typography (later, one at a time).** `--USER__fontSize` (75–250%),
  `--USER__lineHeight` (1–2), `--USER__lineLength`, page margins, then `--USER__fontFamily`
  from a *curated* list. Each: a variable + a control + a `cargo test` on the rendered string.
  Respect embedded-font / author-`!important` intent.
- **Step 6 — review & refactor.** The repo's phase-ending step (commit `b09d6c9`): fold
  duplication in the serve/inject path, confirm the cascade order, re-read against ADR-0003.
</content>
