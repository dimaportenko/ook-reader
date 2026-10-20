---
name: feature-with-grill
description: >-
  lbb:feature with a grilling round in front: before planning, walk the
  feature's design tree with the user in AskUserQuestion rounds until every
  decision is settled and they say go, then run lbb:feature end to end
  (plan docs, one subagent per step on a branch, verify, PR). Use when the user
  says "lbb:feature-with-grill", "grill me then build it", "/grill-me" together
  with "lbb:feature", or asks for a whole feature built but wants the design
  decisions stress-tested first. Do NOT use when the decisions are already
  settled (plain lbb:feature), for a single step (lbb:next /
  lbb:next-implement), or when the learner wants to write the code.
---

# lbb:feature-with-grill — settle every decision, then build the whole feature

[[feature]] asks **one** question round and picks defaults for the rest. That's right when
the feature is small or obvious. This skill is for when it isn't: a one-sentence request
("ai response is markdown text, add render support for chat bubbles") that hides a handful of
real forks (parser vs HTML string, which messages, what links do). It runs a full grilling
first, then hands the settled design to [[feature]] unchanged.

The two halves have a hard boundary: **nothing is written, branched or committed until the
user confirms the shared understanding.** The grilling round is read-only.

## Phase A — Grill

### A1. Facts first, so every question is about a decision

Finding facts is your job. Before the first question, look at the code the feature touches,
just enough to word the options concretely. For example: where the bubble renders, which
crates are already deps, which milestone and phase numbers come next. Do that yourself with
a couple of `grep`/`sed` calls, since it's cheap.

If a question depends on an external fact, such as a crate version, how the framework
handles a link click, or a model id, start a subagent to look it up (`model: "sonnet"`, or
`"haiku"` for a single lookup) and **don't wait for it**. Ask now the questions that don't
depend on that fact. The ones that do wait for the next round. Facts that would only shape
the implementation, not a user decision, go straight to Phase B's research, and you tell the
user that's where they went.

### A2. Map the design tree, ask the frontier

Every decision branches into the ones that hang off it. The **frontier** is every decision
whose prerequisites are already settled. Ask the whole frontier at once, and never ask a
question whose answer depends on another question still open in the same round.

**Ask through `AskUserQuestion`, never as numbered text.** The user asked for this. One
call holds at most 4 questions, so a bigger frontier is split into several calls made one
after another. Each question:

- puts your **recommended option first**, marked `(Recommended)`;
- has 2–4 mutually exclusive options, each with a one-line trade-off;
- gets a `preview` (a code sketch, a type shape, a CSS idea) when the options are
  code-shaped and easier to compare side by side.

The frontier usually holds these, so check each one:

| Branch | Typical fork |
|---|---|
| **Approach** | the shortcut vs the typed/structured way, e.g. HTML string + `inner_html` vs a parsed tree rendered as RSX |
| **Scope of effect** | which inputs or screens it applies to, e.g. assistant messages only vs both |
| **Scope of features** | what's in and what's explicitly out, e.g. CommonMark + tables, with no highlighting |
| **Live/streaming behaviour** | what happens to partial data mid-stream |
| **Safety / external effects** | links, network fetches, untrusted input |
| **Styling / layout** | theme variables, overflow on small screens |
| **Placement** | a new phase in its own folder vs folding it into another phase, and where it sits in the milestone order |
| **Build mode** | build it all and open a PR (→ Phase B) vs write only the plan for hand-building with [[next]] |

### A3. Recompute, repeat, then confirm

After each answer, the settled decisions push the frontier outward, so ask the next round.
Stop when the frontier is empty. Then send **one short message** listing every settled
decision as a single line each, plus the facts that are deferred to research, and ask:
*shared understanding? say go.* **Wait for the go.** If the user changes something, update
the tree and confirm again.

If the user chose *plan only*, run Phase B through step 3 of [[feature]] (the plan docs and
the commit on a branch), then stop.

## Phase B — Build: run lbb:feature

Read `.claude/skills/lbb/skills/feature/SKILL.md` and follow it from step 1, with these
changes:

- **Step 1 (research):** look up only the facts deferred from the grilling. The code read is
  already done, so don't read it again.
- **Step 2 (question round):** **skip it.** The grilling settled it. Ask again only if the
  research turns up a fact that breaks a settled decision (for example, the chosen crate
  can't do X). Ask about that one fork alone, in one `AskUserQuestion`.
- **Step 3 (plan docs):** the phase doc's *Design decisions* section records each settled
  decision, with the note *"settled in a grilling round on <date>"*. Anything you picked
  yourself is marked *(default, overrule in review)*. The README's research section cites
  the subagent's sources with the date.
- **Step 5 (subagents):** each step's prompt passes on the **forks** the previous hand-backs
  left. Keep a running **deferred list** of the forks hand-backs flag ("SoftBreak as newline
  vs space", "mailto on iOS", "simplify not run"). Give the whole list to the review step,
  which must apply each item or record it as skipped with a reason.
- **Step 7 (PR):** the *Review points* section names the decisions made in the grilling
  round that a reviewer is best placed to overrule, as well as the forks the steps took.

Everything else in [[feature]] holds as written: branch first, two commits per step,
verifying the numbers yourself, no AI attribution, and unticked hand-check boxes.

## What makes this skill succeed

- No question in the grilling round asked the user for a fact you could have looked up, and
  no round asked a question that depended on another one still open.
- Every question went through `AskUserQuestion` with the recommendation first.
- Nothing was touched before the user said go.
- The phase doc's decisions match the grilling answers one for one, so the build never
  quietly re-decided anything.
- The review step received every deferred fork, and the PR lists the ones still open.
