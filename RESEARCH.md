# Ook Reader — Research & Plan (Dioxus / Rust)

> Deep research on building a cross-platform EPUB reader in **Rust** with the
> **Dioxus 0.7** UI framework, developed in **NeoVim** (LazyVim).
> Date: 2026-06-14. Supersedes the prior Swift/Readium research (project pivoted
> away from Swift — Dioxus gives one Rust codebase for desktop, mobile, and web).
> All version/library claims verified against live sources on 2026-06-14.

## TL;DR

- **Dioxus 0.7 is the stack.** One Rust codebase targets **web (WASM)**, **desktop**
  (macOS/Windows/Linux), and **mobile** (iOS/Android), switched by Cargo feature +
  `dx --platform`. Stable since v0.7.0 (2025-10-31); 0.7.9 latest at research time.
- **The webview is the headline fit.** Desktop/mobile render through the system
  **webview** (wry/tao → WebKit/WebView2). EPUB content *is* XHTML + CSS, so the
  renderer natively understands book content — no custom layout engine needed.
- **Parse EPUBs with [`rbook`](https://crates.io/crates/rbook)** (Apache-2.0, actively
  maintained) over the more-downloaded `epub` crate (GPL-3.0, viral license).
- **Render each chapter in a sandboxed `<iframe>`** (the epub.js / Readium approach),
  serving EPUB-internal resources via Dioxus's **`use_asset_handler`** custom protocol
  (desktop) or blob URLs (web).
- **Persist with `rusqlite` (bundled SQLite)**; locate the DB via the `directories`
  crate. Abstract persistence behind a trait if web/WASM joins the roadmap.
- **Target order: desktop first** (decided), then mobile + web. Desktop is the easiest
  Dioxus path and gives full filesystem access for the library.
- **Dev in NeoVim is much simpler than the old Swift toolchain** — `rust-analyzer`
  reads Cargo natively (no `xcode-build-server` bridge, no Tuist generate step).
- **LCP / Adobe DRM are out of scope** — they require EDRLab licensing and have no
  credible Rust support. We read DRM-free EPUBs only.

---

## 1. Why Dioxus (and why not Swift)

The original plan was a Swift app (Readium Swift Toolkit) built in NeoVim. That works,
but: (a) the NeoVim/Swift toolchain needs an `xcode-build-server` bridge + Tuist project
generation + per-project `:XcodebuildSetup`; (b) Readium's navigator is UIKit-only and
its package targets iOS only, so macOS needed Mac Catalyst and a separate rendering
path; (c) Swift effectively locks you to Apple platforms.

**Dioxus** removes all three frictions. It is a cross-platform Rust UI framework —
"one codebase ... runs on web, desktop, and mobile"
([learn/0.7](https://dioxuslabs.com/learn/0.7/)). Components are Rust functions that
return an `Element` via an HTML/CSS-like `rsx!` macro.

| Renderer | How it draws | Notes |
|---|---|---|
| **Web** (`dioxus-web`) | Compiles to **WASM**, renders to the browser DOM | distribution = a URL |
| **Desktop** (`dioxus-desktop`) | System **webview** via **wry** + **tao** (windowing) | your Rust runs natively, not in JS |
| **Mobile** (`dioxus-mobile`) | Same webview model, iOS/Android | needs Xcode / Android Studio |
| **Fullstack** (`dioxus-fullstack`) | SSR + server functions on **Axum** | not needed for a local reader |
| **Native / Blitz** | GPU renderer (Stylo/Taffy/Vello), **no webview** | new in 0.7, **partial CSS** — *not* for EPUB |

**Critical point for an EPUB reader:** the default desktop/mobile renderer is the OS
webview, which natively renders XHTML + CSS — exactly EPUB's content model. The cost is
cross-engine differences (WebKit on macOS/iOS, WebView2 on Windows, WebKitGTK on Linux)
— the same caveat every web-based reader lives with. **Do not** target Dioxus Native /
Blitz for book content; its CSS engine is only partial.

Sources: [learn/0.7](https://dioxuslabs.com/learn/0.7/) ·
[FAQ — "Is it Electron?"](https://github.com/dioxuslabs/dioxus/blob/v0.7.2/notes/FAQ.md) ·
[desktop README (wry)](https://github.com/dioxuslabs/dioxus/blob/v0.7.2/packages/desktop/README.md) ·
[0.7 release notes](https://dioxuslabs.com/blog/release-070/).

## 2. Dioxus 0.7 — tooling & programming model

### Tooling: the `dx` CLI

Crate `dioxus-cli`, binary `dx`. Prefer the prebuilt binary (source build is slow):

```sh
cargo binstall dioxus-cli --force      # prebuilt (recommended)
# or: curl -sSL https://dioxus.dev/install.sh | bash
# or: cargo install dioxus-cli --locked  (slow)
```

| Command | Purpose |
|---|---|
| `dx new my-app` | scaffold (`Cargo.toml`, `Dioxus.toml`, `src/main.rs`, `assets/`) |
| `dx serve [--platform desktop\|web\|mobile]` | dev server with **hot reload** |
| `dx serve --hotpatch` | experimental Rust **logic** hot-patching (Subsecond) |
| `dx build --release` | production build |
| `dx bundle --platform desktop` | package an installable artifact |
| `dx fmt` / `dx check` | format `rsx!` / type-check without building |

Sources: [getting started](https://dioxuslabs.com/learn/0.7/getting_started/) ·
[CLI README](https://github.com/dioxuslabs/dioxus/blob/v0.7.2/packages/cli/README.md).

### Programming model

A component is a function returning `Element`, annotated `#[component]`; state lives in
**signals**.

```rust
use dioxus::prelude::*;

fn main() { dioxus::launch(App); }

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);          // reactive state
    rsx! {
        div { "Count: {count}" }
        button { onclick: move |_| count += 1, "Increment" }
    }
}
```

Reactivity primitives (0.7):

- **`use_signal(|| init)`** — fine-grained reactive value; reading subscribes the
  component, writing wakes only subscribers.
- **`Store`** *(new in 0.7)* — reactive primitive for **nested** state (structs,
  collections); only changed sub-fields mark dirty. Use it for the reader's larger state
  tree (library, per-book progress, settings).
- **`use_resource(|| async { … })`** — async derived state; re-runs when read signals
  change. Ideal for async EPUB parsing / file IO.
- **`use_effect`** — side effects on dependency change.
- **Context** — `use_context_provider` / `use_context::<T>()` for app-wide state
  (theme, reader settings).

Routing via `dioxus-router` and `#[derive(Routable)]`:

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(Shell)]
        #[route("/")]            Library {},
        #[route("/book/:id")]    Reader { id: String },
}
```

**Assets / styling:** the `asset!` macro (Manganis) declares static assets at compile
time (hashed, hot-reloadable); inject CSS with `document::Stylesheet { href: CSS }`.
First-party Tailwind support in the CLI.

Sources: [dioxus README](https://github.com/dioxuslabs/dioxus/blob/v0.7.2/packages/dioxus/README.md) ·
[reactivity](https://github.com/dioxuslabs/dioxus/blob/v0.7.2/packages/core/docs/reactivity.md) ·
[Stores (release notes)](https://dioxuslabs.com/blog/release-070/) ·
[assets tutorial](https://dioxuslabs.com/learn/0.7/tutorial/assets/).

> **One documented gap:** Dioxus 0.7's MSRV is not authoritatively published. Use a
> recent **stable** Rust and verify resolution when pinning.

## 3. The EPUB layer

### 3.1 Parsing — use `rbook`

| Crate | Latest | License | Reads | Notes |
|---|---|---|---|---|
| **`rbook`** ✅ | 0.7.7 (2026-05-23) | **Apache-2.0** | EPUB 2/3 | actively maintained, 0 open issues, streaming reader + unified TOC |
| `epub` (epub-rs) | 2.1.5 (2025-10-29) | **GPL-3.0** | EPUB 2/3 | more downloads but viral license, lightly maintained |
| `epub-builder` | 0.8.3 | MPL-2.0 | — | **write-only**, not a reader |
| `epub-parser`, `iepub` | new / MOBI-focused | MIT | yes | niche |

**Decision: `rbook`.** Two decisive reasons over `epub`: (1) **Apache-2.0** vs
**GPL-3.0** (GPL is viral and problematic for app distribution); (2) maintenance — a
May 2026 release with zero open issues. It also has better reader ergonomics.

```rust
use rbook::{Ebook, Epub};
let epub = Epub::open("book.epub")?;
println!("{}", epub.metadata().title().unwrap().value());
let mut reader = epub.reader();
while let Some(Ok(content)) = reader.read_next() {
    let xhtml: &str = content.content();   // chapter XHTML in reading order
}
let cover = epub.manifest().cover_image().unwrap().read_bytes()?;
```

Sources: [rbook crate](https://crates.io/crates/rbook) ·
[docs.rs](https://docs.rs/rbook/latest/rbook/) ·
[epub crate (alternative)](https://crates.io/crates/epub).

### 3.2 Rendering — sandboxed iframe + custom protocol

EPUB chapters are XHTML + CSS, so the webview renders them directly. Two ways to inject
chapter HTML:

- **`dangerous_inner_html`** on an element (sets `innerHTML`). Simple, but **no style
  scoping** — the book's `<style>`/CSS leaks into the app and collides. Only for trusted,
  sanitized fragments.
- **One `<iframe>` per spine item** *(recommended)* — full style/JS isolation, a clean
  URL-resolution context, and a natural pagination unit. This is what epub.js and Readium
  do.

**Serving EPUB-internal resources** (images, fonts, CSS use OPF-relative paths):

- **Desktop/mobile:** register a custom protocol with Dioxus's
  **`use_asset_handler`** (wraps wry's `with_asynchronous_custom_protocol`); read the
  resource bytes out of the EPUB zip and `responder.respond(bytes)`.
  *Gotcha:* the scheme resolves as `wry://path/…` on macOS/iOS/Linux but
  `http://wry.path/…` on Windows/Android — never hardcode `scheme://`.
- **Web/WASM:** no filesystem/custom protocol — extract the zip in WASM and serve each
  resource as a **blob URL**, maintaining an internal-path → blob-URL map and rewriting
  chapter HTML.

Rewrite OPF-relative URLs to the custom-protocol/blob URLs (or inject `<base href>`),
and intercept internal hyperlinks → convert to navigation events. For an iframe, set
`sandbox` (omit `allow-scripts`) to neutralize book JS while still rendering XHTML/CSS.

**Pagination happens in the webview (CSS/JS), not in Rust.** The Readium/epub.js
approach: native CSS **multi-column** (`column-width`/`column-gap`) in a clipped
viewport, advancing pages with `transform: translateX()` / `scrollLeft` (add the gap
into the per-page step). Continuous vertical scroll (`overflow-y:auto`, no columns) is
the simpler first cut. Rust only holds nav state (spine index, page index).

Sources: [escape hatch / dangerous_inner_html](https://dioxuslabs.com/learn/0.7/essentials/ui/escape/) ·
[use_asset_handler](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/fn.use_asset_handler.html) ·
[wry custom protocol](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html) ·
[ReadiumCSS injection & pagination](https://github.com/readium/css/blob/master/docs/CSS03-injection_and_pagination.md).

### 3.3 Reading features — feasibility vs Readium

| Feature | Difficulty | Approach |
|---|---|---|
| Resume position + progress bar | **Easy** | `{spine_index, progression}`; progression = `scrollTop/scrollHeight` via JS eval |
| TOC navigation (nested, NCX + nav.xhtml) | **Easy — full parity** | parser hands you the tree; resolve href → spine index |
| Themes (font, size, spacing, light/dark/sepia) | **Easy — near parity** | inject CSS custom properties on `:root`; optionally vendor [ReadiumCSS](https://github.com/readium/readium-css) (BSD) |
| Full-text search | **Easy** | naive Unicode substring per book, or [`tantivy`](https://crates.io/crates/tantivy) for a large library |
| Precise/shareable highlights | **Hard** | needs a WebView JS bridge that resolves positions in the live DOM |
| EPUB CFI | **Very hard — skip v1** | no mature Rust crate; only matters for cross-reader portability |

**The one structurally hard area** is what Readium's native navigator gives for free:
resolving a stored position back into the **live rendered DOM** (precise highlights,
CFI, jump-to-search-hit). A char offset into Rust-extracted text does **not** map to a
DOM offset (whitespace collapsing, `display:none`). Defer it; when needed, follow
Readium's pattern — store before/highlight/after **text context** and **re-find it in
the DOM via injected JS** at display time, rather than computing CFI.

Sources: [Readium Locator model](https://readium.org/architecture/models/locators/) ·
[ReadiumCSS user prefs](https://readium.org/css/docs/CSS12-user_prefs.html) ·
[tantivy](https://crates.io/crates/tantivy).

### 3.4 DRM — out of scope

Readium **LCP** (now ISO/IEC 23078-2:2024) requires signing EDRLab's agreement,
certification, and issued keys — you cannot legally decrypt production content without
them, and there is **no credible Rust crate** (Readium ships Go/Swift/Kotlin/TS, no
Rust). Adobe ADEPT is proprietary. **We support DRM-free EPUBs only.**

Sources: [EDRLab LCP](https://www.edrlab.org/readium-lcp/) ·
[become a license provider](https://www.edrlab.org/projects/readium-lcp/become-lcp-license-provider/).

## 4. Persistence

**Recommended: `rusqlite` with the `bundled` feature** (statically compiles SQLite in —
the documented way to avoid system-lib issues on iOS/Android). One DB holds the library
table + positions/bookmarks/highlights.

```toml
rusqlite = { version = "0.40", features = ["bundled"] }
directories = "6"   # ProjectDirs::data_dir() → platform-correct app data dir
```

- **MVP shortcut:** `serde_json` index file (no migrations, but rewrites the whole file).
- **Web/WASM caveat:** native SQLite + filesystem do **not** work in the browser — use
  IndexedDB/localStorage there. If web is on the roadmap, **abstract persistence behind
  a trait** with a separate WASM backend.

Sources: [rusqlite](https://crates.io/crates/rusqlite) ·
[directories](https://crates.io/crates/directories).

## 5. Development environment (NeoVim)

Rust in NeoVim is markedly simpler than the old Swift setup. Full runbook:
[`docs/guides/neovim-rust-dioxus-project-setup.md`](docs/guides/neovim-rust-dioxus-project-setup.md).
Summary:

- **Toolchain:** `rustup` (stable; `clippy`/`rustfmt` in the default profile),
  `rustup component add rust-analyzer rust-src`,
  `rustup target add wasm32-unknown-unknown` (web). `cargo binstall dioxus-cli`.
  **macOS desktop webview needs no extra deps** (system WebKit).
- **Editor:** LazyVim **`lang.rust`** extra → **rustaceanvim** owns rust-analyzer
  (don't also call `lspconfig.rust_analyzer.setup`), plus `crates.nvim`, treesitter
  `rust`, and Mason `codelldb`.
- **Debug:** `:RustLsp debuggables` (rustaceanvim auto-wires codelldb; picks the arm64
  `liblldb` automatically on Apple Silicon).
- **Loop:** `dx serve` (hot reload) + `cargo clippy`/`dx fmt`. No generate step, nothing
  to gitignore beyond `/target` and `/dist`.

| Concern | Old: Swift/Xcode | New: Rust/Dioxus |
|---|---|---|
| Project model | Tuist `Project.swift` → generated `.xcodeproj` | `Cargo.toml` (no generation) |
| LSP wiring | `xcode-build-server config` per project | none — rust-analyzer reads cargo |
| First-run | `tuist generate` + `:XcodebuildSetup` | `dx new`, then open + `dx serve` |
| Debugger | `lldb-dap` via xcodebuild.nvim | `codelldb` auto-wired by rustaceanvim |
| Gitignored artifacts | `.xcodeproj`, `buildServer.json`, … | `/target`, `/dist` |

## 6. Recommended architecture (synthesis)

```
parse:    rbook (Apache-2.0)  →  spine, manifest, metadata, TOC, resource bytes
render:   wry webview  →  one sandboxed <iframe> per spine item
serve:    use_asset_handler custom protocol (desktop) / blob URLs (web)
style:    inject CSS custom properties on :root (optionally vendor ReadiumCSS)
paginate: CSS multi-column + translateX in injected JS; Rust holds nav state
search:   tantivy; re-find hits in the DOM by stored snippet
persist:  rusqlite (bundled) in the directories data dir; trait-abstracted for web
state:    signals + Store (0.7) for the library/progress tree; use_resource for IO
scope:    DRM-free EPUBs only (LCP/ADEPT excluded)
```

## 7. Phased plan

Matches [`docs/roadmap.md`](docs/roadmap.md). **Target order: desktop first**, then
mobile + web.

- **Phase 0 — Rust + NeoVim toolchain.** rustup, components, `dx`, LazyVim `lang.rust`;
  validate by building a throwaway `dx new` desktop app and hitting a breakpoint.
- **Phase 1 — Learn Rust + Dioxus** (parallel). Ownership/borrowing, enums/`Result`,
  traits, `async`; Dioxus components, signals, `use_resource`, `rsx!`, router.
- **Phase 2 — Dioxus desktop scaffold.** `dx new` (desktop), Cargo deps, routing shell,
  the NeoVim build/run/debug loop on the real project.
- **Phase 3 — EPUB rendering.** `rbook` → open an `.epub` → render a spine item in an
  iframe with `use_asset_handler`; page turns.
- **Phases 4–5 — Library/import + reading position.** File import, library list with
  covers, persist & restore the last locator (`rusqlite`).
- **Milestone 3 — Reader enhancements.** Themes, TOC nav, annotations, search.
- **Milestone 4 — Multi-platform.** Mobile (iOS/Android) + web (WASM); trait-abstract
  persistence/asset-serving for the browser.

## Open questions (resolve at the relevant phase)

- How well does rust-analyzer handle completion **inside `rsx!`** in practice? (known
  proc-macro limitation; Dioxus mitigates it — confirm during Phase 1/2.)
- iframe-per-chapter vs `dangerous_inner_html`: confirm style isolation and resource
  resolution on macOS WebKit during Phase 3 (`wry://` scheme behaviour).
- Pagination: native CSS multi-column vs continuous scroll for v1 — pick after a spike.
- Web target: does `use_asset_handler` need a full blob-URL fallback, and how much of the
  persistence layer must be trait-abstracted before Milestone 4?
- Reading-position fidelity: is `{spine_index, progression}` enough for v1, deferring
  DOM-precise locators/highlights?

## Key sources

- Dioxus — [learn/0.7](https://dioxuslabs.com/learn/0.7/) ·
  [getting started](https://dioxuslabs.com/learn/0.7/getting_started/) ·
  [0.7 release notes](https://dioxuslabs.com/blog/release-070/) ·
  [assets](https://dioxuslabs.com/learn/0.7/tutorial/assets/)
- EPUB parsing — [rbook](https://crates.io/crates/rbook) · [epub](https://crates.io/crates/epub)
- Rendering — [use_asset_handler](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/fn.use_asset_handler.html) ·
  [wry](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html) ·
  [ReadiumCSS pagination](https://github.com/readium/css/blob/master/docs/CSS03-injection_and_pagination.md)
- Reading model — [Readium Locators](https://readium.org/architecture/models/locators/) ·
  [ReadiumCSS](https://github.com/readium/readium-css) · [tantivy](https://crates.io/crates/tantivy)
- Persistence — [rusqlite](https://crates.io/crates/rusqlite) · [directories](https://crates.io/crates/directories)
- DRM — [EDRLab LCP](https://www.edrlab.org/readium-lcp/)
- Dev env — [rustup](https://rustup.rs) · [rustaceanvim](https://github.com/mrcjkb/rustaceanvim) ·
  [LazyVim lang.rust](https://www.lazyvim.org/extras/lang/rust)
</content>
</invoke>
