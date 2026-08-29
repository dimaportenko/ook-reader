# Phase 10 — Build log

[← Phase 10: Liquid glass chrome](phase-10-liquid-glass.md)

The per-step detail: the check first, then the minimal implementation, then why it works.
The phase doc holds the crux, the design decisions and the step index.

---

## Step 1 — The static material

> **Status:** done — committed in `9f8188b` (133 tests green), driven on the iPhone 17 simulator.

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

## Step 2a — The specular layer

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

The fourth material layer: a bright band raked across the glass at an angle that is a
*registered* `<angle>`, not a token string. The angle stays fixed here. Driving it from the
pointer and from `DeviceOrientationEvent` is Step 2b.

**Why the split.** The phase doc planned one step. It is two ideas with two different risks:
the layer is pure CSS that either paints or does not, while the drive adds a JS input source,
a Dioxus `eval` channel, and a per-frame cost that has to be measured against scrolling text.
The **angle is the natural seam** — 2a registers and reads it, 2b writes it — and the seam
falls exactly where the risk changes.

### The check, first

Same idiom as Step 1: `MAIN_CSS_SOURCE` in `src/main.rs`'s test module, because both ways
this step fails silently are invisible to a screenshot.

| Test | What it stops |
|---|---|
| `the_specular_angle_is_registered_with_every_descriptor_it_needs` | An `@property` rule missing `syntax`, `inherits` **or** `initial-value` is **invalid and dropped whole**. Nothing errors: `var(--glass-angle)` then resolves to nothing, the gradient goes invalid at computed-value time, and `background-image` is dropped. The panel looks exactly like Step 1's. The same test also asserts `.glass` actually *reads* `var(--glass-angle)` — a registered property nobody reads is a no-op |
| `transparency_can_be_turned_off_without_turning_the_chrome_off` *(extended)* | The reduced-transparency branch keeping the highlight. Losing the blur does not remove the specular, and a lit streak raking across a flat opaque panel is precisely the glass cue the setting asks you to drop |

Watched fail first, as two separate tests: `134 passed; 2 failed`, on *"an unregistered
custom property is a token string, not an angle"* and *"a reading app owes the setting an
answer"*. The second was then folded into Step 1's existing reduced-transparency test during
the `simplify` pass — the invariant it asserts is that test's whole subject, and a second
test grepping the same `@media` block was duplication. After the merge: **135 passed, 0
failed**, clippy clean on the host and on `aarch64-apple-ios-sim`.

### What landed

**`assets/main.css`**, immediately above `.glass` — the registration:

```css
@property --glass-angle {
  syntax: "<angle>";
  inherits: false;
  initial-value: 100deg;
}
```

and inside `.glass`, one declaration:

```css
  background-image: linear-gradient(
    var(--glass-angle),
    transparent 34%,
    rgb(255 255 255 / 0.3) 47%,
    rgb(255 255 255 / 0.07) 57%,
    transparent 70%
  );
```

plus `background-image: none` in the `prefers-reduced-transparency` block. That is the
entire diff outside the tests. No new file, no pseudo-element, no Rust.

### Why it works

**The specular is a `background-image`, not a `::after`.** This is the fork worth arguing
with, and the reason is a landmine in the neighbouring stylesheet:
`src/ui/components/popover/style.css` already carries four
`.dx-popover-content[data-side="…"]::after` rules — a tooltip arrow that was **never
finished**, because nothing anywhere declares `content` for that pseudo-element, so it is
never generated. Adding `.glass::after { content: "" }` would have *brought those rules to
life*: at `(0, 2, 1)` they outrank `.glass::after` at `(0, 1, 1)`, so they would have
overridden `top` and `left` out of the specular's `inset: 0` and left a sliver hanging off
the panel's edge. A layer on the element's own `background-image` sidesteps all of it, and is
strictly less machinery besides — no generated box, no `pointer-events: none` needed, no
extra paint layer per glass surface. `::before` was not available either; the gradient border
owns it, and its `mask-composite: exclude` would have ring-masked a specular sharing the box.

**`background-image` paints over `background-color`, and both paint over the filtered
backdrop.** `backdrop-filter` operates on what is *behind* the element; the element's own
background layers then composite on top of that result. So the specular sits above the
blurred book text without disturbing it, and the existing `--glass-tint` fill still shows
through wherever the gradient is `transparent`.

**Registering the property is the load-bearing half, even though nothing animates yet.**
An unregistered custom property is an untyped token string: it substitutes textually and
**cannot be interpolated**, so Step 2b could not `transition` it. Registration also buys two
things now — `initial-value` means `var(--glass-angle)` always resolves, so no `var()`
fallback is needed at the point of use, and a bad value is rejected at *parse* time rather
than poisoning the gradient into invalid-at-computed-value-time and silently dropping the
whole declaration.

**`@property`'s `initial-value` is not a declaration, so it does not fight consumers.** This
matters because Step 1 got burned by exactly that: `--glass-blur: 20px` declared *inside*
`.glass` tied `.icon-button`'s `6px` on specificity and won on source order. An
`initial-value` lives outside the cascade entirely — it is the property's definition, not a
rule that matches an element — so any consumer that declares `--glass-angle` beats it with no
specificity contest at all. It is a *third* pattern alongside the two the material already
uses (`var()` fallback at point of use for the knobs, last-declared-wins for the fill), and
it is the cleanest of the three.

**`inherits: false` is the right answer, not the safe one.** An angle is a property of *this
surface's* relationship to the light, not something a child glass panel should adopt from its
parent. It also stops the value walking down into the popover's contents, where nothing reads
it and every element would carry a computed copy.

**The four stops are asymmetric on purpose.** `transparent → 0.3 → 0.07 → transparent`
gives a bright leading edge with a long dim trail. Three symmetric stops read as a *stripe*;
the trailing shoulder is what makes it read as light glancing off a curved surface.

### Verified — driven on the iPhone 17 simulator

Measured rather than eyeballed, because on a cream theme the effect is real but small and a
screenshot is not evidence.

On a text-free row inside the open contents panel, luminance sampled left-to-right:

| Row | Left edge | Peak | Right edge | Peak at x |
|---|---|---|---|---|
| `y=290` (panel gap) | 228.8 | **236.3** | 229.2 | 226 |
| `y=690` (panel gap) | ~229 | 237.4 | ~231 | 151 |
| `y=845` (backdrop, control) | 242.0 | 242.0 | 242.0 | — flat, spread **0.7** |

Three things fall out of that:

1. **The gradient is live and at full strength.** A `0.3` white stop over a base of 230 has a
   predicted lift of `0.3 × (255 − 230) = 7.5`. Measured lift: **7.5**. So `@property` is
   supported in this WKWebView, the rule was not dropped, and no stop is being clamped.
2. **The band is tilted the way `100deg` says.** 100° points right and slightly *down*, so
   the same gradient position occurs at a smaller `x` as `y` grows. The peak moves left —
   x=226 at y=290, x=151 at y=690.
3. **The variation is the material, not the content.** The backdrop strip below the panel is
   uniform across the identical x range (spread 0.7 ≈ noise), so nothing in the source image
   could have produced the hump.

**And it confirms Step 1's open problem rather than fixing it.** The reason a +7.5 lift is
the *most* this can do on a sepia theme is that the backdrop is already at 230 of 255 — there
are only 25 levels of headroom, and a white specular can only spend them. On a dark theme the
same declaration lifts ~70. The material's light model still assumes a dark backdrop; that is
the next step's problem, not this one's.

### Scope note

Not in this step: **driving the angle** (2b — pointer + `DeviceOrientationEvent`, a JS input
source and the first thing in this phase that costs per-frame work); the **white-tint
saturation on light themes**, which Step 1 pencilled in here and which this step has now
*measured* instead of fixed — it is a change to the material's **fill**, not its
**highlight**, and it wants the app's `--USER__*` theme rather than the dx-components
`--light`/`--dark` switch, which tracks the OS colour scheme and not the reader's chosen
theme; and any **performance measurement**, which belongs with 2b where motion arrives.

**Found, not fixed:** the dead `.dx-popover-content[data-side="…"]::after` arrow rules
described above. Four rules that style a pseudo-element nothing generates — inert today, and
a trap for the next person who reaches for `::after` on a popover. Candidate for Step 4.

---
