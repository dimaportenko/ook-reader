# Ook Reader

A cross-platform e-book (EPUB) reader. **Rust** as the programming language,
**[Dioxus 0.7](https://dioxuslabs.com/learn/0.7/)** as the UI framework, **NeoVim**
(LazyVim) as the IDE. Target order: **desktop first** (macOS/Windows/Linux), then
mobile (iOS/Android) and web (WASM) — one Rust codebase across all of them.

> **History:** this project began in Swift (Readium Swift Toolkit, Apple-only) and was
> restarted on Dioxus/Rust for a real single-codebase cross-platform path. The Swift
> rationale and pivot are recorded in [`RESEARCH.md`](RESEARCH.md).

## Short-term goals

- Stand up a Rust + Dioxus dev environment in NeoVim and confirm the build/run/debug loop.
- Learn enough Rust + Dioxus (ownership, traits, `async`, components, signals, `rsx!`).
- Build a basic EPUB reader (open a book, render it, turn pages, remember position) and
  then add the features missing from other reader apps.

## First steps

- Set up the Rust/Dioxus NeoVim toolchain — see
  [`docs/guides/neovim-rust-dioxus-project-setup.md`](docs/guides/neovim-rust-dioxus-project-setup.md):
  - LazyVim base (https://github.com/lazyvim/lazyvim) + the `lang.rust` extra;
  - Dioxus docs — https://dioxuslabs.com/learn/0.7/ ;
  - rustaceanvim — https://github.com/mrcjkb/rustaceanvim .
- Scaffold the app with `dx new` (desktop), then render a real `.epub` with `rbook` in a
  webview iframe.

Why this matters / how to read the plan: [`docs/roadmap.md`](docs/roadmap.md).
</content>
