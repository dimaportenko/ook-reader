# Phase 2 — Dioxus Project Scaffold

[← Feature: Project Scaffold](README.md) · **Status:** ⬜ planned

## Goal

Create the `ook-reader` app as a Dioxus crate, get it building and running as a desktop
app, and confirm the NeoVim build/run/debug loop on the real project.

## Steps

- [ ] `dx new ook-reader` (desktop template) — or scaffold in place
- [ ] Set up `Cargo.toml`: `dioxus = { version = "0.7", features = ["desktop"] }`,
      add `dioxus-router` for navigation
- [ ] `Dioxus.toml`: app name, default platform `desktop`
- [ ] Minimal app: `src/main.rs` with `dioxus::launch`, a `Route` enum
      (`Library`, `Reader`), and a shell layout
- [ ] `.gitignore` already covers `/target`, `/dist`
- [ ] `dx serve --platform desktop` → a window opens; edit `src/` → hot reload
- [ ] Confirm in NeoVim: rust-analyzer attached, `:RustLsp debuggables` hits a breakpoint

## Notes

- **Source of truth is `Cargo.toml`** — no generated project file, no generate step.
- **Golden rule:** edit `src/` → save → `dx serve` hot-reloads. Changing `Cargo.toml`
  triggers a rebuild automatically.
- Keep the structure ready for mobile/web (Milestone 4): one crate, platform chosen by
  Cargo feature + `dx --platform`.

## Open / to verify

- [ ] Completion quality **inside `rsx!`** in practice (proc-macro limitation —
      [`RESEARCH.md`](../../../../RESEARCH.md) §2).
</content>
