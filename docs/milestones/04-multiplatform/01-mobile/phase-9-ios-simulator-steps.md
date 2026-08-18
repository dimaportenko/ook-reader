# Phase 9 — Run on the iOS simulator — build log

[← Phase doc](phase-9-ios-simulator.md)

Per-step check → minimal code → why, appended newest-last. The
[phase doc](phase-9-ios-simulator.md)'s "Planned steps" checklist is the high-level index;
this file is the detail and the build log.

## The crux

**The UI ports for free; the app's assumptions about its host do not.**

`dioxus`'s `lib.rs` contains exactly this pair:

```rust
#[cfg(feature = "desktop")]
pub use dioxus_desktop as desktop;

#[cfg(feature = "mobile")]
pub use dioxus_desktop as mobile;
```

One crate, two names, each behind its own feature. `dx build --platform ios` invokes rustc
with `--cfg 'feature="mobile"'` and no `desktop` — so on iOS the *code* is all still there
and only the *path to it* is gone. Nothing about the renderer changes; `wry` wraps
`WKWebView` on both sides.

That is why the first iOS build failed the way it did: it got through every dependency —
`rbook`, `uuid`, `directories`, `dioxus-primitives`, and `libsqlite3-sys` compiling bundled
SQLite against the iPhoneSimulator SDK — and then produced **five errors in four files**,
every one of them the name `dioxus::desktop`.

The planning consequence is the important part. **The compiler can see one class of iOS
problem and it is the cheap one.** The expensive ones are all assumptions the desktop never
had to question:

- a file the user picks has a *path* — on iOS it has a security-scoped URL and a sandbox
  boundary;
- a window has a size worth remembering — on iOS the window is the screen;
- pages turn on arrow keys — there is no keyboard;
- the viewport is a rectangle — it is a rectangle minus a notch and a home indicator.

None of those produce a compile error. All of them produce a bad reading experience. So the
phase spends one step on the compiler and the rest on running the thing.

## Step plan

1. **Name the renderer once** — the five `dioxus::desktop` references behind one cfg-gated
   alias. Two-target build.
2. **Launch it** — `dx serve --platform ios` on iPhone and iPad. A discovery step.
3. **Get a book in** *(provisional)* — the import path under the sandbox.
4. **Turn pages by touch** *(provisional)* — tap zones and/or swipe.
5. **Fit the device** *(provisional)* — safe-area insets and thumb-sized chrome.
6. **Review and refactor** — the phase-closing pass.

Steps 3–5 are written from what the crux predicts, not from observation. Step 2 is expected
to re-order them.

---

## Step 1 — Name the renderer once

The whole compiler-guided half of the phase, and it is one idea: **the renderer has two names
and the crate should only know one of them.**

### The check — a two-target build

There is no unit test here; the claim is about which names resolve under which features, and
that is the compiler's job to answer. But it is a genuine runnable check with a genuine red,
and it has to be **two** commands, because the failure mode of this step is fixing iOS by
breaking desktop:

```sh
# the target: red now, green after
cargo check --target aarch64-apple-ios-sim --no-default-features --features mobile

# the safety net: 117 tests, unchanged
cargo test
```

**The red, observed before any edit** — 5 errors across 4 files:

```
src/epub.rs:4:13:        error[E0433]: cannot find `desktop` in `dioxus`
src/epub.rs:4:13:        error[E0432]: unresolved import `dioxus::desktop`
src/window.rs:1:13:      error[E0433]: cannot find `desktop` in `dioxus`
src/ui/reader.rs:275:26: error[E0433]: cannot find `desktop` in `dioxus`
src/main.rs:68:27:       error[E0433]: cannot find `desktop` in `dioxus`
error: could not compile `ook-reader` (bin "ook-reader") due to 5 previous errors
```

Note what is **not** in that list: `src/window.rs:19`, the other `dioxus::desktop` reference
in the repo. It sits inside the `#[cfg(target_os = "macos")]` arm of `remember_frame` and is
compiled out before name resolution ever looks at it. Five errors, six references — the
arithmetic is a small confirmation that the macOS gating already in the file is doing its job.

### The code

Four lines at the crate root, next to the `mod` declarations in `src/main.rs`:

```rust
#[cfg(feature = "desktop")]
pub(crate) use dioxus::desktop as renderer;

#[cfg(all(feature = "mobile", not(feature = "desktop")))]
pub(crate) use dioxus::mobile as renderer;
```

Then the five references become `crate::renderer::…`:

| file | from | to |
|---|---|---|
| `src/window.rs:1` | `use dioxus::desktop::tao::window::Window;` | `use crate::renderer::tao::window::Window;` |
| `src/window.rs:19` | `use dioxus::desktop::tao::platform::macos::WindowExtMacOS;` | `use crate::renderer::tao::platform::macos::WindowExtMacOS;` |
| `src/epub.rs:4` | `use dioxus::desktop::{use_asset_handler, wry::http::Response};` | `use crate::renderer::{use_asset_handler, wry::http::Response};` |
| `src/main.rs:68` | `dioxus::desktop::use_window()` | `crate::renderer::use_window()` |
| `src/ui/reader.rs:275` | `dioxus::desktop::use_window()` | `crate::renderer::use_window()` |

`window.rs:19` is not one of the five errors, but change it too — leaving one lone
`dioxus::desktop` behind in a macOS-only branch is exactly the sort of thing that reads later
as deliberate when it was just missed.

**Two forks worth deciding rather than inheriting.**

*The name.* `renderer` over `platform`, because `platform` is already taken twice in the
neighbourhood — `tao::platform::macos` (which would give you `crate::platform::tao::platform::macos`)
and `dx --platform`. What is being aliased is literally the renderer crate.

*The home.* Four lines at the crate root need no file. If you would rather have somewhere to
write down *why* this exists — and the why is subtle enough to deserve it — a `src/renderer.rs`
holding `pub(crate) use dioxus::desktop::*;` under the same two cfgs behaves identically, at
the cost of a glob re-export and a new module.

### Why it works

**Why the name vanished but the code did not.** The two `pub use` lines above are the entire
difference between `dioxus::desktop` and `dioxus::mobile`. `dioxus`'s own manifest confirms
it: `desktop = ["dep:dioxus-desktop", "dioxus-config-macro/desktop"]` and
`mobile = ["dep:dioxus-desktop", "dioxus-config-macro/mobile"]` — the same dependency, and
the only real divergence is which config macro gets enabled. So aliasing the module is not a
compatibility shim papering over two implementations. It is naming one implementation once.

**Why `not(feature = "desktop")` is load-bearing, and the real lesson of this step.** Cargo
features are **additive and unified across the build graph** — they are not an enum, and
Cargo has no notion of two features being mutually exclusive. `cargo check --target
aarch64-apple-ios-sim --features mobile` (without `--no-default-features`) leaves the default
`desktop` on and turns *both* on at once; that exact invocation is how the codebase was first
proved to cross-compile. Without the guard, both `use` statements would be live and rustc
would answer with `E0252: the name renderer is defined multiple times`. The guard makes the
second arm mean "mobile, and desktop did not already claim the name" — which is what you
meant, and which the first arm's plain `#[cfg(feature = "desktop")]` does not need because it
is the one being deferred to.

This generalises past this step: **any time two Cargo features look like alternatives, one of
them has to say `not(the other)` or some caller will eventually enable both.**

**Why the alias rather than the one-line feature union.** Adding `"dioxus/desktop"` to the
`mobile` feature fixes the build with zero source churn, and it genuinely works today — the
config macros are the only thing that diverges, and this app calls plain `dioxus::launch(App)`
and uses none of them. It is still the worse answer, because it buys that one line by making
`dx serve --platform ios` build with the `desktop` feature *on*. The flags stop describing the
platform. The day anything branches on `feature = "desktop"` — a `dioxus::config` macro, a
future `cfg` of your own — the phone build quietly takes the desktop branch, and the bug will
not look like a feature-flag bug. The alias keeps one name in the code and keeps the flags
honest.

### Scope note

**This step launches nothing.** It makes `cargo check` green for the simulator target and
leaves `cargo test` at 117. The simulator does not boot until Step 2, which is also the first
point at which any of the phase's interesting problems become visible.

It also does not touch **`FRAME_AUTOSAVE_NAME`**, which the iOS build reports as dead code —
`remember_frame`'s non-macOS arm is a no-op that never reads it. That warning is correct and
it is one line to silence, but it is a second idea (dead code under a `cfg`), and it belongs
either folded in deliberately or parked for Step 6. Parked, and noted here so it is not
mistaken later for something nobody saw.
