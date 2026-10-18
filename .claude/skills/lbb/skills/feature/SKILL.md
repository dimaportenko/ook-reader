---
name: feature
description: >-
  Take a feature request from one sentence to a reviewable pull request in a
  learn-by-building project, without the learner writing the code: research
  the external facts, settle the few real decisions with the user, plan it as
  a new phase in the docs, then orchestrate one subagent per step (each doing
  lbb:next-implement + lbb:commit, committing on a feature branch, never
  pushing), verify each hand-back, and finish by pushing the branch and opening
  a PR the user reviews as a whole. Use when the user says "lbb:feature",
  "add <feature>, do the whole thing", "implement the whole phase without me,
  I'll review the result", "quick add <provider/feature>", or otherwise asks
  for an entire feature or phase built end to end for review. Do NOT use for
  a single step (lbb:next / lbb:next-implement), for revising the step in
  flight (lbb:refine), or when the learner wants to write the code.
---

# lbb:feature — a whole feature, planned, built by subagents, shipped as a PR

This is the **orchestrator** of the `lbb` (learning-by-building) skill set. The others work
one step at a time: [[next]] proposes a step, [[next-implement]] writes one, [[refine]]
revises one, [[commit]] ships one. `lbb:feature` runs that loop **for a whole phase**, with
each step built by a subagent and the user reviewing the result once, as a PR.

It trades the most learning of any skill in the family for the most speed. The learning has
to move somewhere: into the **build log** (each step still gets its full test → code → why
entry) and into the **PR**, which the user reads end to end. Both must be as complete as
they'd be for a hand-built phase.

**Only run this when the user explicitly asked for the whole thing built.** "Add X" on its
own is a request to plan; offer this mode and wait.

## Roles

- **You (the orchestrator)** research, ask, write the plan docs, start subagents, check
  their work, open the PR. You keep your own context small. You read hand-back reports, not
  diffs or transcripts.
- **Step subagents** each build exactly one step: derive it from the docs, write the test
  and watch it fail, implement, clean up, commit. They return a short report.

## Procedure

### 1. Research the facts the plan depends on

A feature request often names something you can't verify from memory: a model id, an API,
a library version ("the latest luna model"). Look it up first (WebSearch/WebFetch, the
`find-docs` skill). Get the exact id, the endpoints, the parameters and their defaults,
and anything that limits the design. For example, does the stream carry reasoning? What's
the default effort? Cite the sources to the user.

Then read the code that the feature touches, enough to see whether it's **a new host for
something that exists** (reuse) or genuinely new. That finding is usually the phase's crux.

### 2. Settle the real decisions in one question round

Ask **one** `AskUserQuestion` round with only the forks that change the plan. Put the
recommended option first. Typical ones:

- **Placement:** a new phase in its own feature folder (usually right), or folded into the
  phase in flight. Never quietly bolt a new feature onto an unrelated phase's last step.
- **Scope:** for example, a fixed list of models vs. a live catalog, or one model vs.
  several.

Pick sensible defaults for everything else, and list them in the plan so the user can
overrule them.

### 3. Plan it in the docs (the `lbb:refine` / `lbb:next` shape)

Under `docs/milestones/<NN-milestone>/<NN-feature>/`, write the following, matching the
neighboring phases' style:

- **`README.md`:** what the feature is, the dated research findings with links, and the
  risks.
- **`phase-N-<topic>.md`:** goal, crux, design decisions, and a "Planned steps" checklist.
- **`phase-N-<topic>-steps.md`:** crux, the step plan, and *why this order*. Step details
  are left for the step subagents to write.

Then update the milestone README: add a phase row, a status line, and a dated note on where
the phase sits in the order.

Plan the steps by the [[next]] rules: **order by observability** (entry point and settings
first, routing before the real service, extract the abstraction alongside its second caller),
one idea per step, a **measure-before-tuning** step where performance is the goal, and
**review and refactor last**.

### 4. Prepare to run

- **Branch first.** Bring `main` up to date, then cut `feature/<slug>` or `phase-N-<slug>`.
  Commit the plan docs on it as `docs: plan Phase N, …`. All the work lands on this branch,
  and nothing touches `main`.
- **Credentials.** If a step needs a live key, check for it with
  `[ -n "$VAR" ] && echo set || echo not set`. Never print, `curl` or echo a key. If it's
  missing, tell the user how to add it (`! echo 'export VAR=…' >> ~/.zshenv`) and keep
  going. Without it, the live tests stay `#[ignore]`d, measurement steps fall back to a
  documented guess, and both are flagged as unmeasured.
- **Keep the Mac awake** if the user asks: `caffeinate -dimsu -t 21600` in the background,
  and stop it at the end.

### 5. One subagent per step, strictly in order

Steps depend on each other, so run them **sequentially**, each in the background, and wait
for each hand-back. Set `model` explicitly; multi-file implementation work needs the strong
model. Each prompt must be self-contained:

- Read and follow `.claude/skills/lbb/skills/next-implement/SKILL.md`, then
  `.claude/skills/lbb/skills/commit/SKILL.md`, with these overrides: **commit on the current
  branch, never push, never touch `main`**. Use Conventional Commits with a subject, a blank
  line, then a bullet body. **No `Co-Authored-By` and no AI attribution.**
- Make two commits per step: `feat:`/`refactor:` (code plus the step's build-log section),
  then `docs: record Phase N Step K as done` (real hash and real test count). Point the
  subagent at the previous step's two hashes as the template.
- Name the step, its docs, the files involved, and the **forks the previous steps left for
  it**. Those are carried in the hand-back reports, and passing them along is your job.
- For a live or measurement step: name the env var, give the scratchpad path for throwaway
  scripts, and say "median of ≥3 runs, recorded as a table, decide from data, and replan
  honestly if the data says so".
- For the final review step: run the `code-review` skill (high effort) over the whole phase
  range, fix confirmed bugs, apply the deferred cleanups that pay off and record the rest as
  skipped with reasons, then mark the phase closed. **Hand checks still pending stay marked
  unconfirmed. They are never claimed.**
- Ask for a **short report** (under 15 lines): commit hashes, red then green test counts,
  files touched, forks taken, concerns. No diffs.

After each hand-back, give the user a few lines: what landed, the numbers, and anything
surprising. Then start the next step.

### 6. Verify before you ship

Don't relay a subagent's numbers as your own. Run them yourself:

```sh
git status -sb; git log --oneline main..HEAD
git log main..HEAD --format=%B | grep -ci "co-authored\|generated with"   # must be 0
cargo test 2>&1 | grep "^test result"
cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | grep -v "future version"
```

Check for commits on the branch that you didn't start, such as one the user made by hand.
Include them in the PR and name them.

### 7. Open the PR

Push the branch and `gh pr create --base main`. Write the body so it stands on its own as
the review guide:

- what the feature does, plus a link to the steps doc
- one line per step
- **review points:** the forks a reviewer is best placed to overrule
- **testing:** the real counts, which live tests ran and which didn't, and **checkboxes for
  the hand checks** (`dx serve`, device) that nobody has done yet

No AI attribution footer. Report the PR URL, and stop `caffeinate` if it's running.

## Resuming after a crash

If the session dies partway through, the docs and git hold the whole state. Don't trust the
earlier conversation's sense of progress:

- `git log --oneline main..HEAD` shows which steps have both commits.
- The steps doc's step plan shows which steps are struck through and marked **Done**.
- `git status` shows whether a half-finished step left a dirty tree. Inspect it and finish
  it or reset it. Never re-run a step that's already committed.

Start `caffeinate` again if it was running, then start the next step.

## What makes this skill succeed

- The plan rested on facts you looked up, not ones you remembered, and the user made the
  two or three decisions that were theirs.
- Every step has the full build-log entry, a test watched failing, two commits, and real
  numbers, the same as a hand-built phase.
- Nothing landed on `main`, nothing was pushed before the PR, and no commit or PR carries
  AI attribution.
- Your context stayed small: you read hand-back reports and verified with commands, not
  diffs.
- The PR is a complete review guide, and every check nobody has done is a visible,
  unticked checkbox.
