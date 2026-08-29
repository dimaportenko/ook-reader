# Phase 10 — Build log

[← Phase 10: Liquid glass chrome](phase-10-liquid-glass.md)

The per-step detail: the check first, then the minimal implementation, then why it works.
The phase doc holds the crux, the design decisions and the step index.

---

## Step 1 — The static material

**Status:** 🚧 written, uncommitted — 132 tests green, clippy clean, driven on the iPhone 17 simulator

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Build the `.glass` primitive and put it on the two surfaces that already float over the
book: the contents popover and the settings popover. Three of the four material layers land
here; the fourth (the moving specular) is Step 2.

### The check, first

This step looked purely visual, and it is not. `src/main.rs`'s test module already asserts
on **`MAIN_CSS_SOURCE`** — the `include_str!` of `assets/main.css` that Phase 9's Steps 5 and
5f used to pin the safe-area rules. Three things about this material fail *silently*, in
exactly the way a screenshot cannot catch, so they became real tests in that idiom:

| Test | What it stops |
|---|---|
| `every_backdrop_filter_is_paired_with_its_webkit_spelling` | An unprefixed declaration. Chromium renders it, WebKit ignores it — so the bug ships looking fine on the machine it was written on. Counts `backdrop-filter:` against `-webkit-backdrop-filter:` and demands exactly two of the first per one of the second |
| `the_glass_fill_is_declared_after_the_button_it_overrides` | A future reorder of `main.css`. `.icon-button` and `.glass` both set `background-color` at one class of specificity, so **only source order** decides; swapping them makes every icon button opaque with nothing to point at |
| `transparency_can_be_turned_off_without_turning_the_chrome_off` | The accessibility branch quietly rotting — asserts the reduced-transparency block kills the blur *and* routes through `--glass-fallback` |

Watched fail first: `129 passed; 3 failed`, on *"the glass has no blur to speak of"*, *"the
material itself"*, and *"a reading app owes the setting an answer"*. After the
implementation and the `simplify` pass: **132 passed, 0 failed**, clippy clean.

The rest is eyeball, and still the important half:

**`dx serve`.** Open a book, page to dense text, open the contents popover over it. The body
text should be visibly **blurred and colour-shifted** behind the panel, with a bright
hairline on its top edge and a border brighter at top-left than bottom-right.

**The finding to write down.** The chapter lives in an `<iframe>`. If `backdrop-filter` does
not cross that boundary in WKWebView you get a panel blurring the *reader's own background*
and no text — which looks nearly identical at a glance on a light theme. Check it on
**sepia over a dense paragraph**, where blurred text is unmistakable. Record the answer here:
it decides whether Step 3 is worth planning.

**`just serve-ios`**, then drive to the same state and snapshot:

```
agent-device open com.dimaportenko.ook-reader --platform ios --foreground
```

Chromium is the permissive engine; WebKit is the one that can refuse. A desktop-only eyeball
proves the easy half and would let a WebKit-specific failure ship.

### What landed

**`assets/main.css`** — the material, in the Primitives block, *after* `.icon-button`:

```css
.glass {
  --glass-tint: rgb(255 255 255 / 0.12);

  background-color: var(--glass-tint);
  -webkit-backdrop-filter: blur(20px) saturate(1.8);
  backdrop-filter: blur(20px) saturate(1.8);
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.45),
    inset 0 -1px 0 rgb(255 255 255 / 0.12),
    0 8px 24px rgb(0 0 0 / 0.18);
}
```

plus a `.glass::before` carrying the gradient border (two mask layers composited with
`exclude`, leaving only the 1px `padding` ring), and a
`@media (prefers-reduced-transparency: reduce)` block that drops the blur, hides the ring,
and falls back to `var(--glass-fallback, var(--primary-color))`.

`.icon-button` gained `position: relative` and now declares `--glass-fallback`, reading it
back through `background-color: var(--glass-fallback)`. `.icon-button:hover` no longer sets
`background-color` at all — it sets `--glass-tint` *and* `--glass-fallback`, so hover
survives in both transparency modes.

**`src/ui/components/popover/style.css`** — `.dx-popover-content` lost its opaque
`background` and its inset ring `box-shadow` (the material owns both now) and declares its
own `--glass-fallback`.

**`src/ui/components/popover/component.rs`** and **`src/ui/reader.rs:200`** — `glass`
appended to the class lists, in the shared component rather than at each call site.

### Why it works

**`saturate()` is the load-bearing declaration.** Blur averages colour toward grey, which is
precisely why 2014 glassmorphism looks dead. Pushing saturation back past 1 re-injects the
chroma the blur destroyed, so the panel reads as *tinted by* what is behind it rather than as
a grey card. If you keep one line of this step, keep that one.

**The tint has to be translucent or the blur is invisible.** `backdrop-filter` filters what
is painted *behind* the element; an opaque `background` then paints straight over the result.
That is the commonest way this effect silently does nothing, and it is why
`.dx-popover-content`'s old opaque `background` had to go rather than coexist.

**Specificity, not source order, is what breaks the hover.** Two single-class selectors tie,
and the later wins — which is all `.glass` after `.icon-button` needs for the resting state,
and is what the ordering test pins. Add a pseudo-class and the tie is gone:
`.icon-button:hover` counts as two, outranks `.glass` permanently, and no reordering saves
it. The fix is not a bigger selector — it is to **stop setting the property the material
owns** and move hover onto a *variable* the material reads. That is the general shape of a
composable primitive: expose the knobs, so consumers never have to override your
declarations.

**`--glass-fallback` is a `var()` fallback, not a declaration on `.glass`.** Declaring a
default *on* `.glass` would beat every consumer's own value on the same tie-plus-order rule,
collapsing every surface to one colour. Writing it as
`var(--glass-fallback, var(--primary-color))` inverts that: consumers declare, and the
primitive supplies a neutral safety net only if nobody did. `.icon-button` keeps
`--primary-color-3`, the popover keeps `--primary-color-5`, and neither colour is hardcoded
into the material.

**The ring is `position: absolute; z-index: -1`, and `.glass` deliberately sets no
`position`.** A negative-z-index child paints after its parent's background and before its
content, so the ring sits under the popover's text instead of over it. It needs a positioned
ancestor — but `.glass` **cannot** supply one, because `.dx-popover-content` is
`position: fixed` and a `position: relative` on the primitive would fight it across two
stylesheets whose load order is not guaranteed. So positioning is the *consumer's* contract:
the popover is already `fixed`, and `.icon-button` gained `relative`.

**No `isolation: isolate` is needed.** A non-`none` `backdrop-filter` already forms a
stacking context, so the `-1` cannot escape. In the reduced-transparency branch the filter is
gone — and so is the ring, via `display: none`.

**`-webkit-mask-composite: xor` is the legacy spelling of `mask-composite: exclude`.** Both
ship, because this is exactly the sort of place the two engines diverge and you find out on
the simulator.

**The fallback degrades to the current design, not a worse one.** Reduced transparency lands
on the same opaque panel the popovers had before this step, so someone with the setting on
loses an effect and gains nothing broken. Building the branch in the same change as the
effect is deliberate — retrofitted accessibility branches are the ones that rot.

### Findings (driven on the iPhone 17 simulator)

**The gating question is answered: yes.** `backdrop-filter` **does** sample across the
chapter `<iframe>` boundary in WKWebView. Proven by A/B rather than by eye, because the
sepia-on-cream difference is not eyeball-separable: the contents popover was opened over
page 1, the page turned to 2 behind it, and the popover reopened. A 34×560 crop of the panel
interior, clear of the ToC's own text, moved by **RMSE 0.0605** — against a noise floor of
**exactly 0** measured from two captures of the identical state. The control strip of iframe
visible below the panel moved 0.0645, so the panel is tracking its backdrop at very nearly
the full amplitude of the content change. **Step 3 is unblocked.**

It samples sibling Dioxus DOM too — `NavRow`'s "Prev / Page 1 of 60 / Next" ghosts visibly
through the bottom of the panel, which is the one place the effect is obvious at a glance.

**The icon buttons work and cannot be seen working.** The material applies — measured, the
contents button's left edge, which has the title's tail behind it, is `0.0057` darker than
that same button's clear right edge, while the close button with nothing behind it shows
`0.0000` across the identical measurement. So the mechanism is live and the contrast is
roughly a hundredth of what would be visible. Two separate causes, and the first is a design
flaw in the primitive rather than a wrong number:

1. **`blur()` was hardcoded at 20px with no knob.** A 20px radius on a 40px surface averages
   its entire backdrop into flat colour. Fixed by making the radius `--glass-blur`, the
   third knob alongside `--glass-tint` and `--glass-fallback`; `.icon-button` sets `6px`.

   **The first attempt at that fix was dead on arrival, and the cascade is why.** Declaring
   `--glass-blur: 20px` *inside* `.glass` put it at one class of specificity — the same as
   `.icon-button`'s `6px`, and later in the file, so the material's default beat every
   consumer that set the knob in a plain class rule. `--glass-tint` had the identical defect
   and only appeared to work because its one override lives on `.icon-button:hover`, which
   outranks it at `(0, 2, 0)`. The knobs are now `var()` fallbacks *at the point of use* —
   `blur(var(--glass-blur, 20px))` — matching `--glass-fallback`, which had been written that
   way from the start and was the only one of the three that was right.

   Pinned by `a_surface_can_set_its_own_blur_radius`, which asserts `.glass` declares no
   `--glass-blur` at all. It is the mirror of the ordering test above: the **fill** must be
   declared last so the material wins, the **knobs** must not be declared so consumers win.

   Measured before and after on the same simulator, on the contents button's text-backed
   edge against its own clear edge: `-0.0057` at 20px, `-0.0415` at 6px — a 7.3× increase in
   sampled contrast, against a reference button with nothing behind it at `+0.00004`. The
   title's blurred tail is now visible inside the button rather than merely present.
2. **A white tint over a light theme saturates.** The close button measured `1.0000` on both
   sides — pure white, clipped. The tint wants to be theme-aware, or to darken rather than
   lighten when the backdrop is already light. Not fixed; it is a real open question for
   Step 2, which is where the material's light model gets revisited anyway.

**Not mine, but found here:** the chapter title has no `text-overflow`, so on a phone it runs
under the top-bar buttons and is simply clipped mid-word ("Sherlock Holm"). A layout bug that
predates this step — candidate for Step 4 or Phase 9's Step 6.

**Still untested:** the `prefers-reduced-transparency` branch (asserted in Rust, never
rendered), hover (meaningless on iOS), and the entire Chromium half — the cross-platform
claim rests on engine support, not on a driven Android or desktop check.

### Scope note

Not in this step: the **moving specular** highlight (Step 2 — it needs an
`@property`-registered custom property and a JS input source, which is a different idea and
a different risk); the **top bar and `NavRow` themselves** (Step 3, and only if the iframe
answer is yes — see the phase doc's open question); a **pressed/active state** for touch,
which the hover fix will make conspicuous by its absence; and any **performance
measurement** of blur during a page turn, which belongs with Step 2 where motion is
introduced.

On `.icon-button` specifically, set expectations before you look: on an iPad with a short
book title there is nothing behind the buttons but flat theme colour, and the glass will
look like a faint tinted circle. That is not a bug in the CSS — it is crux §2 showing up on
the smallest surface in the app. Narrow the window until the centred title runs under the
buttons and the material appears. The full payoff arrives with Step 3.

---
