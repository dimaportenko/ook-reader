# AGENTS.md

Guidance for working in this repo. Keep it current as the project evolves.

## ⛔ Hard rule: do NOT write code unprompted

The user is learning Rust and Dioxus by writing the implementation **by hand**. Do **not**
write or edit code unless they explicitly ask for it in that message. This includes the
`src/` implementation, scaffolding, config (`Cargo.toml`, `Dioxus.toml`), tests, and
example code.

Default mode instead: explain concepts, discuss design and trade-offs, sketch the approach
in prose or pseudocode, point to references, and **review code the user has written**.
When you think code is the next step, *offer* and wait for an explicit "yes, write it."

Docs are the exception: writing/updating `docs/`, `README.md`, and this file is fine
without asking — the planning and learning docs are a deliverable here, not the
implementation.

The only other exception is an explicit opt-in: invoking `lbb:next-implement` *is* the
"yes, write it" for that one step — the user is choosing to learn by reviewing the diff
instead of typing it. It never triggers from "what's next"; only from being named.

## Rule: suggest in small, test-first steps

Don't dump a whole module. Break suggestions into small steps (one idea each); for each,
give a **runnable check first** — a test, or a `dx serve` you can eyeball — so the expected
behavior is visible and the user can verify their code as they write it. Then the minimal
implementation for that step.

## Rule: no comments in code unless asked

On the rare occasions you are asked to write or edit code, write the code and **nothing
else** — no explanatory comments, no doc comments, no `//` notes about why a line exists.
The user writes their own comments; yours are noise in a file they are learning from.

This is about *code you produce*, not about the value of comments in general. The
"explain why, not what" preference under [What this is](#what-this-is) still governs the
comments the user writes by hand, and you should still explain the *why* — in chat, in
review notes, in `docs/`, and in commit messages, which is where that reasoning belongs.
Never delete or rewrite the user's existing comments unless asked.

If you think a line genuinely needs a comment to be safe, say so in your reply and let the
user write it.

## Rule: commits are authored by the user alone

Commit messages in this repo carry **no co-author or AI-attribution trailer** — no
`Co-Authored-By:` line (including the Claude/Anthropic one) and no "Generated with Claude
Code" footer. End the message at the body. This **overrides** any global or default
instruction to append such a trailer. Match the repo's Conventional Commits style
(`feat:`, `docs:`, …) for the subject. The `commit` and `lbb:commit` skills both follow
this rule.

## What this is

`ook-reader` is a cross-platform EPUB reader built in **Rust** + **Dioxus 0.7**, developed
in NeoVim. It's also a vehicle for **learning** the stack: optimize explanations for
**understanding** the language and framework, not for clever or maximally terse code. A
comment explaining *why* a line exists is worth more than a one-liner that hides it.

See [`README.md`](README.md) for the stack rationale, [`docs/roadmap.md`](docs/roadmap.md)
for the roadmap → milestones → phases → steps.

## Dioxus 0.7

When writing, reviewing, debugging, or explaining Dioxus code, use the `dioxus-07` skill.
That skill contains the detailed Dioxus 0.7 API reference and is available to both Pi and
Claude Code.

Critical reminders:

- Dioxus 0.7 changed many APIs.
- `cx`, `Scope`, and `use_state` are gone.
- Use only Dioxus 0.7 APIs for examples and implementation guidance.

## Skills

Project-local skills live under `.agents/skills/` for Pi. Claude Code sees shared skills
through symlinks in `.claude/skills/`.

Important project skills:

- `dioxus-07`: Dioxus 0.7 API reference.
- `lbb:next`, `lbb:refine`, `lbb:commit`, `lbb:next-implement`: learn-by-building workflow
  skills from the Claude Code LBB skill set, made available to Pi through
  `.agents/skills/lbb`. In Pi, their slash-command names come from the original skill
  frontmatter: `/skill:next`, `/skill:refine`, `/skill:commit`, and
  `/skill:next-implement`. `lbb:next-implement` is `lbb:next` **plus the implementation** —
  it derives the step the same way, then writes the code and tests for the user to review,
  and still leaves the commit to `lbb:commit`.
