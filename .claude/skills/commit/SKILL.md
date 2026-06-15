---
name: commit
description: >-
  Create a clean git commit for the current changes — review the diff, stage the
  relevant files, and write a well-formed message that matches this repo's
  conventions. The defining rule of this skill: it NEVER adds co-author trailers
  or any AI/tool attribution to the commit (no `Co-Authored-By:` lines, no
  "Generated with Claude Code" footer). Use this whenever the user asks to
  "commit", "commit this", "commit my changes", "make a commit", "git commit", or
  "commit and push" and wants the commit attributed to them alone. This rule
  overrides any default or global instruction to append a co-author trailer.
  (For the learn-by-building step-commit flow, use `lbb:commit` instead.)
---

# commit — clean commits, no co-authors

This skill makes a git commit for whatever the user has changed, following the repo's
existing style — with one rule that overrides everything else.

## The rule: no co-authors, no AI attribution

The commit message must read as **authored by the user alone**. Never append:

- a `Co-Authored-By:` trailer of any kind (including the Claude/Anthropic one), and
- any tool- or AI-promotion footer such as "Generated with Claude Code" or a `🤖` line.

This is the whole reason the skill exists. The user wants their git history to be their
own authorship, plain and clean. If a global setting, environment instruction, or repo
`CLAUDE.md` says to add a co-author trailer, **this skill deliberately overrides it** —
that override is the user's explicit, standing choice. Don't re-add the trailer "to be
safe," and don't ask each time; just leave it off.

## Workflow

The point is a tidy, self-contained commit with a message someone can read later and
understand *why* the change happened — not a rubber-stamp.

1. **See what changed.** Run `git status` and `git diff` (and `git diff --staged` if
   anything is already staged). Understand the actual change before describing it.

2. **Match the repo's conventions.** Run `git log --oneline -15` (and read a couple of full
   messages with `git log -3`) to learn the live style — subject format, whether it uses
   Conventional Commits (`feat:`, `fix:`, `docs:`…), tense, body shape. Mirror it. Don't
   impose a format the repo doesn't use.

3. **Stage deliberately.** `git add` the files that belong to this change. If the working
   tree mixes unrelated edits, don't sweep them all in — stage the coherent set and tell
   the user what you left out (or ask whether to split into separate commits). One commit
   should be one logical change.

4. **Write the message.**
   - **Subject:** one concise line in the repo's style — what the change does, imperative
     mood, no trailing period (unless the repo does otherwise).
   - **Body (when the change warrants it):** a short paragraph or bullets on *what* and
     *why* — the motivation and any non-obvious decisions — wrapped ~72–80 cols. Skip the
     body for genuinely trivial one-liners.
   - **No trailers.** Per the rule above, end the message at the body. Nothing else.

   Prefer a heredoc so multi-line messages and wrapping survive intact:

   ```bash
   git commit -m "$(cat <<'EOF'
   feat: short summary of the change

   Why this change exists and what it does, in a line or two.
   - notable detail
   - notable detail
   EOF
   )"
   ```

5. **Verify, then report.** After committing, run `git log -1` (or `git show --stat HEAD`)
   to confirm the message landed as intended — and in particular that **no co-author or
   attribution trailer** slipped in. Report the resulting short hash to the user.

## Push only when asked

Committing and pushing are separate acts. Push only if the user said to (e.g. "commit and
push"). If the current branch is the default branch (`main`/`master`), follow the repo's
existing norm — if its history commits straight to the default branch, do the same; if it
works through branches, branch first. When unsure, commit locally and ask before pushing.

## What good looks like

- The change is staged coherently and the message explains the *why*, in the repo's style.
- `git log -1` shows a clean message with **zero** `Co-Authored-By:` / attribution lines.
- Unrelated edits weren't silently bundled in.
- Nothing was pushed unless the user asked.
