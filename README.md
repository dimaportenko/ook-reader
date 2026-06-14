# Ook Reader

A cross-platform EPUB reader written in **Rust** with the **[Dioxus 0.7](https://dioxuslabs.com/learn/0.7/)**
UI framework, developed in **NeoVim**. One codebase targets **desktop**
(macOS/Windows/Linux) first, with **mobile** (iOS/Android) and **web** (WASM) to follow.

> **Pivoted from Swift.** This project started as a Swift/Readium app and was restarted
> on Dioxus/Rust for a true single-codebase cross-platform path. See
> [`RESEARCH.md`](RESEARCH.md) for the full rationale and tech evaluation.

**Planning docs:** [`docs/roadmap.md`](docs/roadmap.md) — roadmap → milestones →
features → phases. **Research basis:** [`RESEARCH.md`](RESEARCH.md). **Dev setup:**
[`docs/guides/neovim-rust-dioxus-project-setup.md`](docs/guides/neovim-rust-dioxus-project-setup.md).

## Why this stack

EPUB content is XHTML + CSS. Dioxus desktop/mobile renders through the system **webview**
(WebKit / WebView2), so it understands book content natively — no custom layout engine.
And unlike Swift, the same Rust code runs on desktop, mobile, and the web. See
[`RESEARCH.md`](RESEARCH.md) §1.

## Requirements

| Tool | Why | Install |
|---|---|---|
| [rustup](https://rustup.rs) (stable Rust) | compiler, cargo, clippy, rustfmt | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) (`dx`) | build/serve/bundle Dioxus apps | `cargo binstall dioxus-cli --force` |
| `rust-analyzer`, `rust-src` | LSP + std sources | `rustup component add rust-analyzer rust-src` |
| `wasm32-unknown-unknown` target | web build (optional until Milestone 4) | `rustup target add wasm32-unknown-unknown` |
| NeoVim + LazyVim `lang.rust` | editing, LSP, debug | see [Editor setup](#editor-setup-neovim) |

macOS desktop needs **no extra webview dependencies** (system WebKit).

## Project layout

> The Cargo crate is not scaffolded yet — that's **Phase 2** (`dx new`). This is the
> intended shape; `Cargo.toml` is the source of truth (no project generator).

```
ook-reader/
├── Cargo.toml                 # crate manifest + dependencies (source of truth)
├── Dioxus.toml                # Dioxus app config (name, platforms)
├── src/
│   ├── main.rs                #   dioxus::launch entry point
│   ├── components/            #   UI components (library, reader, …)
│   └── epub/                  #   rbook parsing + resource serving
├── assets/                    # bundled CSS / icons (asset! macro)
├── RESEARCH.md                # tech evaluation + phased plan
├── README.md                  # this file
└── docs/                      # roadmap → milestones → features → phases
```

Build output (`/target`, `/dist`) is gitignored. There is **no generated project file** —
Cargo is the project model.

## Getting started

Once the crate exists (Phase 2):

```sh
dx serve --platform desktop      # dev server with hot reload
dx build --release               # production build
dx bundle --platform desktop     # package an installable artifact
```

Before then, the dev-environment setup (Phase 0) is in the
[NeoVim Rust/Dioxus guide](docs/guides/neovim-rust-dioxus-project-setup.md).

## Daily workflow

> **Golden rule:** edit files under `src/` → just save, `dx serve` hot-reloads.
> Changing `Cargo.toml` triggers a rebuild automatically. **No generate step** (the big
> win over the old Tuist/Xcode loop).

| Command | Action |
|---|---|
| `dx serve` | dev server + hot reload (default platform) |
| `dx serve --platform desktop\|web\|mobile` | serve a specific platform |
| `dx serve --hotpatch` | experimental Rust **logic** hot-patching |
| `dx build --release` / `dx bundle` | production build / package |
| `dx fmt` / `dx check` | format `rsx!` / type-check |
| `cargo clippy` / `cargo test` | lint / test |

## Editor setup (NeoVim)

This repo assumes LazyVim with the **`lang.rust`** extra (`:LazyExtras` → enable
`lang.rust`), which pulls in **rustaceanvim** (rust-analyzer), `crates.nvim`, the Rust
treesitter parser, and Mason `codelldb` for debugging. Don't also call
`lspconfig.rust_analyzer.setup` — rustaceanvim owns the LSP.

| Action | How |
|---|---|
| Run / test from editor | `:RustLsp runnables` / `:RustLsp testables` |
| Debug | `:RustLsp debuggables` (auto-wires codelldb) |
| Expand a macro / explain an error | `:RustLsp expandMacro` / `:RustLsp explainError` |
| Serve the app | terminal split running `dx serve` (hot reload) |

Full machine + per-project runbook, debugging, and gotchas:
[`docs/guides/neovim-rust-dioxus-project-setup.md`](docs/guides/neovim-rust-dioxus-project-setup.md).

## Notes & gotchas

- **Weak completion inside `rsx! { }`** — a rust-analyzer proc-macro limitation; Dioxus
  mitigates it but expect it weaker than plain Rust. Outside `rsx!`, completion is full.
- **`rsx!` isn't formatted by `cargo fmt`** — use **`dx fmt`**.
- **DRM-free EPUBs only** — Readium LCP / Adobe DRM need EDRLab licensing and have no
  Rust support (see [`RESEARCH.md`](RESEARCH.md) §3.4).
- **Don't target Dioxus Native / Blitz for book content** — its CSS engine is only
  partial; the webview renderer is the right target for EPUB XHTML/CSS.
</content>
