# CLAUDE.md

Guidance for working in this repo. Keep it current as the project evolves.

See [`AGENTS.md`](AGENTS.md) for the Dioxus 0.7 API reference (use only that — 0.7 changed
every API; `cx`, `Scope`, and `use_state` are gone).

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

## Rule: suggest in small, test-first steps

Don't dump a whole module. Break suggestions into small steps (one idea each); for each,
give a **runnable check first** — a test, or a `dx serve` you can eyeball — so the expected
behavior is visible and the user can verify their code as they write it. Then the minimal
implementation for that step.

## Rule: commits are authored by the user alone

Commit messages in this repo carry **no co-author or AI-attribution trailer** — no
`Co-Authored-By:` line (including the Claude/Anthropic one) and no "Generated with Claude
Code" footer. End the message at the body. This **overrides** any global or default
instruction to append such a trailer. Match the repo's Conventional Commits style
(`feat:`, `docs:`, …) for the subject. (The `commit` and `lbb:commit` skills both follow
this rule.)

## What this is

`ook-reader` is a cross-platform EPUB reader built in **Rust** + **Dioxus 0.7**, developed
in NeoVim. It's also a vehicle for **learning** the stack: optimize explanations for
**understanding** the language and framework, not for clever or maximally terse code. A
comment explaining *why* a line exists is worth more than a one-liner that hides it.

See [`README.md`](README.md) for the stack rationale, [`docs/roadmap.md`](docs/roadmap.md) for the roadmap → milestones → phases → steps.
