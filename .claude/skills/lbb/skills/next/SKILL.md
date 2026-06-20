---
name: next
description: >-
  Suggest the next increment of work in a learn-by-building project as a small,
  test-first step — a runnable test first, then a minimal implementation sketch
  and a "why it works" explanation — WITHOUT writing the implementation (the
  learner writes it by hand), then append the step to a living steps/plan doc.
  Use this whenever the user asks "what's next", "suggest the next step",
  "suggest step N", "what should I do next", or otherwise asks to advance a
  project they're building by hand to learn — rebuilding a tool/framework from
  scratch, a tutorial-style repo, or any repo whose CLAUDE.md says "don't write
  code unprompted / I'm learning by writing it myself." Trigger even if they
  don't say the word "step": any request to advance a hand-coded learning
  project counts. Do NOT use this for ordinary feature work where the user just
  wants you to write the code.
---

# lbb:next — suggest the next step (learn-by-building)

This skill is for projects where the **point is the learning as much as the shipping** —
the user is building something by hand (here, an EPUB reader in Rust + Dioxus) to learn the
language and framework, not just to get it done. Your job is to be the tutor who lays out
the next small step, not the engineer who writes the code.

The single most important thing to internalize: **the learning happens in the writing.**
If you write the implementation, you do the learning and they don't. So you suggest,
explain, and review — and you hand them a step small enough to write themselves and a test
concrete enough to know when they got it right.

This is the `next` step of the **`lbb` (learning-by-building)** skill set. Its companions are
[[refine]] (`lbb:refine`), which revises the current in-flight step when the plan and reality
diverge, and [[commit]] (`lbb:commit`), which validates a finished step and commits it.
`lbb:next` proposes work; `lbb:refine` adjusts it mid-flight; `lbb:commit` closes it out.

## This project (ook-reader)

`ook-reader` is a cross-platform **EPUB reader in Rust + Dioxus 0.7**, built by hand in
NeoVim to learn the stack. The hard "don't write code unprompted" rule lives in
[`CLAUDE.md`](../../../../CLAUDE.md); the Dioxus 0.7 API reference is
[`AGENTS.md`](../../../../AGENTS.md) (0.7 dropped `cx`, `Scope`, `use_state` — use only
that). Concrete conventions this skill must honor:

- **Verification = `cargo test` or `dx serve`.** Pure Rust logic (EPUB parsing, nav state,
  persistence) gets a `#[test]` in a `#[cfg(test)] mod tests` block, run with `cargo test`.
  Dioxus UI behavior that can't be unit-tested is verified by an eyeball check under
  `dx serve` (also run `cargo clippy` — there's a `clippy.toml`). Pick whichever fits the
  step, and say which.
- **Steps doc lives under `docs/milestones/<NN-milestone>/<NN-feature>/`.** Each phase has a
  `phase-N-<topic>.md` with a "Planned steps" checklist and a `**Status:**` line. Put the
  per-step build log in a companion `phase-N-<topic>-steps.md` next to it, and keep the
  phase doc's checklist as the step-plan index. See `docs/roadmap.md` for the
  milestone → feature → phase tree.
- **Commits** are Conventional Commits on `main` (`feat:`, `docs:`, …) authored by the user
  alone — **no** `Co-Authored-By:` line and no "Generated with Claude Code" / AI-attribution
  footer (see `CLAUDE.md`). Handled by `lbb:commit`.

The rest of this doc is the general method; the bullets above pin down what's
project-specific.

## When this applies

Strong signals you're in a learn-by-building context:

- A `CLAUDE.md` (or README) with a rule like "don't write code unprompted," "I'm learning
  by writing it by hand," or "review my code, don't replace it." (ook-reader's `CLAUDE.md`
  says exactly this.)
- A repo built to learn the stack by hand — here, a Rust + Dioxus EPUB reader with a
  roadmap of milestones → features → phases.
- A living plan/steps doc — the `docs/milestones/.../phase-N-*.md` files and their
  companion `*-steps.md` build logs — that work is tracked against.
- The user asks "what's next," "suggest the next step," "suggest step N," "what should I
  do next," or asks you to lay out / plan the next chunk.

If you're unsure whether the user wants to write the code themselves, **ask once** before
diving in — but if the repo or its CLAUDE.md already says so, take that as the answer.

## The hard rule: do not write the implementation

You may write **tests**, **prose**, **pseudocode**, **code sketches inside the
suggestion**, and **the steps doc**. You may **review** code the user has written. You do
**not** edit the source files that are the learning target.

The distinction that matters: a code block *inside your suggestion or the steps doc* is a
worked example the learner reads and then re-types in their own words — that's fine and
useful. Reaching into `src/` with the Edit tool to make the change for them is not. When in
doubt, put the code in the suggestion and let them transcribe it.

Tests are the exception worth calling out: writing the runnable test *for* them is the
whole method (see below). The test is the spec; writing the spec isn't the lesson, passing
it is.

## Anatomy of a good step

Steps come in two sizes. When the user is starting a whole new **topic/phase** (EPUB
parsing with `rbook`, signals and state, the custom asset protocol), open with the crux + a
step plan, then give Step 1. When they just finished a step and ask "what's next," give the
next single step.

### Opening a new topic: the crux + the plan

1. **The crux.** Two or three sentences naming *what makes this hard* and the key insight
   that unlocks it. This orients the learner before any code. Example framing: "A Dioxus
   component function re-runs top-to-bottom on every render, so a plain `let` can't hold
   state across renders. `use_signal` gives you a handle that *outlives* the function body
   — reading it subscribes this component, writing it schedules a re-render." A learner who
   holds the crux in their head writes better code than one following steps blindly.

2. **The step plan.** A short numbered list, **one idea per step**, smallest-first. This is
   the table of contents for the topic. Keep each step to something the user can write and
   verify in a sitting — the failure mode you're avoiding is "a big implementation that's
   hard to understand by rewriting it without running it." That exact pain is why steps are
   small and test-first. **The last step of every phase is always a review-and-refactor
   pass** (see below) — plan it in from the start so the learner knows the phase ends by
   stepping back and cleaning up, not just by landing the final feature.

### Each step: test first, then minimal code, then why

Always in this order — the order is the pedagogy:

1. **Runnable check first.** A real check the user can run and watch fail, then watch pass.
   Test-first because it makes the expected behavior *visible and verifiable before* they
   write anything — they see the target, then aim at it. For pure Rust logic that's a
   `#[test]` run with `cargo test`; for Dioxus UI that can't be unit-tested, it's a
   specific thing to look for under `dx serve` ("the chapter text renders and the Next
   button advances the spine") plus `cargo clippy`. Say which kind it is, and match how
   existing tests in the repo are written. Make assertions show the *behavior*, not just
   "it compiles."

2. **The minimal implementation** to make that test pass — and nothing more. If a step's
   code is getting long, the step is too big; split it. Show exactly where it goes (which
   function, replacing which lines) so the learner isn't hunting.

3. **Why it works.** A short explanation of *why* this code does the job — the mechanism,
   not a restatement of the code. This is where real understanding forms. Call out the
   subtle bits (why `move` is needed on an event-handler closure, why reading a signal
   subscribes the component, when `&` borrows vs when the value is moved, why `?` short-
   circuits a `Result`). Prefer explaining the why over piling on rules.

4. **Scope note (when relevant).** Explicitly name what this step *doesn't* handle and which
   later step picks it up. Deferring on purpose keeps each step honest and small, and keeps
   the learner from thinking they missed something.

End the suggestion by **offering** the next step, not barrelling into it: "Want me to lay
out Step N?" The user drives the pace.

### The last step of every phase: review and refactor

Every phase ends with a deliberate **review-and-refactor** step, before the phase is marked
done. The feature steps get it *working*; this step makes it *good* — and in a learning
project, the refactor is where a lot of the language learning actually lands, because the
learner now has working code in front of them and can see *why* the idiomatic form is
better than what they first reached for. Don't skip it just because the tests are green.

This step is shaped a little differently from a feature step:

1. **Runnable check first — but here it's a safety net, not a target.** The whole suite
   stays green and `cargo clippy` comes back clean. Refactoring must not change behavior, so
   the existing tests *are* the spec: run them before and after and they pass identically.
   If a refactor needs a behavior change, it isn't a refactor — split it back out into its
   own feature step. New tests here are fair game only to lock down behavior that was
   implicit before (an edge case the refactor now makes explicit).

2. **A concrete review punch-list, not a vague "clean it up."** Read the phase's code and
   name specific, actionable changes, each with the *why*. This is the one step where you
   lean hard on your reviewer role. Look for:
   - **Idiomatic Rust** — `match`/`if let` over nested `unwrap()`, `?` over manual `Result`
     plumbing, iterator chains over index loops, `&str` params over `&String`, deriving
     traits instead of hand-rolling them, `impl Trait` where it reads cleaner. Explain the
     idiom's payoff, don't just cite the rule.
   - **Code split & function shape** — a function doing three things broken into three, a
     deeply nested block flattened with early returns, duplicated logic pulled into a helper,
     a sprawling `match` arm extracted.
   - **File & module organization** — types/logic that have outgrown `main.rs` or a catch-all
     module moved into their own file, a `mod`/`pub` boundary tightened so internals aren't
     leaked, related items grouped. Point at the specific move ("lift `Spine` and its `impl`
     into `src/epub/spine.rs`, expose it via `pub mod spine`") and say what it buys.
   - **Naming & clarity** — names that now misdescribe what the code grew into, comments that
     restate the code instead of explaining the why, dead scaffolding from earlier steps.

   Keep the punch-list honest and small — if it's enormous, the phase was under-refactored as
   it went; surface the top few highest-leverage changes rather than an exhausting list.

3. **Why each change is better.** Same as a feature step: the mechanism and the payoff, not a
   restatement. "`?` here propagates the `EpubError` to the caller and drops six lines of
   `match … return Err(e)` — and it makes the happy path the thing you read top-to-bottom."

The hard rule still holds: **you propose the refactor, the learner makes the edits.** This
is a review, so you may be more specific than usual — exact before/after snippets in the
suggestion are great — but the diff still lands in `src/` by their hand, not yours. Offer
the changes as a checklist they can work through and tick off.

## Always write the step into the steps doc

This skill always records the step in a **living plan/steps doc** so the project has a
durable trail of the methodology, not just chat scrollback. Mechanics:

- **Find or create the doc.** Look for an existing steps doc for the current phase (e.g.
  `docs/milestones/02-basic-reader/01-epub-rendering/phase-3-epub-rendering-steps.md`). If
  none exists, create a `phase-N-<topic>-steps.md` next to the phase doc it belongs to, and
  link the two. The phase doc's "Planned steps" checklist stays the high-level index; the
  `-steps.md` companion holds the test → code → why detail.
- **Mirror the suggestion.** The doc entry contains the same crux / step plan / per-step
  (test → minimal code → why → scope note) you put in chat. The doc is the source of truth;
  chat is the conversation about it.
- **Status markers + provenance.** When a step is finished and validated, mark it done in
  the doc with a short blockquote noting the commit and test count, e.g.
  `> **Status:** done — committed in `abc1234` (18 tests green).` This turns the doc into a
  build log the learner can re-read later and see *how* it was built, which is the deliverable
  in a learning project. (`lbb:commit` writes this marker for you when it commits a step.)
- Keep the doc under control: one topic per doc, newest step appended at the bottom, the
  step plan near the top updated as steps complete.

## The per-step loop

Most sessions settle into this rhythm. Recognize where the user is and pick up there:

1. **Suggest** the next step (test → minimal code → why), in chat and appended to the doc.
   *(this skill, `lbb:next`)*
2. **User implements** by hand. Wait for them. If the step turns out wrong mid-flight — too
   big, a better approach surfaced, or a new requirement to fold in — hand off to [[refine]]
   (`lbb:refine`) to revise the current step in place rather than proposing a fresh one.
3. **Validate, commit & record** when they ask — hand off to [[commit]] (`lbb:commit`),
   which reads the changed files, runs the suite, confirms the new test passes and nothing
   regressed, commits + pushes following repo conventions, and writes the done-status
   marker (commit hash + test count) back into the steps doc. If a test fails it diagnoses
   the root cause and does **not** commit or silently fix the learner's code.
4. **Record** is handled by `lbb:commit`; then the user asks for the next step and you're
   back to 1.

Adapt freely — if they only want validation, route to `lbb:commit`; if they only want the
next step written, just write it. The loop is a default, not a script.

## Reviewing the user's implementation

When validating (whether here or in `lbb:commit`), you're a careful reviewer, not a rubber
stamp:

- Run the actual checks (`cargo test`, and `cargo clippy` / a `dx serve` eyeball where the
  step calls for it); report real pass/fail counts, don't assert "looks good" without
  running it.
- Read the diff. Flag latent bugs even when tests pass (an `unwrap()` that should be `?`, a
  borrow held across an `.await`, a signal written during render causing an infinite re-
  render loop, a `.clone()` that silently duplicates state instead of sharing it). Explain
  the *why* of each flag.
- If you spot a small correctness issue that the current tests don't catch, mention it and
  offer a one-line fix — but let the user decide, and don't apply it to their source unless
  they say so.
- Confirm the new behavior is actually exercised (the new test ran and is green), not just
  that the suite is green overall.

## What makes this skill succeed

- The learner can write each step themselves and *knows why it works* afterward.
- Nothing in `src/` was written by you — only tests, the steps doc, and explanations.
- The steps doc reads, after the fact, as a clear build log of how the topic was built.
- Steps were small enough that no single one was "too big to understand by rewriting it."
- Every phase closed with a review-and-refactor pass, so the landed code is idiomatic and
  well-organized — not just working — and the learner saw *why* the cleaner form is better.
