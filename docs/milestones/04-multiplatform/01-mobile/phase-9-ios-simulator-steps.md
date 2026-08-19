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
3. ~~**Get a book in**~~ — ~~the import path under the sandbox~~ a native
   `UIDocumentPickerViewController`, presented from the root controller tao was making all
   along. **Done** — `badd372`.
4. ~~**Turn pages by touch**~~ — the buttons already worked under tap, so this was swipe: the
   frame measures the gesture, `Turn::of_swipe` decides. **Done** — `bb18938`.
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

## Step 3 — Get a book in

> **Status:** done — committed in `badd372` (118 tests green, both targets clippy-clean).
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand. One new test, plus a full round trip driven on the iPad Pro 13-inch (M5)
> simulator and an `ios:dx:serve` eyeball.

Step 2a left this step rewritten and frightening: *the picker never opens, and the likeliest
reason is that wry puts the webview in no `UIViewController`, so WebKit has nothing to present
its upload panel from.* If that were true, the fix would be somewhere between hard and
upstream.

**It is not true, and five minutes in tao's source says so.** `tao-0.34.8`'s iOS window builder
creates a `TaoUIViewController` (`platform_impl/ios/view.rs:458`), sets it as the `UIWindow`'s
`rootViewController` (`:529`), and hands its view to wry, which `addSubview`s the `WKWebView`
into it (`wry-0.53.5/src/wkwebview/mod.rs:652`). tao even exposes the controller deliberately,
as `WindowExtIOS::ui_view_controller()`. **There has been a presenter the whole time.**

### And the inert `<input type="file">` is Dioxus's own stub

*Found while reviewing the finished step, by asking the obvious question: isn't there a
standard Dioxus way to pick a file?* There is — it is `<input type="file">` — and the answer
turns out to be Finding D's missing cause.

**Dioxus never lets WebKit handle a file input.** `dioxus-desktop` injects JS that intercepts
the click and routes it through its own `__file_dialog` custom protocol
(`dioxus-desktop-0.7.9/src/protocol.rs:67`), which calls [`rfd`](https://crates.io/crates/rfd).
And the entry point is cfg-gated by OS:

```rust
// dioxus-desktop-0.7.9/src/file_upload.rs:44
#[cfg(not(any(
    target_os = "windows", target_os = "macos", target_os = "linux",
    target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"
)))]
pub(crate) fn get_file_event(&self) -> Vec<PathBuf> {
    vec![]
}
```

iOS lands in that arm. `rfd-0.17.2` confirms it from the other side: backends for `macos`,
`win_cid`, `gtk3`, `xdg_desktop_portal`, and `wasm`, and **zero occurrences of
`target_os = "ios"`** in the whole crate.

So the tap was never inert. Dioxus *handled* it, called a stub, and returned an empty file
list — which is exactly why Step 2a saw focus churn in the log, no picker, no error, and
`+0 -0`. Both of Step 2a's guesses were wrong in the same direction: they looked for something
**missing in iOS**, when the gap was in **the layer we control**.

> **The shape that would delete this whole step.** An iOS backend in `rfd`, or an iOS arm on
> `FileDialogRequest::get_file_event`, makes `<input type="file">` work on iOS — at which point
> `src/document_picker.rs` and both `ImportPicker` bodies disappear and `ImportControl` goes
> back to one body. That is the right long-term fix and it is upstream work, not this phase's;
> it is recorded here so the local copy is understood as a *stand-in for a missing backend*
> rather than as the way this has to be done.

### The crux — the picker is not the app's, and neither is the file

Two things about `UIDocumentPickerViewController` shape the whole step.

**It runs in another process.** The Files sheet is a *remote view controller*: it draws over
your window but its views are not yours, so it cannot read your app's sandbox and your app
cannot read its accessibility tree. That is why the results arrive through a **delegate
callback** rather than a return value, and why `agent-device snapshot` sees only
`[other] "dismiss popup"` while the picker is up — the file rows have to be tapped by
coordinate.

**The file it hands back is not where you think.** In `asCopy: true` mode iOS copies the chosen
document into *your* sandbox first (`tmp/<bundle-id>-Inbox/`) and gives you a plain, readable
path. In `asCopy: false` mode you get the original's URL and must bracket every read in
`startAccessingSecurityScopedResource` / `stop…`. This step takes the copy, because
`Library::add_from_path` was always going to copy the bytes into `books_dir` anyway — the
security-scoped dance would buy nothing but a chance to forget the `stop`.

### The check — a test for the half that is testable, a simulator for the half that is not

Two callers now feed one import path — the desktop `<input type="file">` and the iOS picker —
and the thing worth asserting is the shared half: **that a batch import counts every source and
does not stop at the first bad one.** That is host-testable, so it is a `#[test]`:

```rust
#[test]
fn add_all_counts_every_source_and_keeps_going_past_a_bad_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (library, source, _) = library_with_source(&dir);
    let missing = dir.path().join("not-a-book.epub");

    let summary = library.add_all(&[missing, source], 1_000);

    assert_eq!(summary, ImportSummary { added: 1, failed: 1 });
    assert_eq!(library.list().expect("list succeeds").len(), 1);
}
```

The bad source goes **first** on purpose: the assertion that matters is that the good one still
lands. Red was `no method named add_all found for struct library::Library`.

The FFI half has no host test — there is no `UIDocumentPickerViewController` on macOS to stand
one up against — so its check is the simulator, driven end to end:

```
$ agent-device press @e10 --settle          # Import EPUB
$ agent-device press 241 640 --settle       # "On My iPad"  (coordinates: remote view controller)
$ agent-device press 530 490 --settle       # holmes.epub   (selectable — the UTType filter works)
$ agent-device press 865 400 --settle       # Open
settled after 2613ms: +10 -2
+ @e25 [text] "Imported 1 books"
```

The fixture was placed where the picker could see it by dropping it into the simulator's
"On My iPad" storage — the `group.com.apple.FileProvider.LocalStorage` app group container —
which is the headless stand-in for dragging a file onto the Simulator window.

### The code

`Cargo.toml` gains an iOS-only block, mirroring the macOS one: `objc2`, `objc2-foundation`,
`objc2-ui-kit`, and `objc2-uniform-type-identifiers`, all `default-features = false` because
these crates otherwise bind every class in their framework. `objc2-ui-kit` needs `block2` for
`presentViewController:animated:completion:` and the `objc2-uniform-type-identifiers` feature
for `initForOpeningContentTypes:asCopy:`.

`src/document_picker.rs` is new, and sits beside `src/window.rs` as the crate's second
platform shim. It declares one Objective-C class:

```rust
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OokDocumentPickerDelegate"]
    #[ivars = Box<dyn Fn(Vec<PathBuf>)>]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn documentPicker_didPickDocumentsAtURLs(…) { … (self.ivars())(paths); }
    }
);
```

and one function, `pick_epubs(window: &Window, handle: impl Fn(Vec<PathBuf>) + 'static)`, which
resolves the root controller through `window.ui_view_controller()` with the same
`cast` → `as_ref` → early-return shape `window.rs` already uses for `ns_window()`.

`Library::add_all(&[PathBuf], now) -> ImportSummary` is the shared import loop, lifted out of
the `onchange` closure it used to live inside. `ImportControl` now holds one
`Callback<Vec<PathBuf>>` and renders a `#[cfg]`-selected `ImportPicker` — a `<button>` on iOS,
the unchanged `<input type="file">` everywhere else — so the two platforms differ only in how
paths are *produced*.

### Why it works

**A `Callback` carries its own runtime, which is what makes the delegate callable at all.**
The picked paths arrive on the main thread from UIKit's run loop, nowhere near a Dioxus render
or event handler — so a bare `signal.set()` would be reaching for a runtime that is not on the
stack. `dioxus_core::Callback::call` (`events.rs:519`) upgrades its own stored `Weak<Runtime>`,
installs a `RuntimeGuard`, and pushes its origin scope before calling. That is precisely the
"call into Dioxus from a foreign callback" seam, and it is why this step needs no channel, no
coroutine, and no new dependency.

**The delegate is weak, so somebody has to hold it.** `UIDocumentPickerViewController.delegate`
is a weak property — set it and drop the `Retained`, and it is nil before the user has picked
anything. A one-slot `thread_local` keeps the live delegate alive and drops the previous one at
the next press, when it is provably idle.

**`#[thread_kind = MainThreadOnly]` is load-bearing, not decoration.** `UIDocumentPickerDelegate`
is declared `NSObjectProtocol + MainThreadOnly` in `objc2-ui-kit`, so the class cannot be
allocated without a `MainThreadMarker` — the thread-safety requirement UIKit only documents in
prose is enforced by the type system here, and `MainThreadMarker::new()` returning `None` is the
first guard in `pick_epubs`.

### Scope note

**It does not fix the inert `<input type="file">`, though it now knows why.** The cause is
`rfd`'s missing iOS backend, reached through Dioxus's own stub — see above. Fixing it properly
is upstream work and would leave the reader unable to import books until it shipped.

**It does not delete the copy iOS makes.** `asCopy: true` leaves the picked file in
`tmp/<bundle-id>-Inbox/` and nothing removes it — verified on device, one 379 KB `holmes.epub`
still sitting there after a successful import. Deliberately left for review: deleting files is
the riskiest line this step could have contained, and it is the learner's call.

**It does not carry the errors.** `ImportSummary` folds `library::Error` down to two counts, so
"Imported 4 books, 1 failed" can never say *which* or *why* — and `pick_epubs` returns silently
on all four of its guards, which is the same silence Step 2a spent a whole step diagnosing.
Both are noted for **Step 6**.

**It does not touch Android or the layout.** The cfg switch is on `target_os = "ios"`, not on a
capability, so Android's WebView will need a third arm rather than a second impl. And the
library screen it imports into is still the one Step 2a measured: the covers are cramped into
the top-left corner and the safe area is unhandled. That is **Step 5**, and it is next.

## Step 4 — Turn pages by touch

> **Status:** done — committed in `bb18938` (120 tests green, both targets clippy-clean, driven
> on the iPhone 17 simulator).
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent, reviewed
> by hand. The `unsigned_abs` hardening was found by `lbb:commit`'s review pass and applied at
> the user's instruction before the commit landed.

Step 2a already answered half of this step: the `Prev`/`Next` buttons repaginate under tap, so
"turn pages by touch" was never entirely open. What was open is the gesture a reader actually
uses, and the one [`TODO.md`](../../../../TODO.md) has wanted since before there was a phone to
run it on — **swipe**.

### The crux — a swipe is a measurement in one language and a decision in another

The gesture happens inside the sandboxed chapter iframe, which is the one place in this app
that Rust cannot see. So the temptation is to decide there: *if the finger moved more than
40 pixels leftward, post "next page"*. That is the shape almost every swipe tutorial has.

It is the wrong seam here, and this codebase has already picked the right one twice.
`key-listener.js` does **not** decide that ArrowRight means forward; it posts the key name and
lets `Turn::of` in `src/ui/reader.rs` decide. The host's own `onkeydown` posts nothing and calls
the same `Turn::of`. **One policy, two producers** — which is why "what turns the page" is a
thing you can read in one place and, crucially, a thing you can write a `#[test]` about.

A swipe gets the same treatment: **the iframe reports geometry, Rust decides intent.** The
threshold, the horizontal-vs-vertical tiebreak, and the sign-to-direction mapping are all Rust,
where they are testable on the host with no simulator in the loop. The JS is left with the one
job it alone can do: subtract two pointer positions.

### The check — a `#[test]` for the policy, a simulator for the gesture

The decision is pure arithmetic over two integers, so it is a host test, and it is written
against the *rejections* as much as the accepts — a swipe detector that fires on taps and
scrolls is worse than none:

```rust
#[test]
fn only_a_long_mostly_horizontal_drag_is_a_swipe() {
    assert_eq!(Turn::of_swipe(0, 0), None);
    assert_eq!(Turn::of_swipe(-12, 3), None);
    assert_eq!(Turn::of_swipe(-140, 220), None);

    assert_eq!(BridgeMsg::parse("swipe:0,0"), None);
    assert_eq!(BridgeMsg::parse("swipe:left"), None);
    assert_eq!(BridgeMsg::parse("swipe:-140"), None);

    assert_eq!(Turn::of_swipe(i32::MIN, 0), Some(Turn::Next));
    assert_eq!(BridgeMsg::parse("swipe:-2147483648,0"), Some(BridgeMsg::Turn(Turn::Next)));
}
```

**The red, observed before any implementation:**

```
error[E0599]: no variant, associated function, or constant named `of_swipe`
              found for enum `ui::reader::Turn` in the current scope
  --> src/ui/reader.rs:476:26   (and 4 more sites)
error: could not compile `ook-reader` (bin "ook-reader" test) due to 5 previous errors
```

The companion test is the by-now-standard three-hop assertion — the message kind and *both
payload field names* have to appear in the injected assets and in the bridge, because three
files share one wire format with no compiler between any two of them.

The gesture itself has no host test — there is no finger on a build machine — so its check is
the simulator, driven rather than eyeballed, for exactly the reason Step 2a gave: an eyeball
cannot tell "the handler is broken" from "you swiped in the wrong place". Baseline first, one
gesture at a time, reading the page label out of the accessibility tree each time:

```
$ agent-device snapshot -i     # baseline, after opening the book
[off-screen below] 3 interactive items: "Prev", "Page 12 of 79", "Next"

$ agent-device swipe 320 450  80 450   → Page 13 of 79   # right-to-left: forward
$ agent-device swipe  80 450 320 450   → Page 12 of 79   # left-to-right: back
$ agent-device swipe 320 450 300 450   → Page 12 of 79   # 20px: under the floor
$ agent-device swipe 200 600 200 300   → Page 12 of 79   # vertical: not a page turn
$ agent-device press  200 450          → Page 12 of 79   # a tap is still a tap
```

**Read the first attempt as a warning about this kind of check.** The very first swipe appeared
to jump from chapter 1 page 2 to chapter 3 page 12, which looks damning and means nothing: the
snapshot had been taken while the saved-position restore was still in flight, so the "before"
reading was of a screen the app was in the middle of leaving. The run above is the honest one —
relaunch, open the book, confirm the label is *stable across two snapshots*, and only then
gesture. A device check with no settled baseline can manufacture any result you like.

### The code

`src/web/assets/swipe-listener.js` is new, and is deliberately its own file rather than three
more lines in `pointer-listener.js`: every sibling in `INJECTED_ASSETS` owns exactly one
concern, and `pointer-listener.js`'s concern is "something was touched, close the popovers".

```js
let swipeFrom = null;

document.addEventListener("pointerdown", function (e) {
  swipeFrom = { id: e.pointerId, x: e.clientX, y: e.clientY };
});

document.addEventListener("pointerup", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  const dx = Math.round(e.clientX - swipeFrom.x);
  const dy = Math.round(e.clientY - swipeFrom.y);
  swipeFrom = null;
  if (dx === 0 && dy === 0) return;
  window.parent.postMessage({ kind: "ook-swipe", dx, dy }, "*");
});
```

`ook-events-listener.js` forwards it as `swipe:<dx>,<dy>`, and `src/ui/reader.rs` gains the
policy next to the one it already had:

```rust
const SWIPE_MIN_PX: u32 = 40;

fn of_swipe(dx: i32, dy: i32) -> Option<Turn> {
    if dx.unsigned_abs() < SWIPE_MIN_PX || dx.unsigned_abs() <= dy.unsigned_abs() {
        return None;
    }
    Some(if dx.is_negative() { Turn::Next } else { Turn::Prev })
}
```

plus one arm in `BridgeMsg::parse`, which lands on the existing `Turn` variant and therefore on
the existing `Turn::apply` → `state.page_next()/page_prev()`. **No new path through `nav`.**

### Why it works

**`pointerId` is what makes a second finger harmless.** Without it, a two-finger gesture fires
two `pointerdown`s — the second overwrites `swipeFrom` — and then the *first* finger's
`pointerup` computes a delta between one finger's start and another's end. That is a number with
no physical meaning, and it is large and horizontal often enough to turn a page for no reason.
Pinning the gesture to the pointer that started it makes the extra finger a no-op instead.

**The tiebreak is `<=`, not `<`.** A perfectly diagonal drag (`|dx| == |dy|`) is not a page turn;
it is an ambiguous gesture, and the safe reading of an ambiguous gesture in a reader is "do
nothing". Turning the page is not undoable by the same gesture that caused it — the reader has
to notice and swipe back — so the asymmetry of the mistake belongs in the comparison.

**Negative `dx` is *forward*, which reads backwards until you think about the finger.** The
content moves with the finger: dragging right-to-left pulls the next page in from the right,
the same direction the `translateX` in `pagination.css` already moves. The sign is about the
hand, not about the page number.

**`unsigned_abs` rather than `abs`, because the frame is not trusted** *(found reviewing the
step at commit time, and fixed before it landed)*. `i32::MIN.abs()` panics — "attempt to negate
with overflow" — and the wire value that reaches it is `swipe:-2147483648,0`, which parses
perfectly well. That is not a theoretical input: the chapter iframe carries `allow-scripts`, so
**a book's own JavaScript can post any message it likes** to the parent. `unsigned_abs` is total
where `abs` is partial, and it costs nothing: the comparison only ever wanted magnitudes, and a
magnitude has no business being signed. `SWIPE_MIN_PX` becomes a `u32` for the same reason —
the type now says what the number is.

The general lesson outlives this line. Everything arriving over the bridge is **untrusted input
from a document we did not write**, and `BridgeMsg::parse` is the boundary that has to say so.
It already refuses malformed payloads; it now also refuses to panic on well-formed hostile ones.

**The `dx === 0 && dy === 0` gate is the `key-listener.js` precedent, not a second threshold.**
`key-listener.js` filters to the two arrow keys in the frame before posting, because a bridge
message per keystroke is waste. The same argument applies to a bridge message per tap — and the
gate is written as "the pointer did not move" rather than as the 40px rule *specifically so the
threshold stays defined once, in Rust*. A `40` in the JS would be the same drift hazard the
`INJECTED_ASSETS` tests exist to catch.

### Scope note

**It does not make the swipe follow the finger.** There is no rubber-banding, no partial page
under the thumb, no animation — the page changes at `pointerup` or not at all. A live-tracking
transform means driving `--ook-page` fractionally from `pointermove`, which is a different and
much larger idea.

**It does not swipe on the host chrome.** The listener is inside the chapter iframe, so a swipe
that starts on the header or the nav row does nothing. On a phone the iframe is nearly the whole
screen, so this is currently invisible; it is worth knowing before someone reports it as
intermittent.

**`SWIPE_MIN_PX` is viewport-independent by choice.** 40 CSS pixels is a different fraction of an
iPhone than of an iPad, and the codebase elsewhere (`pagination.css`) deliberately derives its
geometry from one source rather than hard-coding numbers. Making the threshold a fraction of the
viewport would mean getting the viewport width over the wire, which is a new idea and not this
step's.

**It does not merge `key:` and `swipe:` into one `turn:` message.** Both decode to
`BridgeMsg::Turn`, and a third gesture would make that duplication worth collapsing — the
producers would post intent and one decoder would read it. Recorded for **Step 6**; doing it here
would mean rewriting the `key:` path this step did not touch.

**It does not fix the layout.** The page label this step reads its evidence from is still
off-screen at rest, which is **Step 5** and unchanged.

> **Amended after use — it turns the page on a text selection.** The tiebreak above reasons
> carefully about *ambiguous* gestures and then misses the one that is not ambiguous at all: a
> drag to select text is long, horizontal and single-pointer, so it satisfies every clause of
> `of_swipe` and turns the page. The argument was made entirely in the geometry of the gesture,
> and the thing that separates these two gestures is not geometric — it is what the document
> did in between. **Step 5c** carries the fix.

## Step 5 — Fit the device

> **Status:** done — committed in `5e0f82e` (123 tests green, clippy clean on both targets,
> driven on the iPhone 17 simulator). The desktop half is an eyeball and is the learner's to
> close; every `env()` is `0` there and the height chain is the same, so the expectation is no
> visible change.
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

The step [Finding B](#finding-b--the-top-safe-area-is-not-handled-and-step-2-was-wrong-about-it)
scheduled. At rest on a tablet the reader opened with **no way to turn a page**: the nav row sat
below the bottom edge of the display, and scrolling it into view pushed the close button up under
the status bar. You could have the top chrome or the bottom chrome, never both.

### The crux — the inset is applied to the *content*, not to the *viewport*

The instinct is to read this as "iOS forgot about the safe area". The opposite is true, and the
difference is the whole step.

A `WKWebView` whose frame is the whole screen defaults to
`contentInsetAdjustmentBehavior = .automatic`: UIKit sets the scroll view's `contentInset.top` to
the safe-area inset, so the document is **drawn 32pt lower**. What it does *not* do is shrink the
scroll view, so the layout viewport stays the full height of the display — and `100vh` still
means the full height of the display. A full-height screen is therefore laid out at exactly the
right size in exactly the wrong place, and the bottom `inset` points of it are pushed past the
bottom edge. Nothing is clipped, nothing errors; the page just becomes 52pt scrollable and half
its chrome is always somewhere else.

The web platform's answer is to take the offsetting away from UIKit and do it yourself. One meta
tag — `viewport-fit=cover` — tells WebKit to stop insetting the content and lay the document out
edge to edge, and *in exchange* it starts paying out the four `env(safe-area-inset-*)` values so
CSS can inset the parts that need it. **The two halves are a bargain, not two features**: without
the meta the `env()`s are silently `0`, and with the meta and no `env()` the content runs under
the Dynamic Island. That coupling — two files, no compiler between them — is what the first test
below exists to hold together.

### The check

Two host tests for the bargain and one for the unit it forces, in the
[`INJECTED_ASSETS`](../../../../src/web/assets.rs) idiom this repo already uses for cross-file
invariants that no compiler can see:

```rust
#[test]
fn the_safe_area_is_only_paid_out_to_a_viewport_that_covers_it() {
    assert!(VIEWPORT.contains("viewport-fit=cover"), ...);
    assert_eq!(MAIN_CSS_SOURCE.matches("env(safe-area-inset-").count(), 4, ...);
}

#[test]
fn the_replacement_viewport_restates_what_it_overrides() { ... }

#[test]
fn a_full_height_screen_is_measured_inside_the_inset_box() {
    assert!(!MAIN_CSS_SOURCE.contains("100vh"), ...);
    assert!(MAIN_CSS_SOURCE.contains("#main"), ...);
}
```

**The red, observed before any implementation:**

```
error[E0425]: cannot find value `VIEWPORT` in this scope
   --> src/main.rs:129:13   (and 3 more sites)
error: could not compile `ook-reader` (bin "ook-reader" test) due to 4 previous errors
```

The real check is the device, because this is a layout bug and `rect`s are the only honest
evidence about layout — the lesson Step 2 learned twice. On the iPhone 17 simulator
(`402 × 874`, top inset 62, home indicator 34), from `snapshot -i --json`:

| | before (Step 2a, iPad) | after (iPhone 17) |
|---|---|---|
| document root `y` | +32, and −20 when scrolled | **62** — the top inset, exactly |
| document root height | 1376 on a 1376 screen | **778** |
| root bottom vs. screen | 1408 vs. 1376 — 32pt off | **840** vs. 874 — the home indicator, exactly |
| scroll range | 52pt on a screen that should not scroll | **0** |
| nav row | `y=1380`, `press` refused it | `y=812`, **pressed → `Page 14 → 15`** |
| close button | `y=−8` when the nav row was visible | `y=74`, and it closes the book |

`62 + 778 + 34 = 874`. The arithmetic closing on the nose is the point: the document is now
exactly the usable rectangle, not the display.

### The code

`src/main.rs` declares the viewport and renders it into the head beside the stylesheets that
were already there:

```rust
const VIEWPORT: &str =
    "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover";

// in App's rsx!, above the existing document::Link's
document::Meta { name: "viewport", content: VIEWPORT }
```

`assets/main.css` spends what the meta buys, and carries the height down to the reader:

```css
body {
  margin: 0;
  box-sizing: border-box;
  padding: env(safe-area-inset-top) env(safe-area-inset-right)
    env(safe-area-inset-bottom) env(safe-area-inset-left);
}

html,
body,
#main {
  height: 100%;
}

.reader-root {
  display: flex;
  flex-direction: column;
  height: 100%;
}
```

and `src/ui/reader.rs` loses the inline layout it used to carry, and anchors its floating title:

```diff
-            style: "display: flex; flex-direction: column; height: 100vh; {settings().inline_styles()}",
+            style: "{settings().inline_styles()}",
...
-                style: "display: flex; justify-content: space-between;",
+                style: "display: flex; justify-content: space-between; position: relative;",
```

### Why it works

**The insets go on `body`, once, rather than on each screen.** There are two full-screen views
and both start at the top of the display; padding one of them would leave the other to rediscover
the same four values. `box-sizing: border-box` is what makes that padding free — without it the
padding adds to the height instead of eating into it, and the overflow comes straight back.

**`height: 100%` replaces `100vh` because `vh` cannot see the padding.** `100vh` is the display,
insets and all — the exact quantity that was 32pt too tall. A percentage height is resolved
against the *parent's content box*, so it is the inset box by construction and can never disagree
with the padding above it. The price is that percentages need an unbroken chain of resolved
heights, and Dioxus mounts the app into `<div id="main">`, which is not ours and had none. **The
first device run caught this and nothing else would have**: the tests were green, the app
launched, and the reader silently collapsed to `250pt` of content height with the nav row
floating at `y=284` in the middle of the screen. `#main` in the chain is the fix; the assertion
in the third test is so the next person to touch that selector list learns it from a failure
rather than from a snapshot.

**The floating title needed a positioned ancestor it never had.** `position: absolute; top: 0`
with no positioned ancestor resolves against the initial containing block — the *viewport*. That
was invisible while the viewport and the header row happened to share a top edge, and stopped
being invisible the moment `body` got 62pt of padding: the title would have gone back under the
Dynamic Island on its own. `position: relative` on the header row re-parents it to the box it was
always meant to be centred over. It changes nothing on desktop, where the two edges still
coincide. (It also does not disturb the two popovers in that row — `.dx-popover` is already
`position: relative`, so it remains their nearer positioned ancestor.)

**Re-declaring the viewport resets it, so the new one has to restate the old one.** WebKit
re-parses `<meta name="viewport">` on insertion and *replaces* the whole viewport description
rather than merging into it, which is why the fourth-from-last word matters: dropping
`user-scalable=no` here would silently re-enable pinch-zoom on a reader that paginates by
transform. That is the second test, and it is guarding a property of the platform, not of us.

**The runtime-injected meta was the risk, and the device settled it.** `document::Meta` is
`use_hook` + an `eval` that appends to `<head>` *after* the page has loaded — so this only works
if WebKit re-processes a viewport meta it did not parse at load time. It does; the measured `62`
and `778` are the proof, since the alternative reading of those numbers dies on the height.

### Scope note

**It does not do the thumb-sized chrome.** The other half of what the phase doc filed under Step
5: the nav row's `Prev`/`Next` are 20pt tall against a 44pt minimum touch target, and the library
grid is still the six-column desktop layout Step 2a found cramped into a corner. That is layout
*sizing*, a separate idea from the viewport, and it is the next step.

**It does not sweep the remaining `vh`.** `pagination.css` keeps its `100vh` — that one is inside
the chapter iframe, whose box is already the inset box, so `vh` there means what it should.
`toc.css`'s `max-height: 80vh` on the contents list is now measured against the whole display and
so can in principle run under the home indicator; on the iPhone 17 it does not (it ends ~30pt
short), and it was strictly worse before this step, so it is noted rather than changed.

**It does not verify the iPad.** The phase's headline evidence was an iPad Pro 13", and this step
was driven on the iPhone 17 — the harder device for the top inset and the only one `dx serve`
would install onto. The arithmetic is device-independent, but "iOS and iPadOS is one port and two
eyeball checks" is the phase's own rule, and the second check is outstanding.

**The desktop half is an eyeball, not a measurement.** `dx serve --platform desktop` builds and
launches; `agent-device`'s macOS runner would not attach and screen capture is unavailable to
this process, so "the window still looks right" is the learner's to confirm. Every `env()` is `0`
there and the height chain is the same, so the expectation is *no visible change at all*.
