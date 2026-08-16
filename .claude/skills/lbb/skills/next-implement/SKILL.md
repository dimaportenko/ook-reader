---
name: next-implement
description: >-
  Do everything lbb:next does — locate the current phase, derive the next small
  test-first step, write the runnable check, explain the crux and the why, and
  record the step in the steps doc — and then ALSO write the implementation in
  src/, run the checks, and clean the diff with the simplify skill, so the
  learner reviews a working, tidy diff instead of typing it themselves.
  The one skill in the lbb family that is allowed to touch the implementation,
  and only because the user explicitly invoked it. Use when the user says
  "lbb:next-implement", "next step but write it", "do the next step for me",
  "implement the next step and I'll review", or otherwise asks to advance a
  hand-coded learning project WITH the code written for them. Never commits
  (that's lbb:commit). Do NOT use when the user wants to write the code
  themselves — that's lbb:next.
---

# lbb:next-implement — propose the next step *and* write it (learn-by-building)

This is the `next-implement` variant of the **`lbb` (learning-by-building)** skill set. It is
[[next]] (`lbb:next`) plus the implementation: same phase-locating discipline, same crux, same
test-first anatomy, same steps-doc entry — and then you actually write the code into `src/`,
run the checks, clean the diff with `simplify`, and hand the learner a **finished, green diff
to review**.

Its companions: [[next]] proposes a step for the learner to write, [[refine]] (`lbb:refine`)
revises the step in flight, [[commit]] (`lbb:commit`) validates and ships it. This skill
replaces `lbb:next` for one invocation only — it does **not** replace the loop.

## Why this exists, and what it costs

The family's premise is that **the learning happens in the writing**. This skill trades that
for a different mode: the learning happens in the **reading and critiquing**. That's a real
mode — reviewing a working implementation you understand well enough to argue with teaches
plenty — but it is *weaker* than writing it, and the trade is the user's to make, never
yours.

So: **only ever run this when the user explicitly invoked it.** Never route here from "what's
next" or "suggest the next step" — those are `lbb:next`. If the user asks for the next step
without asking you to write it, and you think writing it would help, *offer* and wait.

Because the mode is weaker, the deliverable has to be stronger: a diff dumped without
explanation teaches nothing. The **review handoff** (below) is the part that carries the
learning here, and it is not optional.

## What you may write

Unlike every other skill in this family, you may edit `src/`. Specifically:

- ✅ The step's implementation, in the source files the step names.
- ✅ The step's tests, in the repo's existing test idiom.
- ✅ Quality cleanups to that implementation via the `simplify` pass (step 5) — bounded to
  the code this step touched.
- ✅ The steps doc entry (including the provenance note — see below).
- ❌ **Commits.** Still `lbb:commit`'s job. Leave the tree dirty for review.
- ❌ **Scope beyond the step.** Write the one step you derived and nothing more — no
  drive-by refactors of neighboring code, no "while I was in here." If you spot something
  worth changing, name it in the handoff as a candidate for a later step.
- ❌ **Comments.** `CLAUDE.md` is explicit: code you produce carries no explanatory comments,
  no doc comments, no `//` notes. The *why* goes in your chat handoff and the steps doc,
  which is where the learner's own comments will come from. Never touch comments the learner
  already wrote.

## Procedure

### 1. Derive the step exactly as `lbb:next` would

Do not shortcut this because you're about to write the code — the step's *shape* is what
keeps the diff reviewable. Follow [[next]] in full:

- **Step 0: locate the current phase and step from the docs**, not from conversational
  momentum. Earliest phase that is `🚧 in progress`; first unchecked item in its "Planned
  steps"; reconcile against the companion `-steps.md`. Surface any mismatch instead of
  guessing.
- **One idea per step.** The size discipline matters *more* here, not less: an oversized
  step the learner didn't write is an unreviewable diff. If what you derived is two ideas,
  implement the first and say the second is next.
- **Crux first** when opening a new topic — what makes this hard and the insight that
  unlocks it. The learner needs the frame before they read your code.
- **Runnable check first**, then minimal implementation, then why, then a scope note. Same
  anatomy, same order.
- **The last step of every phase is still a review-and-refactor pass.** When that's the step
  you land on, this skill applies the punch-list edits itself — but the punch-list, with the
  *why* for each item, still goes in the handoff. A refactor whose reasoning you kept to
  yourself is the least reviewable diff there is.

### 2. Write the test, watch it fail

Write the step's runnable check first and **run it before implementing**. Report the actual
red — the failure message, not a claim of one. This is the same reason `lbb:next` puts the
test first: it makes the target visible and proves the assertion is live. A test that was
never observed failing has proved much less, and here you have no excuse for skipping it,
since you control both halves.

For a step whose check is visual (Dioxus UI you can't unit-test), there's no red to show:
say so plainly, state the specific thing to look for under `dx serve`, and treat the
learner's eyeball confirmation as the gate.

### 3. Write the implementation

Minimal — exactly what turns the test green, nothing more. Match the surrounding code's
idiom, naming, and module layout; use the `dioxus-07` skill for any Dioxus API and the
`rust-best-practices` skill for Rust shape. Write it the way the learner would have been
told to in `lbb:next`'s sketch, not a cleverer version — the point is that the diff matches
the explanation.

Where the step admits more than one reasonable approach, **pick one and note the alternative
in the handoff.** That fork is often the most valuable thing in the step.

### 4. Run the checks

`cargo test` plus `cargo clippy` (there's a `clippy.toml`). Report **real** pass/fail counts
and confirm the *new* test ran green, not just the suite. Any failure you can't resolve
within the step's scope: stop, report it, don't paper over it.

### 5. Clean the diff with `/simplify`

Once the step is green, run the **`simplify`** skill over the working tree. It reviews the
changed code for reuse, simplification, efficiency, and altitude, and applies the fixes.
Quality only — it doesn't hunt for bugs, and it is not a substitute for the checks above.

This exists because a diff the learner didn't write has to earn its readability some other
way. First-draft code that merely passes is the wrong thing to hand someone for review: the
duplicated branch, the intermediate `let` that says nothing, the hand-rolled loop where the
iterator adaptor reads better. `lbb:next` gets that pass for free — the learner writes it,
you review it. Here you are both halves, so the cleanup has to be a deliberate step.

Constraints that override anything the simplify pass proposes:

- **Stay inside the step.** Simplify the code *this step* touched. Cleanups it spots in
  neighboring code are handoff material for a later step, not edits.
- **No comments**, still. If a simplification would need one to be legible, it isn't a
  simplification — keep the plainer version.
- **Don't out-clever the explanation.** The diff has to match the sketch you'd have given in
  `lbb:next`. Reject a change that's terser but harder for the learner to reason about, and
  say in the handoff that you rejected it and why.

Then **re-run `cargo test` and `cargo clippy`** and report the counts again — the numbers
that matter are the ones from after the last edit. If the simplify pass changed anything
non-trivial, name it in the handoff's file-by-file walkthrough; the learner is reviewing the
final shape and shouldn't have to reverse-engineer which parts came from where.

### 6. Record the step in the steps doc — with provenance

Same steps-doc mechanics as `lbb:next`: find or create
`docs/milestones/<NN-milestone>/<NN-feature>/phase-N-<topic>-steps.md`, append the entry
mirroring the chat (test → code → why → scope note), keep the phase doc's checklist in sync.

**Plus a provenance line**, because the steps doc is a build log and the log should be
honest about how each step was built:

```
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.
```

Do not mark the step done — that's `lbb:commit`'s marker, written after it validates.

### 7. The review handoff — the part that carries the learning

End with a handoff built for *critique*, not for applause. Include:

1. **What changed, file by file** — a short walkthrough of the diff in reading order. Say
   which function, which lines, what each piece does.
2. **Why it works** — the mechanism, same bar as `lbb:next`. The subtle bits get named: why
   `move` on that closure, why reading a signal subscribes the component, why `?`
   short-circuits here, where the borrow ends. If you can't explain a line's *why*, that
   line shouldn't be in the diff.
3. **The forks you took** — every place you chose among reasonable options, with the
   alternative and the trade-off. "I used `Vec<TocEntry>` flattened rather than a nested
   tree because the only consumer is a flat sidebar list; a tree would cost a recursive
   render for no current gain." These are the decisions the learner is best positioned to
   overrule, so hand them the fork explicitly.
4. **What to look at hardest** — two or three specific things you'd want a reviewer to
   push on: a signature you're unsure about, an error case you handled one way, a place the
   idiom could go either direction. Point at real uncertainty; a handoff that claims
   everything is obviously right isn't a review request.
5. **Scope note** — what this step deliberately doesn't do and which step picks it up.
6. **Next move** — remind them the diff is uncommitted: review, edit freely (it's their
   code now), then `lbb:commit` when they're happy. Offer the next step, don't start it.

## When to push back

Say something, once, before implementing, if:

- **The step is a poor fit for being written for them** — it's the phase's central concept,
  the thing the whole phase exists to teach (the first `use_signal`, the first custom error
  type). Note that writing it costs them the lesson, and offer `lbb:next` instead. Then do
  what they say.
- **You're not on the current phase.** Same rule as `lbb:next`: name the discrepancy and
  ask. Writing code for the wrong phase is worse than suggesting a step for it.
- **The step can't be sized down to a reviewable diff.** Split it, implement the first half,
  and say so.

One sentence each. They asked; if they reaffirm, build it in full.

## What makes this skill succeed

- The step was derived with the same rigor as `lbb:next` — right phase, one idea, test
  first — so the diff is small enough to actually review.
- The test was written first and **watched fail**, and the reported pass counts are real —
  re-run after the `simplify` pass, not before it.
- The diff went through `simplify` and came back cleaner without getting cleverer, and the
  checks are green *after* that pass.
- The learner can explain, after reading the handoff, *why* each part of the diff works —
  the forks, the trade-offs, and the parts you were unsure about.
- The diff contains no comments, no scope creep, and no cleverness the handoff didn't
  explain.
- Nothing was committed, and the steps doc records honestly that the agent wrote this one.
