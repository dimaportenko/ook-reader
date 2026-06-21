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

---

## Step 1 — model a theme in Rust

> **Status:** ⬜ planned.

A theme is a set of `--USER__*` values, so model it as data and render it to a CSS string —
pure Rust, fully testable, before any of it touches the webview.

### Runnable check (`cargo test`)

```rust
#[test]
fn night_theme_sets_dark_background() {
    let css = theme_vars(Theme::Night);
    // The USER layer drives colour through these two variables (Readium convention).
    assert!(css.contains("--USER__backgroundColor"));
    assert!(css.contains("--USER__textColor"));
    // Night is light-on-dark; day is the inverse — they must differ.
    assert_ne!(theme_vars(Theme::Night), theme_vars(Theme::Day));
}
```

### Minimal implementation (sketch)

```rust
enum Theme { Day, Sepia, Night }

/// Render a theme to a `:root { … }` block of USER custom properties.
fn theme_vars(theme: Theme) -> String {
    let (bg, fg) = match theme {
        Theme::Day   => ("#ffffff", "#121212"),
        Theme::Sepia => ("#faf4e8", "#5b4636"),
        Theme::Night => ("#121212", "#cfcfcf"),
    };
    format!(":root {{ --USER__backgroundColor: {bg}; --USER__textColor: {fg}; }}")
}
```

### Why it works

- **A theme is just data → CSS.** Modelling it as an enum keeps day/sepia/night exhaustive
  (the compiler flags a missing arm) and makes custom themes a later "another set of values."
- **`--USER__` is the prefix that wins the cascade** (Step 2 wires it after the book CSS).
- **No webview here** — this is the testable half, deliberately first.

---

## Step 2 — inject the USER layer

> **Status:** ⬜ planned.

Step 1 produced the variables; now they must reach the served document *and* a tiny override
sheet must actually *use* them (a variable alone styles nothing). Inject both **after** the
book's CSS so they win at equal specificity.

### Runnable check

- `cargo test`: the served document string contains the `--USER__…` block **after** the book's
  `<link rel="stylesheet">` (assert the `<style>` index is greater than the `<link>` index).
- `dx serve`: with Night injected, the page background goes dark and text light **inside** the
  iframe; the book's structural styling (drop cap, headings) still renders.

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

### Why it works

- **Source order breaks the tie.** Same specificity, later wins — injecting after the book's
  `<link>` is what lets USER beat the author.
- **Minimal, scoped `!important`.** Only on the few properties the theme must enforce — the
  Readium discipline that avoids fighting embedded fonts / author `!important` wholesale.
- **`var(--USER__…)` indirection** is why Step 4's switch is cheap: re-serve with different
  variable values and every rule that reads them updates.

### Scope note

This switches the Step 8 handler from serving raw doc bytes to serving an injected string for
content documents. `inject_css` writes at end-of-head only — fine for the USER (after) layer;
the **RS (before)** layer in Step 3 needs the *start* of `<head>`, which `inject_css` can't do.
Decide that there, not here.

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
