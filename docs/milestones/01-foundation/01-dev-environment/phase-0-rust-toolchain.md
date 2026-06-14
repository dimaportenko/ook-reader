# Phase 0 — Rust + NeoVim Toolchain

[← Feature: Dev Environment](README.md) · **Status:** ⬜ planned

## Goal

Configure NeoVim (on top of LazyVim) to edit, build, run, and debug Rust / Dioxus code,
and confirm the loop works before writing any real app code.

## The stack to install

| Piece | Role | Where |
|---|---|---|
| rustup (stable) | rustc, cargo, clippy, rustfmt | `curl … sh.rustup.rs \| sh` |
| `rust-analyzer`, `rust-src` | LSP + std sources | `rustup component add rust-analyzer rust-src` |
| `wasm32-unknown-unknown` | web build target (optional now) | `rustup target add wasm32-unknown-unknown` |
| Dioxus CLI (`dx`) | build/serve/bundle Dioxus apps | `cargo binstall dioxus-cli --force` |
| LazyVim `lang.rust` extra | rustaceanvim, crates.nvim, treesitter, Mason codelldb | `:LazyExtras` → enable `lang.rust` |

macOS desktop webview needs **no extra deps** (system WebKit).

## Steps

- [ ] Install rustup; confirm `rustc`, `cargo`, `clippy`, `rustfmt`
- [ ] `rustup component add rust-analyzer rust-src`
- [ ] `cargo binstall dioxus-cli --force`; confirm `dx --version`
- [ ] Enable the LazyVim `lang.rust` extra (don't also call `lspconfig.rust_analyzer.setup`)
- [ ] **Validate with a throwaway app:** `dx new hello` (desktop) →
      `dx serve --platform desktop` runs a window → set a breakpoint →
      `:RustLsp debuggables` stops at it
- [ ] Delete the throwaway; don't start the real project until this round-trip works

## Reference

Full runbook (machine + per-project + debugging + gotchas):
[`guides/neovim-rust-dioxus-project-setup.md`](../../../guides/neovim-rust-dioxus-project-setup.md).

## Still simpler than the old Swift setup

No `xcode-build-server` bridge, no Tuist generate step, no per-project
`:XcodebuildSetup` — rust-analyzer reads `Cargo.toml` natively. See
[`RESEARCH.md`](../../../../RESEARCH.md) §5.
</content>
