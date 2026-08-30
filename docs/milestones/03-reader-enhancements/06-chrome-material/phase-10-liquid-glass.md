# Phase 10 — Liquid glass chrome

[← Feature: Chrome Material](README.md) · **Status:** 🚧 in progress ·
build log: [`phase-10-liquid-glass-steps.md`](phase-10-liquid-glass-steps.md)

## Goal

The reader's floating surfaces read as glass — blurred, colour-saturated, rim-lit by the
page behind them — from **one CSS primitive that behaves identically in WebKit and
Chromium**. The phase closes on a vertical slice per
[ADR-0001](../../../adr/0001-walking-skeleton-vertical-slices.md): not "we have a `.glass`
class", but *open the contents panel over a page of text on an iPad and the text shows
through it.*

## The crux

Two constraints, both established by investigation before any code was written. Together
they decide the whole shape of the phase.

### 1. The cross-platform ceiling is `backdrop-filter`'s built-in filter functions

Dioxus renders into **two engine families**: WKWebView on macOS, iOS and Linux
(WebKitGTK), Chromium on Android (WebView) and Windows (WebView2). Cross-platform therefore
means *must work in WebKit*, and that rules out both of the high-fidelity routes:

| Route | What it gives | Why it's rejected |
|---|---|---|
| `backdrop-filter: url(#svg)` with `feDisplacementMap` | Real edge refraction — the lensing that makes glass look like glass | **Chromium only.** WebKit and Gecko restrict `backdrop-filter` to built-in filter functions on purpose, to keep it on the GPU fast path. [w3c/svgwg#1142](https://github.com/w3c/svgwg/issues/1142) is open on making backdrop refraction interoperable; it is not shipping |
| `-apple-visual-effect: -apple-system-glass-material` | The **actual** system material, rendered by WebKit inside our WKWebView | Private. Needs `WKPreferences.useSystemAppearance` set by KVC through `objc2`. Not shippable, and iOS/macOS only — the opposite of this phase's goal |

So the material gets built from `blur()`, `saturate()`, gradients and shadows. That is
enough, because **the gap between "2014 frosted glass" and Liquid Glass is not the blur** —
it is saturation, a rim highlight, a gradient border, and a specular sweep that moves.

Note the asymmetry worth remembering: the restriction is on `backdrop-`filter specifically.
Plain `filter: url(#displace)` *does* work in WebKit, which leaves a door open for real
refraction later by displacing a **copy** of the content instead of the backdrop. That is
out of scope here and noted at the bottom.

### 2. Glass needs something behind it — and three of our four surfaces have nothing

`.reader-root` is a `flex-direction: column` stack (`assets/main.css`). The top bar, the
chapter `<iframe>`, and `NavRow` are **siblings that partition the height** — deliberately,
because [Phase 9 Step 5](../../04-multiplatform/01-mobile/phase-9-ios-simulator.md) tuned
that arithmetic (`62 + 778 + 34 = 874`) until the page had zero scroll range on an iPhone.

Nothing is painted behind the bars. `backdrop-filter` on them samples the page background
and yields a flat tinted panel — the effect costs a GPU pass and buys nothing.

The only surfaces that genuinely overlay the book today are the **popovers**:
`.dx-popover-content` is `position: fixed; z-index: 1000`, and the contents panel is
routinely opened over body text. That is why Step 1 starts there, and why *floating the
bars* is its own step rather than part of styling them — it is a layout change that has to
re-derive Step 5's geometry, and it can only be judged worth doing once the material is
visible.

**`.icon-button` is the partial exception, and it is worth being precise about why.** It has
three instances — the reader's close button (`src/ui/reader.rs:200`) and the shared popover
trigger (`src/ui/components/popover/component.rs:32`), used by both the contents and settings
popovers — and all three are in the top bar. (`NavRow` uses plain unstyled `<button>`s, so
the bottom bar has none.) The top bar owns no background: the buttons sit directly on
`.reader-root`, over the theme colour, *and* over the `position: absolute; left: 0; right: 0`
div holding the title and chapter label. So their backdrop is a flat colour — invisible when
blurred — **plus real text wherever the centred title runs under them**, which happens on a
narrow viewport or a long title and not on an iPad with a short one. Intermittent, but it is
genuine content, and it makes them the only non-popover surface with anything to sample
before Step 3. They go in Step 1; the payoff is partly deferred.

## The open question Step 1 answers

**Does `backdrop-filter` sample across the chapter `<iframe>` boundary in WKWebView?**

An iframe is composited into the parent's paint tree, so in principle a fixed sibling
above it blurs it like any other content. If that holds, the popovers blur real book text
and the whole phase is worth finishing. If it does not, the popovers blur only the reader's
own background, and Steps 2–3 need re-deriving with `lbb:refine` before anyone spends
effort on them. Cheap to answer, and it gates everything — so it is the first thing the
step looks at, not a footnote.

## Design decisions (recorded up front)

- **CSS only; no Rust.** The whole phase should move zero tests. If the count changes, the
  step leaked into logic that belongs elsewhere. *Amended at Step 2b:* this held for the
  material itself and no longer holds for its **input**. A specular that tracks the pointer
  needs an event source, and CSS has none — 2b adds a JS listener and four lines of Rust to
  evaluate it. The decision's intent survives in a narrower form: **the material stays pure
  CSS; only the angle is driven from outside**, and the seam between them is one custom
  property.
- **No glass library.** `liquidGL`, `ybouane/liquidglass` and `liquid-glass-js` are all MIT
  and all look good, and all share one architecture: **rasterize the DOM behind the glass
  into a WebGL texture**, then refract it. liquidGL's own figures are 55ms per capture with
  its custom rasteriser, 86ms via html2canvas; ybouane goes through `html-to-image` →
  `foreignObject`. Our backdrop is *scrolling body text* — every scroll tick invalidates the
  texture. liquidGL additionally documents Safari instability past 50% of viewport, and each
  instance costs one of ~16 WebGL contexts. Wrong architecture for a reader.
- **One primitive, not per-component styling.** A single `.glass` class in `main.css`'s
  Primitives section, next to `.icon-button`, composed onto whatever floats. Applying the
  material four times in four files is how it drifts.
- **`prefers-reduced-transparency` is a requirement, not a nicety.** Supported in Safari 17+
  and Chromium 118+. Apple ships the toggle; transparency behind text is a real legibility
  problem; this is a *reading* app. The reduced branch falls back to the opaque
  `--primary-color-*` panel the popovers already use, so it is the current design, not a
  degraded one.
- **Keep blurred surfaces small and geometrically still.** `backdrop-filter` over scrolling
  text forces a repaint of the blurred region every frame on mobile WebKit. Blur small
  panels, never animate their size or position, and treat a full-width blurred bar as a
  thing to measure before shipping.
- **The bars stay opaque until Step 3 says otherwise.** Deliberate: see crux §2.

## Planned steps

Detail for each lives in
[`phase-10-liquid-glass-steps.md`](phase-10-liquid-glass-steps.md).

- [x] **1. The static material** — a `.glass` primitive (blur + saturate, rim highlight,
      masked gradient border, reduced-transparency fallback) applied to the two popovers,
      which already float over the book, **and to `.icon-button`**, whose three instances
      sit partly over the absolutely-positioned title. Also answers the iframe question
      above. Eyeball under `dx serve` **and** a driven screenshot on the iPhone 17
      simulator, because the cross-platform claim is the point and one engine proves half
      of it. **The iframe question below is answered yes** — the popover blurs real book
      text — so Step 3 is unblocked. — `9f8188b`
- [ ] **2. Make the light move** — the specular sweep, split in two because the layer and
      the input source are different ideas with different risk. The angle is the seam: 2a
      registers and reads it, 2b drives it.
  - [x] **2a. The specular layer** — an `@property`-registered `--glass-angle` and the
        gradient on `.glass` that reads it, at a fixed angle. Pure CSS, no new inputs, and
        the registration is what makes the angle interpolable at all. *Landed:* measured on
        the simulator at the full predicted amplitude, which also confirmed the white tint
        has only 25 levels of headroom on a light theme. — `40627fa`
  - [ ] **2b. Drive the angle from the pointer** — a host-document `pointermove` listener
        that writes `--glass-angle` on the root element, coalesced to one write per frame.
        Split from the mobile input source because the two carry different risk: this one is
        unconditional, `DeviceOrientationEvent` is permission-gated. **Reverses 2a's
        `inherits: false`** — see the build log for why the input source decided that.
  - [ ] **2c. Drive the angle from device orientation** — the mobile half, where there is no
        pointer to follow. Gated on whether WKWebView grants
        `DeviceOrientationEvent.requestPermission()` at all; that needs answering before the
        step can be planned, the way Step 1 answered the iframe question.
- [ ] **3. Float the chrome** *(provisional — gated on Step 1's finding)* — give the top bar
      and `NavRow` something to blur by overlaying them on the page instead of stacking
      them beside it. Re-derives Phase 9 Step 5's viewport arithmetic and the safe-area
      padding Step 5f moved onto `html`. Only worth planning if Step 1 shows the blur
      crosses the iframe.
- [ ] **4. Review and refactor** — the phase-closing pass. Expected to carry the CSS-
      architecture question [`TODO.md`](../../../../TODO.md) has been holding ("CSS
      architecture BEM, OOCSS, etc"), since this phase adds the app's first real primitive
      that is *composed* onto other components rather than owning an element.

## Out of scope

**Real refraction** (`filter: url()` on a duplicated backdrop layer — cross-browser, but it
means maintaining a mirrored subtree; revisit once the material is settled), **the private
`-apple-visual-effect` path**, **any WebGL**, **glass inside the chapter iframe** (that is
ADR-0003's territory and the publisher's CSS), and **Android verification** — the Chromium
half is asserted from engine support, not driven, until Milestone 4 stands up an emulator.
