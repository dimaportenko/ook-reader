---
name: commit
description: >-
  Validate the current learn-by-building step's hand-written implementation and,
  only if everything is green, commit and push it following the repo's
  conventions, then record a done-status marker in the steps doc. Use this when
  the user says "commit this step", "validate and commit", "lbb:commit", "I'm
  done with the step — check and ship it", or otherwise asks to close out a
  finished step in a learn-by-building project (a from-scratch reimplementation,
  a tutorial-style repo, or any repo whose CLAUDE.md says "don't write code
  unprompted / I'm learning by writing it myself"). Companion to lbb:next. Do
  NOT use this to commit ordinary feature work you wrote yourself, or to commit
  without first validating.
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

Same rule as `lbb:next`. While validating you may **read** any file and **run** the test
suite, and you write only **the steps doc** (the done-status marker) and the **commit**.
You do **not** edit the source files that are the learning target to make a failing test
pass. If validation fails, you *diagnose* and hand back to the learner — you do not silently
fix their code. (A `git add` of source files is fine; rewriting them is not.)

## Procedure

Work through these in order. Stop and report if any gate fails.

### 1. Locate the current step

- Read the relevant phase steps doc (`docs/milestones/<NN-milestone>/<NN-feature>/phase-N-<topic>-steps.md`,
  or the phase doc's "Planned steps" checklist) to find the step in flight — the newest one
  not yet marked done.
- Run `git status` and `git diff` to see what the user actually changed. Cross-check: the
  diff should match the step the doc describes (the new test + the minimal implementation).
  If they diverge, surface that before going further.

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

### 3. Run the checks

- Use the project's actual runner. In this repo that's **`cargo test`** for Rust logic, plus
  **`cargo clippy`** for lint (there's a `clippy.toml`). For a step whose check is a visual
  one, run `cargo check`/`cargo clippy` to confirm it builds clean, then have the user
  confirm the `dx serve` behavior — you can't eyeball the webview for them, so the gate is
  *their* confirmation plus a clean build, and you say so explicitly.
- Report **real** pass/fail counts — never assert "looks good" without running it.
- Confirm the **new** test for this step ran and is green (or, for a visual step, that the
  behavior was confirmed), not just that the suite is green overall. A suite that's green
  because the new test didn't execute is a failure of this gate.

### 4. Gate

- **Any failure, regression, or unexplained diff → STOP.** Do not commit. Diagnose the root
  cause, point the learner at it, and explain it. Let them fix it, then re-run from step 1.
- **All green and the diff is clean → proceed to commit.**

### 5. Commit & push

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
- Push only when the user asked you to (they invoked `lbb:commit`, which means commit **and
  push** per its definition — so push unless they say "commit only"). Report the resulting
  commit hash.

### 6. Record the done-status in the steps doc

After the commit lands, write the provenance marker back into the steps doc for this step,
e.g.:

```
> **Status:** done — committed in `abc1234` (20 tests green).
```

Use the real short hash and the real test count from step 3. Update the step-plan checklist
near the top of the doc too, if the doc keeps one. This is the build-log payoff — the doc
should read, after the fact, as a faithful record of how each step was built and verified.

Then offer the next step (or remind the user they can run `lbb:next`).

## What makes this skill succeed

- No commit ever lands on a red or unverified step — the suite was actually run and the new
  test was actually green.
- The commit message and history match the repo's existing conventions, with **no**
  co-author or AI-attribution trailer.
- Nothing in `src/` was written by you — you validated and recorded, the learner wrote the code.
- The steps doc carries an accurate done-status marker (real hash + real test count) for the
  step you committed.
