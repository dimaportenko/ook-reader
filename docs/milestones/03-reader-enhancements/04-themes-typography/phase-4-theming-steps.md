# Phase 4 — Themes & Typography — Build Log

[← Phase doc](phase-4-theming.md) · decision:
[ADR-0003](../../../adr/0003-reader-controlled-theming-injected-layer.md)

Per-step build log: the crux, the step plan, and for each step the runnable check → minimal
implementation → why it works. The phase doc holds the high-level checklist; this file is the
detailed trail. Newest step appended at the bottom.

## The crux

A theme is not "the book's CSS *or* ours." It's a **cascade**: a small reading-system layer
injected *around* the book's untouched CSS, driven by `--USER__*` custom properties on
`:root`. Changing a setting = changing one variable value and re-rendering the document.

The hard part isn't the CSS rules — it's the **injection seam**: getting our stylesheet +
variables *into* a document we render inside an isolated, script-free iframe, on every
settings change. The insight that keeps the seam testable: render each content document as a
**served XHTML resource** (not `srcdoc`), so we own its bytes at serve time — that's where we
inject — and the byte-building is **pure Rust** (`cargo test`), the same Rust/UI split that
kept Phase 3 small. (Bonus: served XHTML is parsed as XML, which fixes the anchor-wrap bug.)

## Step plan (smallest-first, one idea each)

1. **Serve the current content document as XHTML** — iframe `src`, not `srcdoc`. Fixes the
   anchor-wrap bug; opens the injection seam. *(asset handler, content-type, iframe `src`)*
2. **Model a theme in Rust** — `Theme` enum → `:root { --USER__… }` string. Pure Rust test.
   *(enums, match, the `--USER__` convention)*
3. **Inject the USER layer** — variable block + minimal override sheet, *after* the book CSS.
   *(`rbook` `inject_css`, cascade source-order)*
4. **Add the RS-defaults layer before the book CSS** — completes RS < author < USER.
5. **Theme switcher in the app chrome** — `use_signal` → reload the frame.
6. **Typography settings (later)** — font-size, line-height, line-length, margins, fonts.
7. **Review & refactor.**

> **Dependency.** Steps 2–4 all need Step 1's served-document seam. Do Step 1 first.

---

## Step 1 — serve the current content document as XHTML

> **Status:** ⬜ planned. This is the "Next step — fix the anchor bug" work, sequenced here
> because the fix and the theming seam are the same change (ADR-0003).

### The bug this also fixes

Chapter `h-1` contains exactly one anchor — `<a id="chap01"/>`, self-closing, no `href` — and
**zero** `</a>`. The file is XHTML. `iframe { srcdoc }` parses as **HTML**, where `<a>` is not
a void element, so `/>` is ignored, the anchor never closes, and the whole chapter becomes its
descendant — inheriting `1.css`'s `a:link { color: blue }` / `a:hover { color: red }`. Serving
the document with `Content-Type: application/xhtml+xml` and pointing the iframe at it makes the
webview parse it as **XML**, which honours `<a/>`. (`rbook` is not at fault — it round-trips
`<a/>` faithfully; the breakage is the HTML parse of XHTML.)

### Runnable check (`dx serve` + devtools)

Webview wiring, so eyeball — plus a pure-Rust test if you extract the document-building seam.

- The chapter renders as **normal prose**, not a blue link; **hovering does not turn it red**.
- devtools → Network: the document request returns `Content-Type: application/xhtml+xml`.
- `cargo clippy` clean.

If you split out a `fn render_doc(epub, path) -> Result<String>` that returns the rewritten
XHTML, a `cargo test` can assert the self-closing anchor survives intact
(`assert!(html.contains(r#"<a id="chap01"/>"#))`) — proving we're handing the webview real
XHTML, not normalised HTML.

### Minimal implementation (sketch — you write it by hand)

The current `/epub/` handler serves *raw resource bytes*. Content documents need the
**rewritten** form (the `/epub/…` path rewrite from Phase 3 Step 6) and the XHTML content-type.
Two shapes to weigh:

- **Serve content docs through the same handler**, distinguishing "is this a spine document?"
  and, if so, returning `read_str_with(&rewrite)` bytes with `application/xhtml+xml`; other
  resources keep the Phase-3 byte path. The iframe then uses `src: "/epub/{doc_path}"`.
- **Keep the rewrite in `load_spine`** and have the handler look the current doc up by path.

Either way: `content_type_for` gains `"xhtml" | "htm" => "application/xhtml+xml"` (or the doc
branch sets it explicitly), and `SpineList`'s `iframe { srcdoc }` becomes
`iframe { src: "/epub/{current_doc_path}" }`. Keep `sandbox: "allow-same-origin"`.

### Why it works

- **XML parsing honours `<a/>`.** The content-type, not the markup, decides the parser. With
  `application/xhtml+xml` the self-closing anchor is a complete empty element — no wrapping.
- **A real document URL is the injection seam.** Steps 2–4 inject into the bytes this serves;
  a settings change re-serves them. `srcdoc` would force rebuilding one giant string in the
  rsx every change — the served path keeps the document-building in pure Rust.
- **Script-free stays intact.** No `allow-scripts` needed: we re-serve to restyle.

### Scope note

This changes the Phase-3 render path on purpose (ADR-0003, one-way-door flagged). No theming
yet — Step 1 only makes the document render correctly *and* serveable. Themes land in Step 3.

---

## Step 2 — model a theme in Rust

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
- **`--USER__` is the prefix that wins the cascade** (Step 3 wires it after the book CSS).
- **No webview here** — this is the testable half, deliberately first.

---

## Step 3 — inject the USER layer

> **Status:** ⬜ planned.

Step 2 produced the variables; now they must reach the document *and* a tiny override sheet
must actually *use* them (a variable alone styles nothing). Inject both **after** the book's
CSS so they win at equal specificity.

### Runnable check

- `cargo test`: the served document string contains the `--USER__…` block **after** the book's
  `<link rel="stylesheet">` (assert the `<style>` index is greater than the `<link>` index).
- `dx serve`: with Night injected, the page background goes dark and text light **inside** the
  iframe; the book's structural styling (drop cap, headings) still renders.

### Minimal implementation (sketch)

`rbook`'s rewrite can inject CSS just before `</head>` — i.e. *after* the book's `<link>`s,
exactly the USER-after slot:

```rust
let layer = format!(
    "{vars}\nbody {{ background: var(--USER__backgroundColor) !important; \
                     color: var(--USER__textColor) !important; }}",
    vars = theme_vars(theme),
);
let rewrite = EpubRewriteOptions::default()
    .rewrite_paths(PathRewrite::prefix("/epub/"))
    .inject_css(&layer); // confirm exact builder name against your rbook 0.7.x
```

### Why it works

- **Source order breaks the tie.** Same specificity, later wins — injecting after the book's
  `<link>` is what lets USER beat the author.
- **Minimal, scoped `!important`.** Only on the few properties the theme must enforce — the
  Readium discipline that avoids fighting embedded fonts / author `!important` wholesale.
- **`var(--USER__…)` indirection** is why Step 5's switch is cheap: re-serve with different
  variable values and every rule that reads them updates.

### Scope note

`inject_css` writes at end-of-head only — fine for the USER (after) layer. The **RS (before)**
layer in Step 4 needs injecting at the *start* of `<head>`, which `inject_css` can't do; that
is the one spot ADR-0003 flags as a possible `rbook`-fork trigger. Decide there, not here.

---

## Steps 4–7 — sketched

- **Step 4 — RS-defaults layer (before book CSS).** A normalize/`--RS__*` defaults sheet at
  the *start* of `<head>`, completing RS < author < USER. Either a small head-rewrite in our
  serve path or (if awkward) the `rbook` tweak ADR-0003 reserves. Test: ordering of the three
  layers in the served string.
- **Step 5 — theme switcher.** `let mut theme = use_signal(|| Theme::Day);` in the chrome;
  Day/Sepia/Night controls `.set` it; the iframe `src`/reload picks up the new injection.
  Eyeball: click Night → page goes dark.
- **Step 6 — typography (later, one at a time).** `--USER__fontSize` (75–250%),
  `--USER__lineHeight` (1–2), `--USER__lineLength`, page margins, then `--USER__fontFamily`
  from a *curated* list. Each: a variable + a control + a `cargo test` on the rendered string.
  Respect embedded-font / author-`!important` intent.
- **Step 7 — review & refactor.** The repo's phase-ending step (commit `b09d6c9`): fold
  duplication in the serve/inject path, confirm the cascade order, re-read against ADR-0003.
</content>
