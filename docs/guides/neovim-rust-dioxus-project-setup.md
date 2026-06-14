# NeoVim Rust / Dioxus Project Setup — Reusable Runbook

A guide for setting up **Rust development in NeoVim** (LazyVim) for building a
**Dioxus 0.7 cross-platform app** (web / desktop / mobile from one codebase) on macOS.
Copy it into any new Dioxus/Rust project.

**Audience:** macOS + Apple Silicon, LazyVim, NeoVim 0.10+. Validated against Dioxus
0.7.x, rustaceanvim, and the LazyVim `lang.rust` extra (June 2026).

> **Why this is simpler than the Swift/Xcode-in-NeoVim setup:** Rust in NeoVim needs
> **no `xcode-build-server` bridge** (rust-analyzer reads `Cargo.toml`/`cargo metadata`
> natively), **no project generator** like Tuist (Cargo *is* the project model — no
> generated `.xcodeproj` to keep in sync), and **no per-project `:XcodebuildSetup`
> scheme/destination dance**. rust-analyzer auto-discovers the workspace from
> `Cargo.toml`; `dx serve` reads `Dioxus.toml`. There are no machine-specific generated
> artifacts to gitignore beyond `target/`. See `§E` for the side-by-side comparison.

Sections: `§A` machine setup (once per machine — Rust toolchain, targets, `dx`),
`§B` NeoVim/LazyVim Rust config, `§C` debugging (DAP), `§D` per-project setup & daily
loop, `§E` Rust-vs-Swift comparison, `§F` gotchas.

---

## A. Machine setup (once per machine)

### A1. Install the Rust toolchain (rustup)

One-line install — identical on [rustup.rs](https://rustup.rs) and
[rust-lang.org/tools/install](https://www.rust-lang.org/tools/install):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This installs **rustup** (toolchain manager), **rustc** (compiler) and **cargo**
(build tool / package manager) into `~/.cargo/bin`, and defaults to the **stable**
channel.

> rustup's **default profile** already includes `rustfmt` and `clippy` (the minimal
> profile is just `rustc` + `rust-std` + `cargo`).
> Source: [rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html).

### A2. Components: rust-analyzer, clippy, rustfmt, rust-src

```sh
rustup component add rust-analyzer   # the LSP server (also a rustup component)
rustup component add rust-src        # std sources — rust-analyzer wants these
rustup component add clippy rustfmt  # usually already present (default profile)
```

`rust-analyzer` is available as a rustup component
([rust-analyzer binary docs](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)).

> **Which rust-analyzer does the editor use?** With the LazyVim `lang.rust` extra,
> **rustaceanvim** drives the LSP. rustaceanvim looks for a `rust-analyzer` on `PATH`
> (the rustup component above) and, failing that, falls back to a Mason-installed one.
> Pick one source and keep it current to avoid version skew with your compiler — the
> rustup component stays matched to your toolchain. (rust-analyzer's own manual gives no
> hard preference for non-VS-Code editors:
> [manual](https://rust-analyzer.github.io/manual.html).)

### A3. Targets for Dioxus platforms

```sh
# Web (Dioxus compiles the app to WASM):
rustup target add wasm32-unknown-unknown

# iOS (optional — needs Xcode + iOS SDK installed):
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# Android (optional — needs Android Studio: SDK, NDK side-by-side, CMake):
rustup target add aarch64-linux-android armv7-linux-androideabi \
  i686-linux-android x86_64-linux-android
```

- `wasm32-unknown-unknown` is the target Dioxus web builds use
  ([getting started](https://dioxuslabs.com/learn/0.7/getting_started/)).
- Mobile additionally needs **Xcode** (iOS) or **Android Studio + SDK/NDK/CMake** with
  `JAVA_HOME` / `ANDROID_HOME` / `NDK_HOME` set
  ([mobile guide](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/)).
- **`dx` does not auto-add rustup targets** — run the `rustup target add` lines yourself.

### A4. Install the Dioxus CLI (`dx`)

The crate is `dioxus-cli`; the binary it ships is **`dx`**. Prefer the **prebuilt**
binary (building from source can take ~10 min):

```sh
# Fast — prebuilt binary via cargo-binstall (recommended):
cargo install cargo-binstall          # skip if already installed
cargo binstall dioxus-cli --force

# …or the official install script:
curl -sSL https://dioxus.dev/install.sh | bash

# …or from source (slow):
cargo install dioxus-cli --locked
```

Verify: `dx --version`. Source:
[getting started](https://dioxuslabs.com/learn/0.7/getting_started/).

### A5. Desktop webview system deps on macOS

**Nothing extra.** Dioxus desktop renders through **wry/tao**, which use the platform's
native webview — **WebKit** on macOS, which is part of the OS. The docs state plainly:
*"There are no extra dependencies for macOS!"*
([getting started](https://dioxuslabs.com/learn/0.7/getting_started/)).

For contrast (not needed on your machine): Windows needs **WebView2**; Linux needs
**WebKitGTK** + tooling (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libssl-dev`, …).

---

## B. NeoVim / LazyVim Rust setup

### C1. Enable the LazyVim `lang.rust` extra

The fastest path. Either run the UI picker:

```vim
:LazyExtras          " navigate to lang.rust, press x to enable, restart
```

…or add the entry to `lazyvim.json` (`extras` array) by hand — exact string:

```json
{
  "extras": [
    "lazyvim.plugins.extras.lang.rust"
  ]
}
```

Source: [LazyVim lang.rust extra](https://www.lazyvim.org/extras/lang/rust).

**What the extra pulls in** (verified against the
[rust.lua source](https://raw.githubusercontent.com/LazyVim/LazyVim/main/lua/lazyvim/plugins/extras/lang/rust.lua)):

| Plugin | Role in the extra |
|---|---|
| **rustaceanvim** | Owns rust-analyzer. Configured via `vim.g.rustaceanvim`. |
| **nvim-lspconfig** | Present, but `rust_analyzer = { enabled = false }` — lspconfig is told **not** to start rust-analyzer, so rustaceanvim has sole ownership (avoids the double-setup conflict rustaceanvim warns about). |
| **nvim-treesitter** | Adds parsers: `ensure_installed = { "rust", "ron" }`. |
| **crates.nvim** | Cargo.toml completion / version management. |
| **mason.nvim** | Adds **`codelldb`** to `ensure_installed` (for DAP). |
| **nvim-neotest** | Registers the `rustaceanvim.neotest` adapter. |

> The extra also offers an optional **`bacon-ls`** diagnostics provider (installs
> `bacon`) as an alternative to rust-analyzer's on-save check. Leave it off unless you
> want background `cargo check` diagnostics.

### C2. rustaceanvim — use it, don't double-configure

[rustaceanvim](https://github.com/mrcjkb/rustaceanvim) is the successor to
`rust-tools.nvim`. Key rules:

- **Do NOT call `lspconfig.rust_analyzer.setup{}` yourself** — rustaceanvim manages the
  rust-analyzer client. Manual lspconfig setup causes conflicts. (The LazyVim extra
  already disables lspconfig's rust_analyzer for you.)
- Configure it through the global table **`vim.g.rustaceanvim`**, set *before* the
  plugin loads. Structure:
  ```lua
  vim.g.rustaceanvim = {
    tools = {},
    server = {
      on_attach = function(client, bufnr) end,
      default_settings = { ["rust-analyzer"] = { } },
    },
    dap = {},
  }
  ```
- Commands are under **`:RustLsp`** — e.g. `:RustLsp runnables`, `:RustLsp testables`,
  `:RustLsp debuggables`, `:RustLsp expandMacro`, `:RustLsp explainError`,
  `:RustLsp codeAction`, `:RustLsp hover actions`.

### C3. crates.nvim — Cargo.toml

[crates.nvim](https://github.com/saecki/crates.nvim) manages crates.io dependencies in
`Cargo.toml`: completion of crate names / versions / features (via an in-process LSP),
inline "latest version" hints, flags yanked/pre-release/incompatible versions, and code
actions to **update** (newest compatible) or **upgrade** (newest) a crate. Popups for
crate info, versions, and the feature hierarchy; jump to docs.rs / repo / homepage.

### C4. Optional `lua/plugins/rust.lua` customization

The extra is enough for most work. Add this only if you want to tune rust-analyzer or
keymaps. Place at `~/.config/nvim/lua/plugins/rust.lua` (or project-local equivalent):

```lua
-- Extra Rust / Dioxus tuning. The `lang.rust` extra must be enabled.
return {
  -- rustaceanvim settings via vim.g (the extra deep-merges this).
  {
    "mrcjkb/rustaceanvim",
    opts = {
      server = {
        default_settings = {
          ["rust-analyzer"] = {
            -- rsx!/proc-macro support — keep proc-macros and build scripts on.
            procMacro = { enable = true },
            cargo = { buildScripts = { enable = true } },
            -- Use clippy for on-save diagnostics instead of plain `cargo check`.
            check = { command = "clippy" },
            -- Big dependency trees: cache prime + don't index all targets.
            cachePriming = { enable = true },
          },
        },
      },
    },
  },

  -- Treesitter (already added by the extra; harmless if duplicated).
  {
    "nvim-treesitter/nvim-treesitter",
    opts = { ensure_installed = { "rust", "ron", "toml" } },
  },
}
```

> **WASM-target caveat (web-only crates):** if your crate *only* compiles for
> `wasm32-unknown-unknown` (e.g. uses `web-sys`/browser-only APIs), rust-analyzer's
> default host-target check will surface false errors. Pin the analysis target either in
> `vim.g.rustaceanvim` (`["rust-analyzer"] = { cargo = { target =
> "wasm32-unknown-unknown" } }`) or, better, project-wide in `.cargo/config.toml`
> (`[build] target = "wasm32-unknown-unknown"`), which switches both cargo and
> rust-analyzer. A typical Dioxus app that *also* builds desktop usually does **not**
> need this — leave the host target. Sources:
> [RA configuration](https://rust-analyzer.github.io/book/configuration.html),
> [RA #3592](https://github.com/rust-lang/rust-analyzer/issues/3592).

---

## C. Debugging (DAP)

The LazyVim `lang.rust` extra installs **`codelldb`** via Mason and `dap`/`dap-ui` come
from LazyVim's `dap.core` extra (enable it via `:LazyExtras` if not already on).

**rustaceanvim auto-wires DAP** — if `codelldb` is available (Mason install or on
`PATH`) it self-configures the adapter; you don't need a manual `dap.adapters.codelldb`
table. Just run:

```vim
:RustLsp debuggables      " pick a debuggable target → builds it → starts codelldb
```

Then drive it with LazyVim's standard dap keymaps (`<leader>db` breakpoint,
`<leader>dc` continue, `<leader>di` step into, etc.).

> **Apple Silicon note:** `codelldb` ships native arm64 macOS builds (via Mason), and
> rustaceanvim picks the OS-correct `liblldb` (`.dylib` on macOS) automatically — no
> manual liblldb path. This is simpler than the plain-`nvim-dap` route, which can't
> resolve VS Code's `${cargo:program}` placeholder and needs a custom `enrich_config`
> to find the built binary. Source:
> [nvim-dap #671](https://github.com/mfussenegger/nvim-dap/discussions/671),
> [rustaceanvim](https://github.com/mrcjkb/rustaceanvim).

Manual fallback (only if you skip rustaceanvim's integration):

```lua
local dap = require("dap")
dap.adapters.codelldb = {
  type = "server",
  port = "${port}",
  executable = {
    command = vim.fn.stdpath("data")
      .. "/mason/packages/codelldb/extension/adapter/codelldb",
    args = { "--port", "${port}" },
  },
}
```

> Debugging applies to **desktop / native** builds. The **web/WASM** build is not
> debugged through codelldb — use browser devtools for the web target.

---

## D. Per-project setup & daily workflow

### E1. Create a project

```sh
dx new my-app        # interactive template picker (web / desktop / fullstack / …)
cd my-app
```

`dx new` scaffolds `Cargo.toml`, `Dioxus.toml`, and `src/main.rs`. There is **no
generator step to re-run** and no generated project file to gitignore — Cargo is the
project model. `.gitignore` only needs:

```gitignore
/target
/dist
```

Open the folder in NeoVim — rust-analyzer attaches automatically from `Cargo.toml`.

### E2. Run / build (terminal and editor)

| Command | Action |
|---|---|
| `dx serve` | Dev server with **hot reload** (default platform) |
| `dx serve --platform web` | Serve the web build |
| `dx serve --platform desktop` | Serve the desktop (webview) build |
| `dx serve --platform mobile` | Serve mobile (after sim/emulator is up) |
| `dx serve --hotpatch` | Experimental Rust hot-patching |
| `dx build --release` | Production build |
| `dx bundle --platform desktop` | Bundle a distributable (also `web`/`ios`/`android`) |
| `dx fmt` | Format `rsx!` blocks |
| `dx check` | Type-check without building |
| `cargo build` / `cargo test` | Standard Cargo |
| `cargo clippy` / `cargo fmt` | Lint / format Rust source |

Source for `dx` commands:
[Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) and the CLI reference.

> `--platform` may also be expressed with shorthand flags in 0.7 (e.g. `dx serve
> --web` / `--desktop` / `--webview`). Both forms appear in the docs.

### E3. NeoVim integration for running

There is no dedicated "dx.nvim". Practical options:

- **Terminal split** running `dx serve` (LazyVim ships a terminal toggle —
  `<C-/>`); hot reload means you rarely restart it.
- **`:RustLsp runnables` / `:RustLsp testables`** for cargo-level run/test from the
  editor (these are rustaceanvim, not Dioxus-aware).
- An overseer.nvim / `vim.system` task wrapping `dx serve` if you want it bound to a
  key — optional, not required.

> **Golden rule:** editing files under `src/` needs no regeneration — just save and let
> `dx serve` hot-reload. Changing `Cargo.toml` triggers a rebuild automatically. This is
> the big DX win over the Tuist loop (`edit Project.swift → tuist generate`): **there is
> no generate step**.

---

## E. Rust-in-NeoVim vs Swift/Xcode-in-NeoVim (high level)

| Concern | Swift / Xcode (prior guide) | Rust / Dioxus (this guide) |
|---|---|---|
| Project model | Tuist `Project.swift` → generated `.xcodeproj` | `Cargo.toml` (no generation) |
| LSP wiring | `xcode-build-server config` bridge per project | none — rust-analyzer reads cargo natively |
| Editor LSP | sourcekit-lsp (Xcode shim) via lspconfig | rust-analyzer via rustaceanvim |
| First-run ceremony | `tuist generate` + `:XcodebuildSetup` (scheme/device) | `dx new`, then just open + `dx serve` |
| Debugger | `lldb-dap` (bundled in Xcode 16+) via xcodebuild.nvim | `codelldb` (Mason) auto-wired by rustaceanvim |
| Regenerate on structural change | yes (every manifest edit) | no |
| Gitignored generated artifacts | `.xcodeproj`, `.xcworkspace`, `buildServer.json`, … | just `/target`, `/dist` |
| Run / hot reload | build & run to simulator | `dx serve` hot reload |

Net: the Rust path removes two whole bridges (the build-server and the project
generator) and the per-project scheme/destination setup. rust-analyzer + Cargo are
self-describing in a way the Xcode toolchain is not.

---

## F. Gotchas & corrections

| Symptom | Cause / fix |
|---|---|
| Double LSP / "rust-analyzer already attached" weirdness | Don't call `lspconfig.rust_analyzer.setup{}`. rustaceanvim owns it; the LazyVim extra already sets `rust_analyzer = { enabled = false }` in lspconfig. ([rustaceanvim](https://github.com/mrcjkb/rustaceanvim)) |
| Poor / no completion **inside** `rsx!{ }` | Proc-macro completion is a known rust-analyzer limitation. Dioxus mitigates it (the `rsx!` parser emits hidden completion-hint modules), so element/attribute names do complete, but expect it to be weaker than plain Rust. Keep `procMacro.enable = true`. Outside `rsx!`, completion is full. ([discussion #1895](https://github.com/DioxusLabs/dioxus/discussions/1895), [RA #10894](https://github.com/rust-lang/rust-analyzer/issues/10894)) |
| `rsx!` not formatting with `cargo fmt` | `cargo fmt` doesn't touch macro bodies. Use **`dx fmt`** for `rsx!` blocks. |
| Slow first index / high CPU on large dep trees | rust-analyzer indexes the whole dependency graph on first open; it's a one-time cost per workspace (cached). Let it finish; `cachePriming.enable = true` helps. Avoid `checkOnSave.allTargets` for no-std/wasm crates (false errors). ([RA configuration](https://rust-analyzer.github.io/book/configuration.html)) |
| False errors in a **web-only** crate (`web-sys`, browser APIs) | rust-analyzer is checking the host target. Set `cargo.target = "wasm32-unknown-unknown"` (rustaceanvim) or `[build] target` in `.cargo/config.toml`. Skip this if the crate also builds desktop. ([RA #3592](https://github.com/rust-lang/rust-analyzer/issues/3592)) |
| `dx` build can't find webview / fails on macOS | Shouldn't happen — macOS needs no extra deps (system WebKit). Re-check `xcode-select -p` is set if Apple toolchain CLI is missing. ([getting started](https://dioxuslabs.com/learn/0.7/getting_started/)) |
| `dx new` template not building for web | Ensure `wasm32-unknown-unknown` target is added (`§A3`) — `dx` does not add it for you. |
| Want codelldb but it isn't found | It's installed by the `lang.rust` extra via Mason (`:Mason` to verify `codelldb`); ensure the `dap.core` extra is also enabled for the dap UI. |

## References

- Rust install — [rustup.rs](https://rustup.rs) · [rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) · [rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html)
- rust-analyzer — [binary docs](https://rust-analyzer.github.io/book/rust_analyzer_binary.html) · [manual](https://rust-analyzer.github.io/manual.html) · [configuration](https://rust-analyzer.github.io/book/configuration.html)
- Dioxus 0.7 — [getting started](https://dioxuslabs.com/learn/0.7/getting_started/) · [mobile guide](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/)
- LazyVim — [lang.rust extra](https://www.lazyvim.org/extras/lang/rust) · [rust.lua source](https://raw.githubusercontent.com/LazyVim/LazyVim/main/lua/lazyvim/plugins/extras/lang/rust.lua)
- Plugins — [rustaceanvim](https://github.com/mrcjkb/rustaceanvim) · [crates.nvim](https://github.com/saecki/crates.nvim) · [nvim-dap codelldb #671](https://github.com/mfussenegger/nvim-dap/discussions/671)
- rsx!/proc-macro — [Dioxus discussion #1895](https://github.com/DioxusLabs/dioxus/discussions/1895) · [RA proc-macro #10894](https://github.com/rust-lang/rust-analyzer/issues/10894)
</content>
</invoke>
