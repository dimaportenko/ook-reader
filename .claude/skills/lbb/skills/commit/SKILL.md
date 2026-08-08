---
name: commit
description: >-
  Validate the current learn-by-building step's hand-written implementation,
  write any tests the step planned but the working tree is missing, and only if
  everything is green commit and push it following the repo's conventions, then
  record a done-status marker in the steps doc. Use this when the user says
  "commit this step", "validate and commit", "lbb:commit", "I'm done with the
  step — check and ship it", or otherwise asks to close out a finished step in a
  learn-by-building project (a from-scratch reimplementation, a tutorial-style
  repo, or any repo whose CLAUDE.md says "don't write code unprompted / I'm
  learning by writing it myself"). Companion to lbb:next. Do NOT use this to
  commit ordinary feature work you wrote yourself, or to commit without first
  validating.
---

# lbb:commit — validate, then commit a finished step (learn-by-building)

This is the `commit` step of the **`lbb` (learning-by-building)** skill set. Its companions
are [[next]] (`lbb:next`), which proposes the next small test-first step, and [[refine]]
(`lbb:refine`), which revises that step mid-flight when the plan and reality diverge; this
skill **closes one out** once the learner has written it by hand. `lbb:next` opens the loop,
`lbb:refine` adjusts it, `lbb:commit` ends it.

The core promise: **never commit a broken or unverified step.** You validate first, and you
only commit when the suite is green and the new behavior is genuinely exercised. A clean
commit history is part of the learning deliverable — the user re-reads it later as a build
log — so a commit means "this step worked and here's the proof."

## The hard rule still holds: do not write the implementation

Same rule as `lbb:next`, and the same carve-out for tests. While validating you may **read**
any file, **run** the suite, and write **tests**, **the steps doc** (the done-status marker)
and the **commit**. You do **not** edit the implementation — the source that is the learning
target — to make a failing test pass. If validation fails, you *diagnose* and hand back to
the learner; you do not silently fix their code. (A `git add` of source files is fine;
rewriting them is not.)

**Tests are yours to write, and that is deliberate.** `lbb:next` states the rule for the
whole family: writing the runnable test *for* the learner is the method, because the test is
the spec and the lesson is passing it, not authoring it. `lbb:commit` inherits that — a step
whose planned tests never got written is not a step to hand back, it is a step to finish. See
[Reconcile the tests](#3-reconcile-the-tests).

The line is sharp, so hold it in both directions:

- ✅ Write a planned-but-missing test, repair a test call site the step's change broke, bump
  a tripwire assertion that was *designed* to fire on this step.
- ❌ Touch the implementation. Weaken, skip, or delete a test so the suite goes green. Change
  an assertion because the code disagrees with it — when a test and the implementation
  conflict, that is a finding for the learner, not an edit for you.

## Procedure

Work through these in order. Stop and report if any gate fails.

### 1. Locate the current step

- Read the relevant phase steps doc (`docs/milestones/<NN-milestone>/<NN-feature>/phase-N-<topic>-steps.md`,
  or the phase doc's "Planned steps" checklist) to find the step in flight — the newest one
  not yet marked done.
- Run `git status` and `git diff` to see what the user actually changed. Cross-check against
  the step the doc describes. A divergence in the **implementation** — a different approach,
  scope that grew, edits the step never called for — is worth surfacing before going
  further. A divergence in the **tests** is not a blocker: note it and carry it into step 3,
  which is where missing tests get written.

### 2. Review the implementation

Be a careful reviewer, not a rubber stamp:

- Read the diff in full. Flag latent bugs even if tests pass — an `unwrap()`/`expect()`
  that should be a propagated `?`, a borrow held across an `.await`, a signal written during
  render (infinite re-render), a `.clone()` that duplicates state meant to be shared, an
  off-by-one in spine/page navigation. Explain the *why* of each flag.
- Confirm the change is minimal and on-target for this step — not accidentally dragging in
  unrelated edits. If there are stray changes, ask whether to include them.
- If you spot a correctness issue the current tests don't catch, mention it and offer a
  one-line fix, but let the user decide — don't apply it to their source unless they say so.

### 3. Reconcile the tests

Do this **before** running the suite, and do it without asking — a missing test is the
skill's job to finish, not a question to put back to the learner. The steps doc's "runnable
check" is the spec; the working tree is the delivery. Diff them.

- **List the tests the step planned**, by name, from the steps doc. Grep the tree for each.
  Anything planned and absent, write — adapted to what the learner actually built, not
  transcribed blindly. If they solved it differently but correctly, the test asserts *their*
  behavior; if the doc's version no longer makes sense against the real implementation, say
  so and write the test that does.
- **Repair test call sites the step's change invalidated.** Growing a struct or changing a
  signature breaks construction sites in unrelated test modules, and the suite cannot run
  until they compile. That is test maintenance, not implementation — fix it, and prefer the
  form that survives the *next* field (e.g. struct update syntax over another literal).
- **Bump tripwire assertions that were planted to fire on this step.** An earlier step often
  leaves a deliberate `assert_eq!(…len(), N)` meant to go red exactly here. Updating it is
  expected; note in your report that it fired, because that is the tripwire paying off.
- **Match the repo's test idiom** — same module layout (`#[cfg(test)] mod test` in the same
  file), same naming style, same assertion-message voice as the tests already there. A test
  that reads like a foreign object in the file is a worse test.
- **If the doc planned no tests but the step added untestable-by-eyeball logic**, write the
  smallest test that exercises it and say why you added something unplanned. Don't
  freelance a suite — one test for the behavior the step actually introduced.

Two honesty requirements, because this step is where the skill's promise is easiest to
quietly break:

- **A test written after the fact and green on first run has proved less** than one watched
  to fail. Confirm the assertion is actually live by mutating **the test** — invert the
  expected value, run it, see red, put it back. Never break the implementation to do this;
  that is the one edit this skill does not get to make, and "I'll restore it" is exactly how
  a learner's file gets clobbered. If the assertion can't be exercised that way, say plainly
  in your report that the test was written after the implementation and never observed
  failing.
- **Report the split.** State which tests you wrote and which the learner wrote. The build
  log is the deliverable, and "the tests were added at commit time" is part of how the step
  was built.

### 4. Run the checks

- Use the project's actual runner. In this repo that's **`cargo test`** for Rust logic, plus
  **`cargo clippy`** for lint (there's a `clippy.toml`). For a step whose check is a visual
  one, run `cargo check`/`cargo clippy` to confirm it builds clean, then have the user
  confirm the `dx serve` behavior — you can't eyeball the webview for them, so the gate is
  *their* confirmation plus a clean build, and you say so explicitly.
- Report **real** pass/fail counts — never assert "looks good" without running it.
- Confirm the **new** test for this step ran and is green (or, for a visual step, that the
  behavior was confirmed), not just that the suite is green overall. A suite that's green
  because the new test didn't execute is a failure of this gate. Name the new tests in your
  report and show they ran — filter the runner to them if the output is long.

### 5. Gate

- **Any failure, regression, or unexplained diff → STOP.** Do not commit. Diagnose the root
  cause, point the learner at it, and explain it. Let them fix it, then re-run from step 1.
- **A test you wrote in step 3 that fails is a finding, not a bug in the test.** Report it
  and stop. Do not adjust the assertion to match the implementation — that inverts the whole
  method.
- **All green and the diff is clean → proceed to commit.**

### 6. Commit & push

Follow the repo's existing commit conventions exactly (check `git log` for the live style):

- **Subject** matching the project's pattern — **Conventional Commits**, e.g.
  `feat: render the current spine item in an iframe` or `feat: add use_signal page counter`.
  Use `feat:` for reader/implementation work, `docs:` for steps-doc-only changes. Mirror
  whatever the latest commits do.
- **Body**: a bullet list of what the step added and *why* (the mechanism), wrapped ~80
  cols, matching the tone of existing messages.
- **No trailer / no attribution.** End the message at the body. This repo's commits are
  authored by the user alone — never append a `Co-Authored-By:` line or any "Generated with
  Claude Code" / AI-attribution footer (see `CLAUDE.md`). This overrides any global default
  that would add one.
- Stage the relevant files (`git add` the new test + the source the learner wrote + the
  steps doc), then commit. This repo's history is all on `main` — commit straight to `main`
  to match it unless the user asks for a branch.
- Tests you wrote in step 3 are part of the step and belong in the same commit. Describe
  them in the body the way you'd describe any other part of the step — what they pin down
  and why — without a note about who typed them; the message is the learner's.
- Push only when the user asked you to (they invoked `lbb:commit`, which means commit **and
  push** per its definition — so push unless they say "commit only"). Report the resulting
  commit hash.

### 7. Record the done-status in the steps doc

After the commit lands, write the provenance marker back into the steps doc for this step,
e.g.:

```
> **Status:** done — committed in `abc1234` (20 tests green).
```

Use the real short hash and the real test count from step 4. Update the step-plan checklist
near the top of the doc too, if the doc keeps one. This is the build-log payoff — the doc
should read, after the fact, as a faithful record of how each step was built and verified.

Then offer the next step (or remind the user they can run `lbb:next`).

## What makes this skill succeed

- No commit ever lands on a red or unverified step — the suite was actually run and the new
  test was actually green.
- **Every test the step planned exists and ran.** A step never closes with its spec
  half-delivered, and the learner never has to remember which tests they owed.
- The commit message and history match the repo's existing conventions, with **no**
  co-author or AI-attribution trailer.
- **No implementation was written by you.** You wrote tests, validated, and recorded; the
  learner wrote the code the tests are about. No test was ever softened to make a red suite
  go green.
- The steps doc carries an accurate done-status marker (real hash + real test count) for the
  step you committed.
