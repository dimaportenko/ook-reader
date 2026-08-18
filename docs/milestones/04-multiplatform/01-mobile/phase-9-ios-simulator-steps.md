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

1. ~~**Name the renderer once**~~ — the five `dioxus::desktop` references behind one
   cfg-gated alias. Two-target build. **Done** — `1fc64eb`.
2. **Launch it** — `dx serve --platform ios` on iPhone and iPad. A discovery step.
3. **Get a book in** *(provisional)* — the import path under the sandbox.
4. **Turn pages by touch** *(provisional)* — tap zones and/or swipe.
5. **Fit the device** *(provisional)* — safe-area insets and thumb-sized chrome.
5a. **Give it an icon** *(provisional)* — the springboard tile is blank. Found by Step 2.
6. **Review and refactor** — the phase-closing pass.

Steps 3–5 are written from what the crux predicts, not from observation. Step 2 is expected
to re-order them.

## Step 1 — Name the renderer once

> **Status:** done — committed in `1fc64eb` (117 tests green, both targets check clean).
> **Written by:** `lbb:next-implement` — implementation written by the agent, reviewed by
> hand. No test: the step's check is the compiler.

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

**The green, after the edit.** `cargo check --target aarch64-apple-ios-sim
--no-default-features --features mobile` finishes clean; `cargo test` is 117 passed, 0
failed, unchanged; `cargo clippy --all-targets` is clean. The iOS check keeps one warning —
`constant FRAME_AUTOSAVE_NAME is never used` — which is the known one, parked below.

**A third invocation, run to prove the guard rather than to pass.**
`cargo check --target aarch64-apple-ios-sim --features mobile` — no
`--no-default-features`, so `desktop` stays on and *both* features are live at once. It
compiles. That is the `not(feature = "desktop")` arm doing its job: without it this exact
command is the one that answers `E0252: the name renderer is defined multiple times`. Worth
running once, because it is the invocation a future contributor is most likely to type by
accident.

### Scope note

**This step launches nothing.** It makes `cargo check` green for the simulator target and
leaves `cargo test` at 117. The simulator does not boot until Step 2, which is also the first
point at which any of the phase's interesting problems become visible.

It also does not touch **`FRAME_AUTOSAVE_NAME`**, which the iOS build reports as dead code —
`remember_frame`'s non-macOS arm is a no-op that never reads it. That warning is correct and
it is one line to silence, but it is a second idea (dead code under a `cfg`), and it belongs
either folded in deliberately or parked for Step 6. Parked, and noted here so it is not
mistaken later for something nobody saw.

---

## Step 2 — Launch it

> **Status:** recorded, **gate not closed.** The app builds, installs, and renders on both
> simulators, and the sandbox claims below were re-verified against the iPad's data container.
> But two eyeball checks this step named are still outstanding — tapping the book to open the
> reader, and tapping `Choose Files` to import — because `simctl` cannot synthesize taps. The
> step closes when those are confirmed by hand, not before.
>
> **Written by:** `lbb:next-implement` — but see the scope note: **this step has no diff.** It
> is a discovery step, and nothing in `src/` needed changing to complete it.

The first time the app runs on a phone. The step's job is not to build anything — it is to
replace the crux's *predictions* with observations, so Steps 3–5 are derived from what iOS
actually does rather than from what the desktop assumptions suggest it might.

### The check — `dx build` + the simulator, and an eyeball

No test; nothing in the crate can see a running app. There is also no red to watch, because
the step's claim ("it launches") has no failing state short of a crash.

```sh
dx build --platform ios
xcrun simctl boot <ipad-udid> && open -a Simulator
xcrun simctl install <ipad-udid> target/dx/ook-reader/debug/ios/OokReader.app
xcrun simctl launch  <ipad-udid> com.dimaportenko.ook-reader
xcrun simctl io      <ipad-udid> screenshot ipad.png
```

The gate is the library screen appearing. It appeared, on **iPad Pro 13-inch (M5)** and
**iPhone 17**, both iOS 26.5. `dx build --platform ios` exits 0 and bundles
`OokReader.app` with `UIDeviceFamily = [1, 2]` — one universal binary, both device families,
exactly as the phase doc predicted.

### What was found

**Three things work that were not certain, and one of them is the phase's central mechanism.**

1. **The app launches and renders on both devices.** No crash, no panic, no blank webview.
2. **Persistence works inside the sandbox, end to end.** `Config::app_dir()` → `directories`
   → `…/Library/Application Support/com.dimaportenko.ook-reader/`; `ensure_dirs()` created
   `books/`; `Db::open` created `library.sqlite3`. Verified by reading the simulator's data
   container, not inferred from the app not crashing. **`directories` mapping to the iOS
   container was a guess in the phase doc and is now a fact.**
3. **`use_asset_handler` — wry's custom protocol — works on iOS.** This is the important one.
   Every EPUB resource the reader serves goes through it, so if it had not worked the phase
   would have been a rewrite rather than a port. Proved by seeding a real cover file and
   watching the book's own blue *Strand Library* cover render through `/covers/{name}`.
4. **The top safe area is already respected**, on the iPhone's Dynamic Island and the iPad's
   status bar alike, with no `viewport-fit` or `env()` work. **This shrinks Step 5** — see
   below.

**Two things are wrong and neither is fatal.**

5. **The app icon is blank on the home screen.** `Dioxus.toml`'s `icon` list is `.icns`,
   `.ico` and desktop PNGs; iOS wants its own sizes. Cosmetic, but it is the first thing you
   see — and **it is a hole in the port, not a setting that was missed.** That key belongs to
   `dx bundle`, and dx 0.7.9's iOS path never reads it:

   | | |
   |---|---|
   | the iOS plist template | `assets/ios/ios.plist.hbs` carries no icon key at all — no `CFBundleIcons`, no `CFBundleIconFiles` |
   | the generated plist | `plutil -p …/ios/OokReader.app/Info.plist` matches nothing on `icon`, case-insensitive |
   | the built `.app` | holds exactly three entries: `Info.plist`, `assets/`, and the binary. No `Assets.car`, no `AppIcon*.png` |

   dx's escape hatch is `[application].ios_info_plist` (`apple.rs:186`), a path to an
   `Info.plist` of your own that replaces the generated one wholesale. That covers the plist
   keys; getting the image files *next to* it inside the `.app` is the second half and is not
   yet established.
6. **The chrome is desktop chrome.** A single top-left column, a `Remove` button as wide as
   the cover, and the import control rendering as WKWebView's stock `Choose Files` pill.
   Legible, and clearly not designed for a tablet.

**Two things could not be tested, and the reason is worth recording.** `simctl` cannot
synthesize taps, and driving the Simulator through System Events needs macOS Accessibility
permission this session does not have — `osascript` returned error `-25204`. So **tapping the
book to open the reader, and tapping `Choose Files` to import, are both outstanding and are
the learner's eyeball.** Everything above was verified without a single tap; everything below
needs one.

> **A fixture is left in the iPad simulator on purpose.** The Sherlock Holmes epub was seeded
> directly into the sandbox — file copied into `books/`, row inserted into `books`, cover
> extracted and `cover_path` set — precisely because import could not be tested. It is a
> **diagnostic, not Step 3**: it proves the *reader* can be reached without waiting on the
> *importer*. Tap it and the reader opens on a real book.

**One correction, recorded because the mistake is instructive.** The first seeded book showed
a cover and that was briefly read as proof the asset handler worked. It was not: `cover_path`
was `NULL`, so `cover_name()` returned `None` and `LibraryBooks` took its **placeholder**
branch — `placeholder-2.jpg` with the title and author drawn over it, which looks convincingly
like a cover. The protocol was only actually exercised once a real `.cover.jpg` was seeded and
`cover_path` set. *A rendered image is not evidence about where the image came from.*

### What this changes about the plan

- **Step 5 (fit the device) shrinks.** The top inset it was mostly about is already handled.
  What is left is the bottom home-indicator region and thumb-sized chrome — and the app icon,
  which was not on the list at all.
- **Step 3 (get a book in) is unchanged and is still the risk.** Nothing observed here makes
  `file.path()` more or less likely to work; it is simply still untested.
- **A new item: the app icon — Step 5a.** First read as a footnote on Step 5, then given its
  own step once the evidence above showed it is not one line of TOML: it needs a plist dx does
  not generate *and* an image-into-bundle mechanism that is still unknown. Small, but it has a
  research half.

### Scope note

**This step wrote no code, and that is the finding, not a shortfall.** The phase doc called
Step 2 a discovery step; discovery found nothing broken enough to need a source change. The
crate is untouched: **117 tests green**, `cargo clippy --all-targets` clean on desktop, and
the iOS target still carrying only the known `FRAME_AUTOSAVE_NAME` dead-code warning parked
in Step 1.

It deliberately does **not** open the reader, import a book, touch layout, or add an icon.
Those are Steps 3–5, now with better information behind them.
