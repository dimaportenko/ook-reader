---
name: refine
description: >-
  Revise the current in-flight step of a learn-by-building project — the one
  lbb:next proposed and the learner is partway through, not yet committed.
  Re-derive it when the approach or scope changed (it's too big and needs
  splitting, a better approach surfaced mid-implementation, or a new requirement
  has to be folded in — e.g. styling the current phase with an SCSS file or
  pulling in image assets), or make a light edit to its steps-doc entry (fix
  wording, add an in-progress note, adjust the checklist). Use this whenever the
  user says "refine the step", "lbb:refine", "this step is too big, split it",
  "the approach changed, update the step", "fold images/CSS/SCSS into this
  phase", "adjust the test", "tweak step N", or otherwise asks to change the
  step currently in progress rather than propose a brand-new one. Keeps the
  family's hard rules: never writes the src/ implementation, never commits. Do
  NOT use this to propose the *next* step (that's lbb:next) or to validate and
  commit a finished step (that's lbb:commit).
---

# lbb:refine — revise the current step (learn-by-building)

This is the `refine` middle of the **`lbb` (learning-by-building)** skill set. It sits
between its two companions: [[next]] (`lbb:next`) *opens* the loop by proposing a step, and
[[commit]] (`lbb:commit`) *closes* it by validating and committing. `lbb:refine` works on the
step **in flight** — the one proposed but not yet committed — when reality diverges from the
plan and the step itself needs to change before the learner finishes it.

Why this exists: a step is a hypothesis about the right next increment, written *before* the
learner starts typing. Hypotheses are wrong sometimes — the step turns out too big to write
in a sitting, a cleaner approach reveals itself once code is on the screen, or the learner
decides the current phase should also pull in something new (a stylesheet, an image asset).
When that happens you don't want a *fresh* step (that throws away the framing the learner is
already holding) and you don't want to silently let the plan and the reality drift apart. You
want to **revise the current step in place** — and keep the steps doc honest as the source of
truth. That's this skill.

## The hard rule still holds: do not write the implementation

Same rule as `lbb:next` and `lbb:commit`, because it's the whole point of the project: **the
learning happens in the writing.** When refining you may write **tests**, **prose**,
**pseudocode**, **code sketches inside the suggestion**, and **the steps doc**. You may
**read** any source file to understand what the learner has tried so far. You do **not** edit
the `src/` files that are the learning target, and you do **not** commit (that's
`lbb:commit`). A refined step is still a step the learner writes by hand — you've just
changed *what* the step is, not done it for them.

## What "the current step" means

The current step is the **newest step in the steps doc that isn't marked done** — the one the
learner is actively working on. Locate it the same way `lbb:commit` does:

- Read the phase steps doc
  (`docs/milestones/<NN-milestone>/<NN-feature>/phase-N-<topic>-steps.md`) and find the last
  step without a `> **Status:** done …` marker. The phase doc's "Step plan" checklist near
  the top is the index; the unchecked entry is your target.
- Run `git status` / `git diff` to see how far the learner has actually gotten. What they've
  already written constrains a sensible refinement — if they're three lines from done, don't
  re-derive the whole step; if they've barely started, you have more room.

If no step is in flight (the last one is already committed and there's no new one), the user
probably wants `lbb:next`, not `refine`. Say so rather than inventing a step to refine.

## Two modes — read which one the situation calls for

`lbb:refine` covers a spectrum from a one-line doc tweak to a full re-derivation. Pick the
*lightest* touch that does the job; reach for re-derive only when the step's substance
actually changed.

### Light edit — the step is right, the entry needs fixing

Use when the plan is sound and you're just keeping the doc accurate: fix a typo, sharpen the
"why," add an in-progress note ("blocked on figuring out the asset path — see Step 4"), tick
a sub-point, correct a file path. Edit the existing steps-doc block in place. Don't
re-derive the test or the code sketch — leave the learner's target where it is.

### Re-derive — the step's substance changed

Use when *what the learner should build* has shifted. The common cases:

- **Too big → split.** The step is more than one idea or can't be written-and-verified in a
  sitting. Break it into two (or more) smaller steps, smallest-first, each with its own
  runnable check. Replace the one oversized entry with the new sequence and update the step
  plan/checklist. This is the failure mode the whole method exists to avoid — a step too big
  to understand by rewriting it — so splitting is the most valuable refine you do.
- **Better approach surfaced.** The learner discovered a cleaner path once code was on the
  screen. Rewrite the test (if the observable behavior changed), the minimal-code sketch, and
  especially the **why** — name *why the new approach is better*, since that comparison is
  itself a lesson.
- **New requirement folded in.** The learner wants the current step/phase to also cover
  something new (styling, an asset, an edge case). Re-derive the step to include it — or, if
  it's really its own idea, split it out as its own step rather than bloating this one.

When you re-derive, follow `lbb:next`'s anatomy exactly — **runnable check first, then the
minimal implementation sketch, then why it works, then a scope note** — because a refined
step is held to the same bar as a fresh one. Re-read [[next]] if you need the full shape.

## Procedure

1. **Locate the current step** (steps doc + `git diff`, as above). Confirm out loud which
   step you're refining and that it isn't already done.
2. **Understand what's changing and why.** Get the learner's reason — too big? better idea?
   new requirement? Reflect it back in one line so you're refining the right thing. The *why*
   shapes whether this is a light edit or a re-derive.
3. **Pick the lightest mode that fits** (above).
4. **Revise in the steps doc, in place.** Don't append a new step at the bottom and orphan
   the old one — edit the existing entry (or, when splitting, replace it with the new
   sequence). Keep the step-plan checklist near the top in sync. Mirror the same content in
   chat so the conversation and the doc agree.
5. **Offer, don't barrel on.** End by handing control back: "Want to take a run at the
   revised step?" When it's done, the learner moves to `lbb:commit`. The pace is theirs.

## Worked scenario — folding assets and styling into the current step

The live case for this project: the learner is in **Phase 3 (EPUB rendering)** and wants to
start using **resources — image assets and an SCSS/CSS file** — in the current phase. That's
a textbook `refine`: a new requirement folded into the step in flight.

Two things to get right, both grounded in the repo:

- **Honor the deferral discipline (ADR-0002).** The phase-3 steps doc deliberately starts
  *crude* — raw `dangerous_inner_html`, no asset protocol, the book's own CSS not loading —
  because for a prose novel it already reads, and "faithful styling" is the deferred unlock
  pulled in *when it becomes the worst real annoyance.* So before re-deriving, check: is this
  styling work the learner's *own* app chrome (a stylesheet for the reader UI, an icon) —
  which is fair game now — or is it the deferred book-CSS / asset-protocol work the doc is
  explicitly saving for later? If it's the latter, say so and let the learner decide to pull
  it forward on purpose rather than drift into it. Surfacing that trade-off is the refine.

- **Point at the real Dioxus mechanism, don't hand-wave.** Per the `dioxus-07` skill,
  local resources go through the `asset!` macro (paths start at the project root, e.g.
  `asset!("/assets/cover.png")`), images via
  `img { src: asset!("/assets/…") }`, and a stylesheet via the `document::Stylesheet`
  component:

  ```rust
  document::Stylesheet { href: asset!("/assets/reader.css") }
  ```

  For **SCSS** specifically: the asset pipeline is what compiles it, so the step's runnable
  check has to account for that — verify the `dx serve` build actually picks up and compiles
  the `.scss`, and confirm the exact `asset!`/extension handling against the `dioxus-07`
  skill and Dioxus 0.7 docs rather than assuming. Don't assert SCSS
  behavior you haven't confirmed — a refined step that points the learner at a wrong macro
  wastes their hand-writing time.

Then re-derive: most styling/asset work is a **visual** step (eyeball under `dx serve` +
`cargo clippy`, since you can't unit-test that a stylesheet loaded), so the runnable check is
a specific thing to look for — "the reader's `.css`/`.scss` rules visibly apply and the
cover `<img>` renders" — not a `#[test]`. Keep it one idea: if "load a stylesheet" and "add
the cover image" are two ideas, that's two steps, not one bloated one.

## What makes this skill succeed

- The current step changed for a clear reason, and the steps doc now reflects reality — no
  orphaned old entry, no drift between the plan and what the learner is actually building.
- A too-big step got *split* rather than rationalized; each resulting step is still small
  enough to understand by rewriting it.
- Nothing in `src/` was written by you, and nothing was committed — you revised the plan and
  the doc; the learner writes the code and `lbb:commit` ships it.
- When the refinement brushed against deliberately-deferred work (the asset-protocol /
  faithful-styling unlock), you surfaced the trade-off instead of quietly pulling it forward.
