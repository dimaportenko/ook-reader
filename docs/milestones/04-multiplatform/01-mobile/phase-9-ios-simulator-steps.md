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
2. ~~**Launch it**~~ — `dx serve --platform ios` on iPhone and iPad. A discovery step.
   **Done** — no commit; it produced no diff.
2a. ~~**Drive it by tap**~~ *(added by `lbb:refine`, because Step 2 ran out of fingers)* — stand up
   [`agent-device`](https://github.com/callstack/agent-device) and use it to close Step 2's two
   blocked observations. Not a feature step: it is the phase's missing **check**.
   **Done** — tooling in `99d68c7`; no `src/` diff.
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
4. ~~**The top safe area is already respected**, on the iPhone's Dynamic Island and the iPad's
   status bar alike, with no `viewport-fit` or `env()` work. **This shrinks Step 5.**~~
   **Wrong — retracted by [Step 2a](#finding-b--the-top-safe-area-is-not-handled-and-step-2-was-wrong-about-it).**
   The inset is applied as an *offset* without the viewport being shrunk, so every `100vh`
   screen hangs 32pt off the bottom. This claim was read off a **library** screenshot, where
   the content is too short for anything to fall off the bottom edge. Step 5 does not shrink;
   it grows.

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

> **Superseded, and that is the point.** "The learner's eyeball" is a fine answer once; it is a
> bad answer for **Step 4**, whose whole subject is what a *swipe* does. So the wall this step
> hit turned out to be a phase-level gap rather than a Step 2 gap, and it is closed by
> [Step 2a](#step-2a--drive-it-by-tap), which was added afterwards by `lbb:refine`. The two
> outstanding observations belong to 2a now; this entry keeps them only as the record of why 2a
> exists.

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

- ~~**Step 5 (fit the device) shrinks.**~~ **Retracted by Step 2a** — the top inset is not
  handled, and it is the reason the reader's nav bar is unreachable at rest on an iPad. Step 5
  grows, and is now load-bearing for the phase's bar. The app icon, which was not on the list
  at all, joins it.
- **Step 3 (get a book in) is unchanged and is still the risk.** Nothing observed here makes
  `file.path()` more or less likely to work; it is simply still untested. *(Step 2a tested it:
  the risk was real and worse than stated — the picker never opens at all.)*
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

> **Status:** done — **no commit, because there is no diff.** The step's check was the launch
> itself and its product is the findings above; `cargo test` stayed at **117**, unchanged. The
> two observations it could not make were closed by Step 2a, and **finding 4 was retracted
> there** — the strikethrough above is the record of the correction, not a tidy-up.

---

## Step 2a — Drive it by tap

> **Added by `lbb:refine`** after Step 2 landed. It is not a step the plan predicted; it is the
> plan noticing that the next four steps have no runnable check.

### Why this is a step and not a footnote on Step 2

Step 2 ended with two questions it could not answer — *does the reader open?* and *does
`Choose Files` hand back a path `fs::copy` can read?* — and parked both as "the learner's
eyeball". Read that as a Step 2 shortfall and you fix it by tapping twice. Read it as a
**phase** shortfall and something larger is visible: every step from here has the shape *do
something on the device, then look at what happened*, and this project's two verification
tools cannot reach that shape.

- `cargo test` cannot see a running app. Nothing in the crate observes a simulator.
- `dx serve` + eyeball assumes a pointer you are already holding, and assumes the thing you
  are checking is *visible*. **Step 4's subject is a swipe** — a gesture, not a picture — and
  its failure mode is "the page did not turn", which a screenshot cannot distinguish from
  "you swiped in the wrong place".

So the wall Step 2 hit is the phase's missing third verification tool.
[`agent-device`](https://github.com/callstack/agent-device) (Callstack) is a CLI *and* an MCP
server that drives a simulator: open an app, snapshot its accessibility tree into referenceable
elements, press one, settle, look at the diff. This step is where it enters the project — and,
because it is infrastructure rather than a feature, where its limits get established before
three later steps are built on top of it.

### The setup — done, and it is user-owned by default

```sh
npm install -g agent-device@latest          # -> agent-device 0.20.9, /opt/homebrew/bin
npx skills add callstack/agent-device       # -> 4 skills, .agents/skills/ + .claude/skills/ symlinks
agent-device doctor
```

Node here is **v22.21.1** against the package's `engines.node: >=22.12`. `doctor` returns
**warn, no hard blockers** — the only complaints are a missing HarmonyOS `hdc` and a missing
Vega CLI, neither of which this project will ever have. It also warms an Xcode runner build in
the background on first run, so the first `open` is the slow one.

The skills arrive as a set of four — `agent-device`, `ios-simulator`, `android-emulator`,
`dogfood`. Two are out of this phase's scope and were kept anyway rather than hand-pruning a
tracked install; `skills-lock.json` records all four, the same way it records
`rust-best-practices`.

> **Installation was run here only because it was explicitly asked for.** The tool's own skill
> says *"Treat installation and upgrades as user-owned setup steps. Do not run that command
> autonomously."* That remains the standing rule in [`AGENTS.md`](../../../../AGENTS.md); this
> was an instruction, not an agent deciding to install something.

**One gotcha worth the line:** sessions are keyed by **working directory**
(`cwd:9c17880f281d1877:default`). Running `agent-device screenshot` from a scratch directory
after opening from the repo root answers `SESSION_NOT_FOUND`. Drive from the repo root.

### The answer to the open question: the tree is healthy

The step's one genuine uncertainty was whether a WKWebView publishes anything addressable.
It does — **outcome #1, not the sparse fallback**:

```
$ agent-device open com.dimaportenko.ook-reader --platform ios --foreground --device "iPad Pro 13-inch (M5)"
Snapshot: 9 nodes
@e1 [application] "OokReader"
@e2 [window]
@e3 [webview] "Dioxus app"
@e4 [other] "Dioxus app"
@e5 [other] "Remove"
@e6 [button]
@e7 [button] "Remove"
@e8 [other] "Import EPUB"
@e9 [button] "Import EPUB"
```

`snapshotQuality: {state: "healthy", backend: "tree"}`. In the reader it is 64 nodes deep —
every ToC link by name, the headings, the pager. Sub-frames get a `~sN` pin (`@e39~s932177`),
which is how the paginated iframe's contents are addressed. `rect`s come back in the JSON, so
the harness doubles as a **layout probe**, which turned out to matter more than the tapping.

So Dioxus + wry on iOS is drivable, and every step after this one has a real check.

### Finding A — four buttons with no accessible name

Predicted as outcome #3 and half-true. The tree is there; some of the **labels** are not:

| ref | what it is | source |
|---|---|---|
| `@e6` (library) | the book cover — the primary control on the screen | `src/ui/library.rs:44`, `button.book-cover` wrapping an `img` with no `alt` |
| `@e6` (reader) | close | `src/ui/reader.rs` top bar |
| `@e9` (reader) | contents | " |
| `@e10` (reader) | settings | " |

The cover one is the sharpest: it is a *real* `<button>` — the markup is semantically right —
and it is still anonymous, because everything inside it is an unlabelled image. `--overlay-refs`
annotated two of nine nodes and skipped it.

**This is a finding about this app, not about the tool, and it is not a mobile problem.** The
same four buttons are unnamed under VoiceOver on the desktop build; iOS is just the first place
anything looked. An `alt` on the cover `img` and three `aria-label`s would close it.
**Scheduled for Step 6**, not fixed here.

### Finding B — the top safe area is *not* handled, and Step 2 was wrong about it

The correction this step exists to have found. Step 2 reported the top inset as "already
respected, with no `viewport-fit` or `env()` work". It is not, and the geometry says so exactly.

Every number below is from `snapshot --json`, on the iPad Pro 13-inch (M5):

| | |
|---|---|
| window / webview | `1032 × 1376` at `y=0` — the webview is the whole screen |
| Dioxus document root | height **1376** — this is `height: 100vh` from `src/ui/reader.rs:164` |
| root `y`, scrolled to top | **+32** |
| root `y`, scrolled to bottom | **−20** |
| scroll range on a screen that should not scroll | **52pt** |

Read those two offsets together and the bug is plain. The document is exactly as tall as the
screen, but it is laid out starting **32pt down**, below the status bar — so `100vh` is 32pt
too tall to fit, and the bottom of the reader falls off the bottom of the display. The
consequences, both observed:

- **Scrolled to top** — top bar visible; the nav bar sits at `y=1380` on a 1376-tall screen.
  `press` refuses it outright: *"Ref `@e64` is off-screen and not safe to press."*
- **Scrolled to bottom** — nav bar visible at `y=1338`; the close button is now at `y=−8`,
  clipped under the status bar, and pressing it does nothing.

**You can have the top chrome or the bottom chrome. Never both.** On an iPad, at rest, the
reader opens with no way to turn a page.

> **Why Step 2 got it wrong, which is the same mistake it already confessed to once.** Step 2
> judged the safe area from screenshots of the **library** — a short screen where nothing
> reaches the bottom, so nothing visibly falls off it. The top of the content did clear the
> status bar, and that was read as "the inset is handled". What is actually happening is worse
> than no inset at all: the layout is *offset* down without the viewport being *shrunk*, which
> silently pushes 32pt of every full-height screen past the bottom edge. **A screen that
> happens to fit is not evidence that the viewport is right** — the same shape of error as
> reading a placeholder cover as proof of the asset protocol.

### Finding C — paging by tap works

```
$ agent-device press @e64 --settle
Tapped @e64 (584, 1338)
settled after 758ms: +1 -1 (~63 unchanged)
- @e63 [text] "Page 1 of 2"
+ @e63~s932186 [text] "Page 2 of 2"
```

Tapping the book cover opens the reader on a real book — **Step 2's first outstanding
observation, closed** — and the `Next` button repaginates. So the whole reading path works on
iPad under touch, once you can reach the button. Swipe is untested; that is Step 4.

### Finding D — `<input type="file">` is inert on iOS, and this rewrites Step 3

**Step 2's second outstanding observation, closed — with a worse answer than the one the plan
was braced for.**

Pressing the import control does nothing. Not a wrong path, not an error — nothing:

```
$ agent-device press @e9~s932197 --settle
Tapped @e9 (234, 454)
settled after 662ms: +0 -0 (~9 unchanged)
```

Tried again on the `Choose Files` pill by coordinate (`146 454`, dead centre per
`--overlay-refs`): identical. No picker, no dialog, no view change, and `agent-device logs`
over the tap shows nothing but focus churn — no upload panel, no presentation failure, no
exception. Taps in general reach the webview; the cover and `Next` both responded.

**A hypothesis with source evidence, short of proof.** In wry 0.53.5 the file-upload handler
is macOS-only:

```rust
// wry-0.53.5/src/wkwebview/class/wry_web_view_ui_delegate.rs:101
#[cfg(target_os = "macos")]
#[unsafe(method(webView:runOpenPanelWithParameters:initiatedByFrame:completionHandler:))]
fn run_file_upload_panel(…)
```

That method is macOS-only in WebKit too, so its absence is not itself the cause — on iOS
WebKit presents its own `WKFileUploadPanel`. But presenting anything on iOS needs a
`UIViewController`, and wry's `wkwebview` module never creates one: it only ever
`addSubview`s the webview. No view controller, nothing to present from, silent no-op. Fits
every observation; unproven until something fixes it.

**What it does to the plan:** Step 3 was written as *"the picker returns a security-scoped URL
and `file.path()` may not be readable."* That question is moot — the picker never opens.
Step 3 is now a **native import channel** (a `UIDocumentPickerViewController` through
`objc2`, or the Files-app "Open in" route, or an in-app source that is not the filesystem at
all), and it has gone from the phase's likeliest split to its largest step.

### Why it works — the mechanism worth keeping

**The accessibility tree is the API, and it is also a mirror.** A UI driver has no privileged
view of the app; it reads the tree the OS builds for assistive technology, and addresses
elements by what that tree calls them. Two things follow, and both showed up here within one
session: testability and accessibility are the *same property* (Finding A is a VoiceOver bug
the harness found while looking for a tap target), and **`rect`s in that tree are ground truth
about layout** — Finding B is a bug no screenshot produced and no eyeball would have explained,
recovered from two numbers.

**Refs are frame-scoped, and the tool enforces it.** `@e12` means "the twelfth node of the
snapshot you are looking at now". Reuse one after a mutation and you get
*"Ref `@e6` needs a complete snapshot — the current frame only authorizes its emitted refs"*
rather than a tap on whatever is twelfth this time. `--settle` exists so the next ref comes
from the state you actually caused. That refusal is the feature: the standard way these
harnesses go quietly wrong is a stale ref that still resolves.

**Refusing to press is also the feature.** *"off-screen and not safe to press"* is how Finding
B surfaced. A harness that helpfully scrolled-then-tapped would have turned the page and
reported success, and the reader would still be unusable at rest on an iPad.

### Scope note

**No Rust was written and `src/` is untouched.** `cargo test` stays at **117**. What landed is
the harness, four skills, `skills-lock.json`, and the findings above.

Nothing found here was fixed: the missing labels go to **Step 6**, the safe-area overflow to
**Step 5**, the import channel to **Step 3**. Swipe was not attempted (**Step 4**), the iPhone
was not re-driven (the geometry bug is a viewport bug and will be worse on a phone, not
different), and no `.ad` replay script or CI wiring was written.

**What it changes about the plan, which is most of the plan:**

- **Step 3 is rewritten and is now the phase's biggest step** — not a path problem, a missing
  picker.
- **Step 5 grows back.** Step 2 said it shrank because the top inset looked handled. It is not
  handled, and it is the reason the reader cannot be paged at rest. Step 5 is now load-bearing
  for the phase's own bar — *a book, on a tablet, that you can read.*
- **Step 4 is half-answered.** Button paging works under touch; only swipe is open.
- **A new Step 6 item:** accessible names for four buttons.

> **Status:** done — the harness, its four skills and the `AGENTS.md` rules are committed in
> `99d68c7`; this build-log entry follows it. **No `src/` diff:** `cargo test` is **117 passed,
> 0 failed**, unchanged, and `cargo clippy --all-targets` is clean apart from the unrelated
> `block v0.1.6` future-incompat note.
>
> **Written by:** the agent, at the user's explicit instruction to install the tool and run the
> mobile checks — including the `npm install -g`, which the standing rule in `AGENTS.md`
> otherwise reserves for the user.
>
> **How it was verified, stated precisely.** This is a step whose check is a running app, and
> the usual gate for those in this project is *the learner's eyeball*. That is **not** what
> happened here: the app was driven by the agent, and the evidence is machine-readable rather
> than visual — snapshot JSON with rects, press diffs, and the app log, all quoted verbatim
> above. Stronger than an eyeball for the geometry, and weaker in one specific way: **nobody has
> yet held the iPad and formed an opinion about how it feels.** Findings B and D are the ones to
> re-check by hand if any of this is ever doubted.
>
> **Nothing found here was fixed.** Import → Step 3, safe area → Step 5, accessible names →
> Step 6. All three are recorded in the phase doc's checklist.
