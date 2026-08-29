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
5. ~~**Fit the device**~~ — *split on implementation:* this half is the **viewport** —
   `viewport-fit=cover`, `env()` insets, `100vh` → `100%` down an unbroken chain.
   **Done** — `5e0f82e`.
5a. **Give it an icon** *(provisional)* — the springboard tile is blank. Found by Step 2.
   Installing an icon turns out to be a `dx bundle` stage that has no iOS path, so the PNGs
   go in by `just install-ios`. **Written, awaiting `lbb:commit`.**
5c. ~~**Let a reader select text**~~ *(bug, reported from use — regression on Step 4)* — a
   selection drag and a page swipe are the same pointer event; vetoed at `pointerup`.
   **Done** — `be6cdcd`.
5d. ~~**Let the library survive a move**~~ *(bug, scheduled at the user's request)* — store the
   file name, not the container path that iOS regenerates. **Done** — `36052ba`.
6. **Review and refactor** — the phase-closing pass. Carries `FRAME_AUTOSAVE_NAME` dead code
   under iOS (Step 1) and accessible names for four unnamed buttons (Step 2a).

Steps 3–5 were written from what the crux predicts, not from observation, and Step 2a duly
re-ordered them: 5a–5d all arrived from running the thing.

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

## Step 5c — Let a reader select text

> **Status:** done — committed in `be6cdcd` (126 tests green), clippy clean on desktop and
> clean on `aarch64-apple-ios-sim` apart from the `FRAME_AUTOSAVE_NAME` warning parked for Step
> 6. Driven on the iPhone 17 simulator.
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

Reported from use, and a regression on [Step 4](#step-4--turn-pages-by-touch): **dragging to
select text turns the page**, losing both the selection and the place the reader was in.

### The crux

Step 4's tiebreak reasoned entirely in the *geometry* of the gesture — long enough, more
horizontal than vertical, one pointer id — and a selection drag satisfies every clause of it.
That is not a threshold that was set too low; it is a question asked of the wrong thing. **A
page swipe and a selection drag are indistinguishable at `pointerdown` and indistinguishable
in their deltas.** What separates them is not the shape of the path but *what the document did
while the finger was down*: one left a selection behind and the other did not.

So the fix is a new **observation** taken at `pointerup`, not a new threshold. And because the
frame is the only place that can see a selection, the observation has to travel — which puts
the interesting decision back where the phase has been putting it all along.

### The check

```rust
#[test]
fn a_drag_that_leaves_a_selection_is_not_a_swipe() {
    assert!(crate::web::assets::INJECTED_ASSETS.contains("isCollapsed"));

    assert_eq!(Turn::of_swipe(-140, 6, true), None);
    assert_eq!(Turn::of_swipe(-140, 6, false), Some(Turn::Next));

    assert_eq!(BridgeMsg::parse("swipe:-140,6,true"), None);
}
```

Red before the change on its first line — nothing in the injected assets asked about a
selection — and red on the third for the same reason the wire had no field to carry the
answer.

### The code

`swipe-listener.js` takes the observation at `pointerup`, where the answer exists:

```js
  const selection = window.getSelection();
  const selected = !!selection && !selection.isCollapsed;
  window.parent.postMessage({ kind: "ook-swipe", dx, dy, selected }, "*");
```

`ook-events-listener.js` widens the wire to carry it, and `Turn::of_swipe` gains the veto:

```diff
-    dioxus.send("swipe:" + e.data.dx + "," + e.data.dy);
+    dioxus.send("swipe:" + e.data.dx + "," + e.data.dy + "," + e.data.selected);
```

```diff
-    fn of_swipe(dx: i32, dy: i32) -> Option<Turn> {
-        if dx.unsigned_abs() < SWIPE_MIN_PX || dx.unsigned_abs() <= dy.unsigned_abs() {
+    fn of_swipe(dx: i32, dy: i32, selected: bool) -> Option<Turn> {
+        if selected || dx.unsigned_abs() < SWIPE_MIN_PX || dx.unsigned_abs() <= dy.unsigned_abs() {
```

### Why it works

**The veto is in Rust, not in the listener, and that is the whole design question of this
step.** Suppressing the `postMessage` inside `swipe-listener.js` would fix the bug in one line
and no wire change — and it would make `of_swipe` a lie. Step 4's claim is that `of_swipe` is
*the* place that decides what a swipe means; a second, silent decider in JavaScript is how that
claim stops being true. The rule the codebase already follows is visible one branch up:
`key-listener.js` posts the DOM key *name* and `Turn::of` decides what an arrow means. JS
observes, Rust decides. A selection is an observation; vetoing a page turn is a decision. So
the observation crosses the wire and the decision stays put — and stays unit-testable, which
the JS half is not.

**`isCollapsed` is the right question, and "is there a selection object" is not.**
`getSelection()` returns a live `Selection` at essentially all times, with a collapsed range —
a caret — when nothing is selected. Testing for the object's existence would veto every swipe.
`isCollapsed` is false exactly when start and end differ, which is exactly "there is text
selected."

**`!!selection` is doing real work despite the `&&` after it.** `selection && !selection.isCollapsed`
evaluates to `null`, not `false`, when `getSelection()` returns null — and `null` crosses the
wire as the string `"null"`, which `bool::from_str` rejects, which makes the whole message
unparseable and silently kills paging. The coercion is what keeps the field a boolean.

**The bool crosses the wire as `"true"`/`"false"` by construction.** JS string-concatenates a
boolean to exactly those two spellings and `bool::from_str` accepts exactly those two — so
there is no hand-written mapping between the two languages to drift. Anything else is a parse
failure and no turn, which is the safe direction to fail.

**A stale selection blocks the next swipe, and that is correct rather than tolerated.** The
veto asks about the selection at the *end* of the gesture, so a swipe begun while an older
selection still stands is also refused. That is the desirable answer: on iOS a drag over live
selection UI is the reader adjusting their selection, not paging. Tapping elsewhere collapses
the caret first, and the swipe after that is unvetoed — measured below.

### Driven on the device

iPhone 17, `agent-device`, on the fixed build. The A/B is the point: three horizontal drags,
all well past `SWIPE_MIN_PX`, separated only by whether a selection stood at `pointerup`.

| gesture | selection at `pointerup` | page |
|---|---|---|
| `swipe 340 500 60 500` (−280px, fast) | none | 2 → **3** |
| `gesture pan 300 500 -240 0 1600` (−240px, slow) | none | 3 → **4** |
| `longpress 200 400 900` | — | 4 (Copy/Look Up callout up) |
| `gesture pan 200 400 160 0 900` (+160px) | **live** | 4 → **4** |

The last row is the bug. Same class of gesture as the first two, +160px against a 40px
threshold, and the page does not move.

### Scope note

**It does not reproduce the bug on the pre-fix build under `agent-device`.** The step planned
to, and the attempt is written up in *Left standing* — it was defeated by the simulator losing
the imported book on every reinstall, not by the gesture. The A/B above is the substitute
control, and the pre-fix behaviour is determined by inspection: the fourth row's `dx` is +160
with `dy` 0, which old `of_swipe` maps unconditionally to `Turn::Prev`.

**It does not distinguish "this gesture made the selection" from "a selection was already
there."** Recording the collapsed state at `pointerdown` as well would allow that, at the cost
of a second field and a subtler rule. The simpler question is the right one until a real
gesture argues otherwise — see the *why* above.

**It does not merge `key:` and `swipe:` into one `turn:` message.** Still Step 6's, and this
step made the case marginally stronger by giving `swipe:` a third positional field that `key:`
has no analogue for.

**It does not touch the desktop mouse path.** A mouse drag to select text on desktop reaches
the same listener and is now vetoed there too, which is the same fix and the same intent; it is
an eyeball the learner has not been asked for, since desktop paging is normally by arrow key.

### Left standing

**Four `settings::test` failures blocked the commit gate, and did *not* predate this step** —
an earlier reading of them said so and was wrong. Measured against both commits: `05ff6bb` is
green and `ed8ce0d`, the extraction of the layer into `user-layer.css`, is red. The formatter
that ran over the new file reflowed its selector lists across newlines and flipped
`[style*='…']` to double quotes, and all four tests located their rule with
`layer.split('\n')` — so they were pinning the layer's formatting while claiming to pin its
behaviour. Fixed as its own commit, `7d878e0`, ahead of this one: `rule_declaring` splits on
`}` and matches inside the block, `compact` strips whitespace and quotes before the
containment checks. Each of the four was mutated and watched go red, so none was softened to
reach green.

**~~`simctl install` over the app loses the imported EPUB.~~ It does not — the stored path
goes stale.** Diagnosed after the step closed, and the first reading above was wrong: nothing
is deleted. Every imported EPUB is still on disk. `BookFiles::import` returns an *absolute*
path and `books.path` stores it verbatim; on iOS that path runs through the app's data
container, whose UUID iOS regenerates on each install while migrating the contents to the new
directory. Measured: the data lives in container `131366EC` and the DB row names `85CEE5A3`,
with the file present and intact under the former.

The asymmetry that made it read as selective deletion is the tell — the **database** is opened
at a path recomputed every launch (`Config::app_dir()`, `src/main.rs:60`) and so is always
found, while the **book file** is opened at a path read back out of the DB and frozen at import
time. One is recalculated, the other is remembered, and only the remembered one can rot. It
never showed on desktop because macOS's `data_dir()` is stable for the life of the account;
iOS is the first platform where the app's own home directory moves.

Not a simulator artifact: reinstalling from Xcode, or shipping a user a new build, does the
same thing. The fix is to store the path **relative to `books_dir`** and rejoin it on read, the
way the DB is already located. `cover_path` carries the identical bug (`files.rs:33`).
`source_path` is a third case and a different question — it is an external path used for dedup
through a `UNIQUE` constraint, and the iOS picker hands back a temp-inbox URL. ~~**Not
scheduled**~~ — **scheduled and fixed as [Step 5d](#step-5d--let-the-library-survive-a-move)**,
at the user's request. It is a `03-reader-enhancements`-flavoured data bug rather than one of
Phase 9's layout items, and it is here because iOS is what exposed it.

## Step 5d — Let the library survive a move

> **Status:** done — committed in `36052ba` (127 tests green, clippy clean).
>
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

*Bug, diagnosed after Step 5c and scheduled at the user's request. Not a layout item, and not
strictly a Phase 9 item either — but iOS is the platform that exposed it, so it is logged
where it was found.*

### The crux — one path is recalculated, the other is remembered

The app knows two locations, and it learns them in two different ways.

The **database** is found by recomputing `Config::app_dir()` on every launch
(`src/main.rs:60`). Whatever `directories` says today is where the app looks today.

The **book file** is found by reading a string back out of that database — a string frozen at
import time, because `BookFiles::import` returned an absolute `PathBuf` and
`Library::add_from_path` stored it verbatim.

On every desktop platform those two agree forever, because `data_dir()` is stable for the life
of the account. On iOS they do not: the app's data container is a directory whose name is a
UUID, and **iOS regenerates that UUID on each install**, migrating the contents into the new
one. So the bytes survive and the remembered path does not.

Measured on the simulator before the fix: exactly one container on disk,
`131366EC-B726-4E3E-A1F9-B40FDCFFC44E`, holding all three imported EPUBs intact — while the
`books` rows named `85CEE5A3-…`, a container that no longer existed. Nothing had been deleted.
The library was pointing at an address the reader had moved out of.

That framing is the whole step. The bug is not "the file went missing" and the fix is not
"handle a missing file"; the bug is **that a path was stored at all**, and the fix is to store
the one part of it that cannot rot — the file name — and rejoin it with a freshly computed
`books_dir` on every read. The same trick the database itself already uses, and the same trick
`use_register_covers_handler` already used for cover images, which is why covers kept rendering
on a build where books would not open.

### The check

One test in `src/library/mod.rs`'s test module, simulating the container rename with
`fs::rename` on a tempdir.

**The bug, reproduced.** `books_reopen_after_the_app_directory_moves` imports into
`container-a`, moves the whole directory to `container-b`, reopens the database from its new
home, and asks the row to open its book. Watched fail before any implementation existed, with
precisely the simulator's error:

```
Archive(UnreadableArchive { source: Os { code: 2, kind: NotFound,
  message: "No such file or directory" },
  path: Some("…/container-a/books/644d5dde-0c71-4e62-a618-fb544d515456.epub") })
```

That is the prize of this step independent of the fix: a bug that previously needed a
simulator, a reinstall and a hand-driven import to observe is now a unit test that runs in
10ms.

### The code

`BookFiles` stops dealing in paths and starts dealing in **names**, with one method that knows
where the directory is:

```rust
pub(crate) fn path_of(&self, name: &str) -> PathBuf {
    self.dir.join(name)
}

pub(crate) fn import(&self, source: &Path) -> Result<String, std::io::Error> {
    let name = format!("{}.epub", Uuid::new_v4());

    if let Err(error) = fs::copy(source, self.path_of(&name)) {
        self.remove(&name);
        return Err(error);
    }

    Ok(name)
}
```

`write_cover` and `remove` take names too, so every filesystem touch in the module routes
through `path_of`. `Library` re-exports it as `book_path`, which is what the UI calls.

`Book`'s fields are renamed to say what they now hold — `path` → `file_name`,
`cover_path` → `cover_name` — so the compiler finds every consumer rather than letting a
`String` quietly change meaning. `Book::cover_name()`, the helper that existed only to strip a
directory off the stored cover path for the `/covers/` URL, **dissolves**: the field is already
the name.

### Why it works

**The rename is doing real work.** A `String` field that changes meaning while keeping its name
is the exact shape that re-rots six months later. `path` → `file_name` turned a silent semantic
change into ~20 compiler errors, every one of them a place that had to be re-read and decided.

**Covers survived the move all along, and that was the clue.** `use_register_covers_handler`
serves `/covers/<name>` by joining the name onto a freshly computed `books_dir`. It never read
the stored path, so it never rotted — the app rendered a perfect grid of book covers it could
not open. The fix generalises what the covers handler was already doing.

### Scope note

**It ships no migration for rows already on disk.** An earlier draft of this step carried a
`Db::shorten_paths_to_names` that ran from `migrate()` and rewrote every stored `path` and
`cover_path` down to its file name. It was cut at commit time: the only databases holding
absolute paths are the developer's own, they were repaired by hand, and the app has no users
yet. Writing migration code for zero rows is code that can only rot before it can ever run.

Worth knowing if that decision is ever revisited — a stale row is *invisible on desktop*.
`PathBuf::join` discards its receiver when the argument is absolute, so for an un-migrated row
`book_path()` returns the old absolute path unchanged, which on a desktop platform is still a
valid path to a real file. Nothing looks wrong. The breakage is iOS-only, because that is the
only platform where the old container is genuinely gone. Any future need for this migration
will therefore surface as a device bug, not a failing test.

**It does not touch `source_path`.** That column is a genuinely different thing: an *external*
path, to a file the app does not own, used only as the `UNIQUE` key that makes reimport
idempotent. It cannot become a name — two books named `book.epub` in different folders are two
books. On iOS it is worse than stale, it is meaningless: the document picker hands back a
temp-inbox URL that is deleted shortly after. Reimport dedup on iOS therefore does not really
work yet, and that is a separate step with a separate question ("what identifies a book?"), not
a path-joining fix.

**It does not rename the `path` and `cover_path` columns.** The struct fields say `file_name`
and `cover_name`; the schema still says `path` and `cover_path`, so the SQL in `books.rs` is
now the one place where the two vocabularies meet. `ALTER TABLE … RENAME COLUMN` would close
the gap and is cheap on SQLite 3.25+; it was left out because it is schema churn for
readability, not for correctness.

**It does not verify on the simulator.** The failure is now a unit test, and the fix is a
join — there is nothing about it that a device could show that the test does not. What the
device would still answer is a question this step deliberately dropped: whether books imported
by an *older* build come back. They do not, and are not meant to — see the migration note
above. A fresh import on the simulator exercises everything this step actually changed.

**It does not add a repair path for a genuinely missing file.** A row whose managed copy really
was deleted still fails at `Epub::open` with a raw error in the status line. That was true
before and is unchanged; `reimport_repairs_a_missing_managed_copy` covers the recovery route
that exists.

## Step 5a — Give it an icon

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

*Provisional when planned, found by running it: the springboard showed the default blank tile.
The first step in this phase whose diff contains no Rust at all.*

### The crux — an icon is a declaration, not a file

On iOS an app icon is not something the app *has*. It is two things that have to agree:

1. **`CFBundleIcons` in `Info.plist`**, naming icons by *base name* — `AppIcon60x60`, never a
   path — with iOS appending `@2x` / `@3x` and `.png` itself;
2. **the PNGs at the root of the `.app`**, because that base name is resolved against the
   bundle root and nowhere else.

Neither half existed, and this step turned out to be about *why* the second half is hard.

### The check — the red, and what four builds established

Measured before touching anything:

```sh
dx build --platform ios
ls target/dx/ook-reader/debug/ios/OokReader.app     # assets  Info.plist  ook-reader
plutil -p .../Info.plist | grep -i icon             # exit 1 — no CFBundleIcon* key at all
```

Three entries, no icon key. Note also what is *not* in `assets/`: `dx` copies the files the
`asset!()` macro linked, not the `assets/` directory, so `assets/icons/icon.png` — the 1024²
source this project has had since Milestone 1 — had never shipped in an iOS build.

Then four builds, each answering one question:

| tried | result |
|---|---|
| `[ios] icon = ["assets/icons/icon.png"]` | **parsed and ignored.** The key is real in dx 0.7.9's schema — *"Icons for the app. Overrides `bundle.icon` for iOS builds"* — and the build path never reads it. No warning. |
| `[ios] resources = [...]` | **parsed and ignored**, same way. Nothing reached the bundle. |
| `[ios.plist]` with nested `CFBundleIcons` tables | **works.** Rendered into the plist as a correct nested dict, `~ipad` variant included. |
| `dx bundle --platform ios --package-types ios` | **a passthrough.** Same three-entry `.app`, no icon. |

The last row is what turned a guess into a finding. Compare the two macOS outputs already on
disk:

```
target/dx/.../debug/macos/OokReader.app/Contents/Resources/    assets            # dx build
target/dx/.../bundle/macos/macos/OokReader.app/Contents/Resources/
                                          OokReader.icns  assets                 # dx bundle
```

**Installing an icon is a *bundler* feature, not a *builder* feature — on every Apple
platform.** `dx build` has never installed one, not even on macOS; `dx bundle` does it for
macOS through tauri-bundler, and dx 0.7.9 has **no iOS icon path at either stage**. So the
phase doc's "gap in the port" was right, and bigger than it looked: it is not a missing key,
it is a missing stage.

One more measurement decided the shape of the fix. Copying the PNGs in by hand and rebuilding:

```
--- after rebuild ---
assets  Info.plist  ook-reader
```

**A rebuild wipes the bundle root.** So the copy is not a one-time repair; it has to run after
every build, which is exactly what makes it a recipe rather than a note.

**The gate is an eyeball, and it is a real one** — a plist key is not an icon until the
springboard draws it. Driven with `agent-device open com.apple.springboard` to reach page 2 on
the iPhone 17, and `simctl install` on an iPad Pro 13" (M5). Both render the icon; the two
`rfd` sample apps sitting beside it on the same page still show the blank default tile, which
is the before-picture preserved by accident.

### The code

Four prescaled PNGs committed under `assets/icons/ios/`, from the 1024² source (`sips -z`),
named for the base names iOS will ask for:

```
AppIcon60x60@2x.png      120   iPhone
AppIcon60x60@3x.png      180   iPhone Plus / Pro Max
AppIcon76x76@2x.png      152   iPad
AppIcon83.5x83.5@2x.png  167   iPad Pro
```

The declaration, in `Dioxus.toml`, through the one hook that works:

```toml
[ios.plist]
CFBundleIcons = { CFBundlePrimaryIcon = { CFBundleIconFiles = ["AppIcon60x60"] } }
"CFBundleIcons~ipad" = { CFBundlePrimaryIcon = { CFBundleIconFiles = ["AppIcon60x60", "AppIcon76x76", "AppIcon83.5x83.5"] } }
```

And the stage dx does not have, as a `justfile` recipe:

```make
ios_app := "target/dx/ook-reader/debug/ios/OokReader.app"

install-ios: boot-ios
    dx build --platform ios
    cp assets/icons/ios/*.png {{ios_app}}/
    xcrun simctl install booted {{ios_app}}
```

### Why it works

**Base names, not paths — which is why `assets/icons/icon.png` can never simply be named.**
`CFBundleIconFiles` entries are stems. iOS takes `AppIcon60x60`, appends the scale suffix for
the device it is on and `.png`, and looks in the bundle root. There is no syntax for a
subdirectory, so an icon living under `assets/` is unreachable no matter what the plist says.
That single fact is what forces the copy step to exist.

**`~ipad` is a plist convention, not something invented here.** iOS resolves a key suffixed
`~ipad` in preference to the bare key when running on an iPad, for *any* key — the same
mechanism as the `UISupportedInterfaceOrientations~ipad` dx already writes into this plist.
That is how one bundle carries two icon sets, and it is why the iPad check was worth running
separately: it exercises a key the iPhone never reads.

**The 1024² image alone is not enough on this route.** A single marketing-size icon works when
`actool` compiles an `Assets.xcassets` into an `Assets.car` and synthesises the plist keys —
the modern path, and it needs a build stage this project does not have. Without it the
springboard does no downscaling: it asks for a specific pixel size and draws nothing if the
file is absent. Hence four files, not one.

**No alpha channel, checked rather than assumed.** iOS rejects an icon with transparency;
`sips -g hasAlpha assets/icons/icon.png` says `no`, and `sips -z` preserves that.

### The trade this step makes — and it is worse than it first looked

The plan was that `serve-ios` keeps hot reload and keeps the blank tile, while `install-ios`
is the slower path with a correct springboard. Measured, that framing was too kind.
`dx serve --platform ios` does not merely *skip* the icon — it **undoes** it:

```
=== build output after dx serve ===        assets  Info.plist  ook-reader
=== installed container after dx serve === assets  Info.plist  ook-reader
```

Serve wipes the bundle root, rebuilds without the PNGs, and reinstalls over the good install.
The springboard follows immediately — no icon-cache lag, the tile is blank again on the next
screenshot. Which also explains a wiped build output seen earlier and briefly blamed on
nothing: that is just what a build does.

So **`install-ios` is a one-shot demo path, not a fix.** The icon is correct until the next
`dx serve`, and `dx serve` is the command the daily loop is made of. There is no seam to fix
this in dx 0.7.9: serve builds and installs internally, and the only directory it copies is
the linked-`asset!()` set, whose hashed names can never satisfy a `CFBundleIconFiles` stem.

**And it cannot be worked around from this side.** Asked directly — can `dx serve` be made to
show the icon? — the answer is no, and the reason is that **an iOS icon is baked at install
time**. `installd` renders it into its own cache when the app is installed; the container is
not re-read afterwards. Three measurements:

| | result |
|---|---|
| install *with* the PNGs in the bundle | icon renders |
| install *without* them — what `dx serve` does | blank tile |
| copy the PNGs into the *installed container*, then restart SpringBoard | **still blank** |

So the files must be inside the `.app` at the moment `simctl install` runs, and `dx serve`
owns that moment end to end. Everything that could open a seam was checked and is absent:
`[ios].icon` and `[ios].resources` parse and are ignored, dx 0.7.9 has no pre/post-build hook
(the only `hooks` strings in the binary belong to cargo-generate and NSIS), and
`dx bundle --package-types ios` is a passthrough. Post-install patching was the last candidate
and the third row rules it out.

`install-ios` is therefore the whole of what is available today, and the blank tile under
`serve-ios` is not a trade that was chosen so much as one that cannot be refused.

### Scope note

- **No `#[test]`, and that is not a lapse.** There is no Rust in this step. The suite stays at
  **127 green**, clippy clean, and the check is a build product plus two eyeballs — the same
  shape as Step 2.
- **Only the springboard sizes are declared.** The 29pt (Settings) and 40pt (Spotlight) icons
  are not generated, so those surfaces may still draw blank. They were not verified either way;
  declaring sizes without checking them is how this kind of config rots.
- **`[ios].icon` and `[ios].resources` were removed after testing.** Both parse and do nothing.
  Config that looks load-bearing and is not is worse than no config — the finding belongs in
  this log, not in `Dioxus.toml`.
- **Not the launch screen.** The plist names `UILaunchStoryboardName = LaunchScreen` and no such
  storyboard exists in the bundle; the white flash on launch is that, and it is separate.
- **Not Android, macOS or Windows.** `[bundle].icon` still feeds `dx bundle` for the desktop
  targets, unchanged.
- **Not icon design.** `assets/icons/icon.png` remains the single source; the four files are
  derived from it and regenerating them is four `sips` lines.
- **Not the four unnamed buttons.** Accessible names stay parked for Step 6.

---

> **Status:** done — committed in `e2f0f94` (127 tests green; no `src/` diff, so the
> count is unchanged by design).

## Step 5e — Install a signed release build on a real device

> **Added at the user's request**, mid-5a. It corrects a claim this log made earlier — that a
> device install would need a signing pipeline the project does not have — and replaces it with
> a measured one.

### The correction

Step 5a's write-up said dx's iOS bundler "only zips a `.app` it refuses to look inside," and
inferred from that that signing was ours to build. **That was wrong, and it was wrong because
it was inferred rather than run.** `dx build --help` has three flags the earlier reading missed:

| flag | what it does |
|---|---|
| `--device [<DEVICE>]` | targets `aarch64-apple-ios` instead of `…-ios-sim` |
| `--codesign` | runs `codesign` over the finished bundle |
| `--apple-entitlements` / `--apple-team-id` | override the auto-provisioned pair |

And `request.rs:726` makes the second implied by the first — `args.codesign || device.is_some()
|| args.apple_entitlements.is_some()` — so **`--device` alone is the whole device path.** dx
scans `~/Library/Developer/Xcode/UserData/Provisioning Profiles` (falling back to the pre-Xcode-16
`~/Library/MobileDevice/…`), picks the best match for the bundle id, ranks *exact app ID >
more provisioned devices > newer file*, copies the winner in as `embedded.mobileprovision`, and
signs. One command produces a signed, profile-bearing, arm64 `.app`.

### The two things it still gets wrong

**1. The entitlements are copied verbatim from a wildcard profile.** `auto_provision_entitlements`
lifts `application-identifier` straight out of the profile, so a `TEAM.*` profile yields a signed
`application-identifier` of literally `Y5Q5H3AG9D.*`. iOS requires the concrete
`Y5Q5H3AG9D.com.dimaportenko.ook-reader`. `installd` rejects the install with `0xe8008015`.

**2. It is the wrong side of the icon copy.** Same shape as 5a's finding, one stage later: dx
signs the bundle, and a signature *seals its contents*, so the four PNGs added afterwards
invalidate `_CodeSignature` unless the bundle is signed again.

Both are fixed by the same move — **re-sign after copying**, with corrected entitlements. The
entitlements are not hardcoded: they are read back out of the bundle dx just signed
(`codesign -d --entitlements - --xml`), and the team prefix out of the profile dx just embedded
(`ApplicationIdentifierPrefix`), so the recipe follows whatever profile dx chose.

### The code

Two recipes, split so the *choosing* is separable from the *installing*. `pick-device` resolves
a device to a UDID and prints nothing else on stdout; `install-device` consumes it.

```make
pick-device query="":
    #!/usr/bin/env bash
    set -euo pipefail
    json=$(mktemp)
    script=$(mktemp)
    trap 'rm -f "$json" "$script"' EXIT
    xcrun devicectl list devices --json-output "$json" > /dev/null
    cat > "$script" <<'PY'
    ... filter by query, else print a numbered menu and read a choice ...
    PY
    QUERY="{{query}}" python3 "$script" "$json"

install-device query="":
    #!/usr/bin/env bash
    set -euo pipefail
    udid=$(just pick-device "{{query}}")
    dx build --platform ios --release --device "$udid"
    cp assets/icons/ios/*.png "{{ios_device_app}}/"
    identity=$(security find-identity -v -p codesigning | awk '/Apple Development:/ {print $2; exit}')
    team=$(security cms -D -i "{{ios_device_app}}/embedded.mobileprovision" \
      | plutil -extract ApplicationIdentifierPrefix.0 raw -)
    entitlements=$(mktemp)
    codesign -d --entitlements - --xml "{{ios_device_app}}" > "$entitlements"
    plutil -replace application-identifier -string "$team.{{bundle_id}}" "$entitlements"
    plutil -replace keychain-access-groups -json "[\"$team.{{bundle_id}}\"]" "$entitlements"
    codesign --force --entitlements "$entitlements" --sign "$identity" "{{ios_device_app}}"
    xcrun devicectl device install app --device "$udid" "{{ios_device_app}}"
```

`just install-device` with no argument lists the paired devices and prompts; `just install-device
ipad` matches on name, model or UDID and skips the prompt when the match is unique; a single
paired device is taken without asking. **The menu goes to stderr and only the UDID goes to
stdout**, which is what lets `install-device` capture the answer with `$(...)` while the prompt
still reaches the terminal.

One bash trap worth remembering, because the first attempt hit it: the picker script is written
to a temp file rather than piped in with `python3 - <<'PY'`. Feeding a script on **stdin spends
stdin**, so the interactive `readline()` gets EOF and the prompt silently reads an empty string.
Reading from `/dev/tty` is the usual fix and is in there as the first choice, but it fails
outright where there is no controlling terminal — so the script is a file and stdin stays free
for both.

The picker resolves to a **UDID** and not a name because the name is a trap: `devicectl` prints
`Dmytro’s iPhone` with a **typographic** apostrophe, and passing the ASCII one straight back to
`--device` gets `CoreDeviceError 1000 — device not found`. Substring matching sidesteps it — you
type `iphone`, never the apostrophe — and the UDID it resolves to is unambiguous and is what
`devicectl device install` wants anyway. Signing by the identity's **SHA-1** rather than its
common name is the same instinct applied to the keychain, which holds *four* certificates named
`Apple Development: Dima Portenko (BWUDYFW6A8)`, three of them expired.

### Verified — it installs

Both devices run the release build. Everything below is measured on this machine:

| | |
|---|---|
| binary architecture | `arm64`, non-fat — the device triple, not the simulator's |
| `embedded.mobileprovision` | present in the `.app` |
| `Info.plist` | carries both `CFBundleIcons` and `CFBundleIcons~ipad` |
| the four PNGs | present in the bundle root after the copy |
| `codesign --verify` | *valid on disk*, *satisfies its Designated Requirement* |
| install — iPhone 13 Pro | **succeeded** |
| install — iPad Pro 11" | **succeeded** |

Getting there cost three rejections, and **the useful part of this step is that they are three
different failures wearing nearly the same clothes.** `installd` checks cheapest-first, so they
arrive in a fixed order and each one only becomes visible once the previous is cleared:

| code | what it actually means | fix |
|---|---|---|
| `10005` | Developer Mode is disabled | Settings → Privacy & Security, then restart |
| `0xe8008015` | the signing **certificate** is not in the profile | refresh the profile |
| `0xe8008012` | **this device** is not in the profile | register the device |

`0xe8008015` is the one worth slowing down on, because it has three causes that the message
does not distinguish:

1. the device is not in the profile — *ruled out here;* both UDIDs were listed
2. the signed `application-identifier` does not match the bundle id — *real, and the recipe
   fixes it* by rewriting the wildcard dx copies in
3. the signing certificate is not among the profile's `DeveloperCertificates` — **this was it**

Cause 3 held because the only profile matching this bundle id was the wildcard `Y5Q5H3AG9D.*`,
in date to 2026-11-06 but carrying exactly one certificate — serial `08576C51…`, **expired
2026-03-04**. The certificate that signed the app had been renewed since. A profile does not
go stale when it expires; it goes stale when the certificate inside it does, and nothing in
the error says so.

### What minting the profile taught

Xcode mints one from a throwaway project — `xcodegen` a single-target app with the right bundle
id, then `xcodebuild -allowProvisioningUpdates`. Two findings came out of doing it:

**App IDs are globally unique across teams, and a free team can hold one hostage.** The first
attempt targeted the paid personal team `F2MP7G7FM5` and was refused:
*"the app identifier … cannot be registered to your development team because it is not
available."* `com.dimaportenko.ook-reader` was already registered to `HNVBRBU7PH`, the **free**
personal team, from some earlier experiment. Free-team App IDs are created implicitly by Xcode
and are not listed in the developer portal — Certificates, Identifiers & Profiles is a paid
membership feature and the portal's team switcher does not show Personal Teams — so there is no
UI to release it. The identifier is effectively stuck on the free team. The escape hatch, if the
free team's terms ever bite, is a bundle id that is still unclaimed.

**Xcode registers the device it is pointed at, and only that one.** Building for
`generic/platform=iOS` produced a profile containing the iPhone alone, which is why the iPad
then failed `0xe8008012` rather than succeeding. Re-running against
`-destination 'platform=iOS,id=<ipad-udid>'` produced a second profile with both. dx picks
between them correctly without help — its ranking is *exact app ID > more provisioned devices >
newer file*, and the two-device profile wins on the middle term.

### The identity is derived, not guessed

The first draft of the recipe took the first `Apple Development` line out of
`security find-identity`, which is exactly what dx does. Both are wrong for the same reason:
this keychain holds valid identities on **three** teams, and the one that must sign is whichever
team owns the profile dx just embedded — `HNVBRBU7PH` here, which is neither first nor the one
dx chose.

So the recipe reads the certificates *out of the embedded profile*, SHA-1 fingerprints each,
and picks the first that `security find-identity -v` reports as a valid keychain identity. That
fingerprint is exactly the hash `codesign --sign` wants, so no name lookup is needed. It follows
whatever dx selects, across every team on the machine, and when nothing matches it says so
instead of deferring the truth to a hex code from `installd`.

### The seven-day clock

`HNVBRBU7PH` is a **free** personal team, so the profile expires **2026-09-05 — seven days**.
Both installs stop launching then and need `just install-device <device>` again. That is a
property of the team, not of this recipe; a paid team issues a year. It is recorded here because
the failure it produces later — an app that launches for a week and then refuses — looks nothing
like a provisioning problem from the outside.

### Scope note

- **No `src/` diff and no new test.** The suite stays at **127 green**, clippy clean. There is
  no Rust in this step; the only honest check is the artifact and the install, and the tables
  above are it.
- **Not `.ipa`, not TestFlight, not the App Store.** `dx bundle --package-types ipa` exists and
  is untouched. Submission additionally needs `CFBundleIconName` and a compiled asset catalog
  (`actool` → `Assets.car`) that the loose `CFBundleIconFiles` route does not satisfy — the one
  place 5a's icon mechanism genuinely does not carry.
- **The icon on hardware is now checkable but unchecked.** It renders on both simulators, and
  the release build carries the plist keys and the PNGs onto both devices — but nobody has
  reported back on the springboard, so 5a's claim stays reasoned rather than measured.
- **The throwaway Xcode project is not in the repo.** It lives in scratch, exists only to make
  Apple mint a profile, and re-creating it is a `project.yml` and one `xcodebuild`. Checking it
  in would imply the project builds through Xcode, which it does not.
- **`serve-ios` is unchanged and still iconless.** 5a's wall is about `dx serve` owning the
  build→install moment, and nothing here opens it.

> **Status:** done — committed in `e2f0f94` (127 tests green; no `src/` diff, so the
> count is unchanged by design).

---

## Step 5f — Paint the safe area

> **Added at the user's request**, from a screenshot: the strip above the status bar and
> below the home indicator is white while the book is sepia.
>
> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

### The crux

Step 5 gave the app its box: `body` pads itself by `env(safe-area-inset-*)`, so nothing the
app draws lands under the status bar or the home indicator. That is the *layout* half of the
safe area, and it is correct. The half it left behind is **who paints the strip that padding
carves out** — and the answer CSS gives is *the element that owns the padding*, because an
element's background fills its padding box, not just its content box.

`body` owns the padding and owns no colour. The theme's colour lives two boxes further in,
on `.reader-root`, put there by `Settings::inline_styles()`. So the strip falls through
`body` (transparent) to `html` (transparent) to the canvas, whose default is white — and a
sepia book gets white bars top and bottom.

That framing decides the fix. The strip is *outside* every box the component tree can reach,
so no component can colour it; the colour has to be given to a box that covers the insets,
and the only such box is the canvas. Which means the theme has to stop being a fact about
one component and become **a fact about the document** — declared at `:root`, where `html`
can read it.

### The check

Three assertions, one per seam that can silently come apart:

```rust
#[test]
fn the_theme_paints_the_safe_area_strip() {
    let canvas = MAIN_CSS_SOURCE
        .split('}')
        .find(|rule| rule.contains("var(--USER__backgroundColor)"))
        .expect("nothing outside the reader's own box carries the theme");

    assert!(
        canvas
            .split_once('{')
            .is_some_and(|(selector, _)| selector.trim() == "html"),
        "the theme is painted inside the inset box, so the strip the padding \
         leaves keeps the canvas default",
    );

    assert!(
        settings::Settings::default().vars().starts_with(":root {"),
        "the canvas can only read a variable declared at the document root",
    );

    assert!(
        ROOT_THEME_JS.contains("getElementById"),
        "a push per settings change appends a new <style> every time unless it \
         finds the one it wrote last",
    );
}
```

Red before the fix, on the first assertion: *nothing outside the reader's own box carries the
theme.* The selector assertion is the load-bearing one — a rule that paints
`--USER__backgroundColor` on `.reader-root` would satisfy a naive "does the CSS mention the
variable" check while leaving the bug exactly where it was.

### The code

`assets/main.css` — the canvas takes the theme:

```css
html {
  /* prevent scroll bounce behavior */
  overscroll-behavior: none;
  background-color: var(--USER__backgroundColor);
  color: var(--USER__textColor);
}
```

`src/web/assets/root-theme.js` — a `<style>` the app owns and rewrites:

```js
const css = await dioxus.recv();

let style = document.getElementById("ook-theme");

if (!style) {
  style = document.createElement("style");
  style.id = "ook-theme";
  document.head.append(style);
}

style.textContent = css;
```

`src/main.rs` — pushed from `App`, not from `Reader`:

```rust
use_effect(move || {
    let push = document::eval(ROOT_THEME_JS);
    _ = push.send(settings().vars());
});
```

### Why a push and not a `<style>` in the tree

Dioxus has `document::Style`, which renders a `<style>` into the head and would take
`settings().vars()` as its child in one line. **It cannot be used here**, and the reason is
in its own doc comment: *"Any updates to the props after the first render will not be
reflected in the head."* It is a `use_hook` — it inserts once. The theme picker changes the
theme *live*, so a head component would paint the canvas with whatever theme was loaded at
launch and then never move again. The bug would look fixed until you touched the picker.

So the theme reaches the host document the same way it already reaches the frame: pushed
through `document::eval` from an effect that reads `settings()`, which is what subscribes it.

**What is pushed is the whole `:root` block, not a list of properties.** `theme-listener.js`
inside the frame loops over `css_vars()` calling `setProperty`/`removeProperty`, and it has
to, because it is patching a root it does not own. The host owns its `<style>` outright, so
replacing `textContent` gets removal for free: `Settings::vars()` already filters empty
values out of the block, so switching the font back to Publisher simply stops declaring
`--USER__fontFamily`. One assignment, no loop, and it reuses a function that was already
written and already tested.

### Why `html` and not `body`

Either works for the strip — `body`'s background would paint its own padding box, and with
`html` transparent it would propagate to the canvas as well. `html` is chosen because that
propagation rule is genuinely surprising CSS that depends on `html` having no background of
its own, and because the canvas is also what shows during rubber-band overscroll. Naming the
box you mean is cheaper to read than relying on a rule that silently stops applying the day
someone gives `html` a colour.

### Verified — on the iPhone 17 simulator

| | |
|---|---|
| reader, sepia | both strips sepia, continuous with the page — the reported bug is gone |
| library, sepia | same, and the library is themed for the first time |
| live switch to night, driven through the picker | canvas and both strips go dark **without a reload** |
| back to sepia | returns |
| tests | **128 green** (was 127), clippy clean |

### Scope note

- **The theme now reaches the library screen too.** That is a consequence, not creep: the
  colour had to move to the document to cover the strip, and the document is what the library
  renders into. `color` travels with it deliberately — painting the canvas dark under Night
  while leaving the library's text at the UA default black is a worse bug than the one being
  fixed.
- **`Settings::inline_styles()` is now redundant** — `.reader-root` re-declares at component
  scope every variable `:root` already carries, and re-applies a background and colour it
  would otherwise inherit. Removing it touches four tests in `settings/mod.rs` that are not
  about this bug, so it is left for **Step 6**'s refactor pass.
- **Not the chrome's sizing.** Step 5b still owns the 20pt `Prev`/`Next` targets and the
  six-column library grid.

> **Status:** done — committed in `da2cacd` (128 tests green).

---

## Step 5g — Fit the contents panel to the phone

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

**Bug, reported from use.** On the iPhone the contents popover hangs off the left edge of
the screen: every chapter title is clipped mid-word, and the top-level entries lose their
numerals entirely.

### The crux — a `max-width` that measures the wrong box

The popover is not too wide by accident. Three CSS facts compose into the bug, and only the
third is in this repo's own stylesheet:

1. `.dx-popover-content`'s base rule is `position: fixed` with
   `max-width: calc(100% - 2rem)`. Those two belong together: for a fixed box the containing
   block *is* the viewport, so `100%` means "the screen".
2. Every `[data-side="…"]` rule then overrides it to `position: absolute`. The containing
   block becomes the nearest positioned ancestor — `.dx-popover`, which is
   `position: relative; display: inline-block`, i.e. **the 40pt icon button**. The base
   `max-width` silently starts meaning "100% of the trigger minus 2rem", which is negative,
   and stops bounding anything.
3. `.contents-popover__content` declares `min-width: 24rem`. Per CSS §10.4 `min-width` beats
   `max-width` unconditionally, so the panel takes 384px whatever the viewport says.

`ContentAlign::End` then pins the panel's **right** edge to the trigger's right edge. Driven
measurement (`agent-device snapshot -i --json`, iPhone 17, 402pt viewport):

| box | x | width | right |
|---|---|---|---|
| viewport | 0 | 402 | 402 |
| contents trigger | 290 | 44 | **334** |
| contents panel | **−50** *(reported clamped to 0)* | 384 | 334 |
| a chapter row | −30 | 344 | 314 |

`334 − 384 = −50`: fifty points of every row are off the left edge. The accessibility tree
clamps a negative `x` to `0` while keeping `width`, which is why the panel *reports*
`x=0, width=384` — the screenshot, not the number, is what shows the clipping.

So the panel is not merely too wide; it is **measured against the wrong box**. No width alone
can fix it — the trigger sits 68pt in from the right edge, so even a panel exactly `100vw`
wide would start at −68. The panel has to stop being anchored to the trigger.

### The check — driven, not asserted

There is no Rust here to unit-test; the whole change is CSS, and CSS's effect on a phone is
what `agent-device` exists for in this repo. The **red** is the table above: right edge at
the trigger's 334, left edge at −50.

The regression guard that *is* assertable is the new cross-file coupling — the ToC now splits
its layout at the same viewport width the shared popover stylesheet already widens at, and it
must reset the `min-width` floor when it does:

```rust
#[test]
fn the_panel_becomes_a_sheet_below_the_width_the_popover_widens_at() {
    assert!(POPOVER_CSS.contains(&format!("@media (width >= {NARROW_MAX})")));

    let sheet = TOC_CSS
        .split_once(&format!("@media (width < {NARROW_MAX})"))
        .expect("the contents panel has a narrow-viewport rule")
        .1;

    assert!(
        sheet.contains("min-width: 0"),
        "the 24rem floor outgrows the viewport the sheet is pinned to",
    );
}
```

It fails on the pre-fix tree at the `expect`: *the contents panel has a narrow-viewport rule*.

### The code

```css
@media (width < 40rem) {
  .contents-popover__content[data-side][data-align][data-state] {
    position: fixed;
    top: auto;
    right: calc(1rem + env(safe-area-inset-right));
    bottom: calc(1rem + env(safe-area-inset-bottom));
    left: calc(1rem + env(safe-area-inset-left));
    min-width: 0;
  }

  .contents-popover__list {
    max-height: 75dvh;
  }
}
```

### Why it works

**`position: fixed` is the whole fix.** It restores the containing block the base stylesheet
originally assumed: the viewport. Once the panel is measured against the screen, `left` and
`right` are gutters off the screen's edges rather than offsets from a 40pt button, and
overflow becomes impossible by construction — the trigger's position stops mattering.

**Why three attribute selectors.** The declarations being overridden live at
`.dx-popover-content[data-side="bottom"]` (`position`, `top`) — specificity (0,2,0) — and
`…[data-side="bottom"][data-align="end"]` (`left`, `right`) — (0,3,0). A single class is
(0,1,0) and loses. `[data-side][data-align][data-state]` brings the ToC's rule to (0,4,0),
which wins outright. A tie would *not* have been safe: `dx` copies `toc.css` before
`popover/style.css`, so on equal weight the shared file's anchoring would win on source
order. All three attributes are always present — the primitive emits `data-state` on every
popover for its open/closed animation.

**`min-width: 0` is not tidying.** With both `left` and `right` set, the used width is
`402 − 16 − 16 = 370`. A 384px floor would override that and put the panel back over the
right edge. Resetting it is what lets the gutters actually hold.

**`env(safe-area-inset-*)` because fixed boxes escape `body`.** Step 5 paid the insets as
padding on `body`; a `position: fixed` box is laid out against the viewport and never sees
them. The sheet has to re-inset itself, which is why the same four `env()`s reappear here.
(`main.rs`'s test counts them in `main.css` only, so it stays green.)

**`75dvh` protects the chrome.** The first driven build was correct horizontally and wrong
vertically: with `top: auto` the panel grew to the content's full height and landed at
`y = 84`, on top of the header — covering its own trigger, so the toggle could not be
toggled back, with only a 22pt strip left to dismiss by. Capping the scroller drops the
panel's top to `y = 128`, clear of the title's `126`. `dvh` and not `vh` for Step 5's
reason: on iOS `vh` measures the *large* viewport, so the existing `80vh` was really 733pt on
an 874pt screen, not 699.

### Forks taken

- **Scoped to the contents panel, not to every popover.** The obvious deeper fix is to make
  `.dx-popover-content` itself viewport-bounded in the shared stylesheet. Rejected on
  measurement: the settings popover renders at `x=246, width=140, right=386` — a dropdown
  that fits with room to spare. Turning it into a full-width sheet would redesign a component
  nobody reported broken. The general defect (a `max-width` that means "the trigger") is real
  and is noted for Step 6.
- **A bottom sheet, not a top one.** Anchoring to the top would need a `top` equal to the
  chrome's height, and the chrome's height is an inline style in `reader.rs` over a
  two-line title — a number CSS cannot name without a shared custom property. `bottom` needs
  only the home-indicator inset, which `env()` already gives.
- **Overriding the anchor rules rather than dropping `position: relative` from
  `.dx-popover`.** Making the trigger un-positioned would hand the panel to the next
  positioned ancestor — the reader's header row, which happens to be full-width and would
  have given a *correct* `top: 100%` for free. Rejected because it silently depends on an
  inline style in a different file: change `reader.rs`'s header and the popover moves.

### Verified — on the iPhone 17 simulator

| | | |
|---|---|---|
| | before | after |
| panel x → right | −50 → 334 | **16 → 386** |
| panel y → bottom | 130 → 870 | **128 → 825** |
| chapter row x → right | −30 → 314 | 36 → 366 |
| trigger covered | — | no (panel top 128 > title bottom 126) |
| toggle closed by re-pressing the trigger | — | yes |
| picking a chapter | — | jumps to *I. A Scandal in Bohemia* and closes |
| tests | 128 | **129 green**, clippy clean |

### Scope note

- **Not the shared popover stylesheet.** `.dx-popover-content`'s `max-width: calc(100% - 2rem)`
  is dead as written — it is authored for the `position: fixed` the base rule declares and
  every side rule then discards. Repairing it is Step 6 material, and it is now the third item
  on that list.
- **Not the `80vh` on the wide-screen path.** `.contents-popover__list` still caps at `80vh`
  above 40rem, and `vh` is the wrong unit there too. Changing it moves the desktop panel, which
  is not this bug.
- **Not landscape.** The `env()` left/right insets are declared, but the phase has only ever
  been driven in portrait; a rotated phone is unverified.
- **Not the iPad.** At 1024pt it is above the breakpoint and keeps the dropdown unchanged —
  reasoned, not driven.

> **Status:** done — committed in `0a135fc` (129 tests green).

## Step 6 — Review and refactor

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

The phase-closing pass. Four items were parked across Steps 1, 2a, 5f and 5g, and the
thing they have in common is that **each one is a leftover of a decision the phase later
reversed** — not sloppiness, but the sediment a port leaves when it changes its mind about
where something lives. That is the frame worth holding: the punch-list is short because the
reversals were few, and every item is "the old owner never let go", not "nobody wrote this
properly".

### The check

Three of the four items have no assertion available, and it is worth being exact about
why rather than calling the whole step eyeball-only:

| item | check | why not a `#[test]` |
|---|---|---|
| `FRAME_AUTOSAVE_NAME` | `cargo clippy --target aarch64-apple-ios-sim --no-default-features --features mobile` — **zero warnings** | the defect *is* a warning; the compiler is the assertion, and it only fires on a target `cargo test` never builds |
| `Settings::inline_styles()` | it compiles with no callers, and `the_theme_paints_the_safe_area_strip` still passes | a test that greps for a function you just deleted asserts nothing |
| `.dx-popover-content`'s `max-width` | **new test**, below | the source text is the behavior — the repo's existing CSS idiom |
| four unnamed buttons | a driven accessibility snapshot | `rsx!` has no unit-test seam, and the accessibility tree is the *only* place the defect was ever visible |

The one real test, in `src/ui/toc.rs`'s existing test module (which already reads
`POPOVER_CSS`):

```rust
#[test]
fn the_popover_is_bounded_by_the_viewport_and_not_by_its_trigger() {
    let base = POPOVER_CSS
        .split_once(".dx-popover-content {")
        .expect("the rule every popover starts from")
        .1
        .split_once('}')
        .expect("an unclosed rule")
        .0;

    assert!(
        !base.contains("max-width: calc(100%"),
        "every [data-side] rule re-positions the panel to absolute, where a \
         percentage max-width resolves against the 40px trigger",
    );
    assert!(
        base.contains("dvw"),
        "only a viewport unit means the same thing under both position \
         schemes the rules disagree about",
    );
}
```

Watched red before the fix: *"every [data-side] rule re-positions the panel to absolute,
where a percentage max-width resolves against the 40px trigger"*.

### The punch-list

**1. `FRAME_AUTOSAVE_NAME` moved inside the function that uses it** (`src/window.rs`).
It was a `pub(crate) const` at module scope, read from exactly one place — the
`#[cfg(target_os = "macos")]` body of `remember_frame`. On every other target that body
does not exist, so the const has no readers and `#[warn(dead_code)]` fires; it was the
only warning in the iOS build. The fix is not a `#[cfg]` on the const (which would be a
second copy of the same condition, free to drift from the first) but moving the const
*into* the `cfg`-gated body. A `const` inside a function is a perfectly ordinary item in
Rust — same compile-time evaluation, same `&'static str` — it just inherits the gate from
its enclosing scope instead of restating it.

**2. `Settings::inline_styles()` deleted** (`src/settings/mod.rs`, `src/ui/reader.rs`).
It built `<declarations> background-color: … color: …` for a `style="…"` attribute on
`.reader-root`. Step 5f moved the whole theme onto the canvas: `root-theme.js` pushes
`:root { … }` into a `<style>` the app owns, and `main.css` gives `html` the background
and text colour. From that moment the inline copy declared the same custom properties, on
a descendant of the element that already had them, to the same values — a second channel
carrying identical cargo. Removing it also drops a `format!` over the full `css_vars()`
list from the reader's render body, which runs on every page turn.

**3. `.dx-popover-content`'s `max-width` made viewport-relative**
(`src/ui/components/popover/style.css`). `calc(100% - 2rem)` was authored against the base
rule's `position: fixed`, where `100%` is the viewport. Every `[data-side]` rule then sets
`position: absolute`, and a percentage `max-width` on an absolutely-positioned box resolves
against its containing block — `.dx-popover`, the `position: relative` wrapper around a
**40pt** trigger. So the declaration was computing `calc(40px - 2rem)` ≈ 8px. It never
showed, because `min-width` beats `max-width` in the CSS box algorithm and both panels
declare one (140px settings, 24rem contents). `100dvw` is the fix rather than deletion:
deleting leaves the panels with no upper bound at all, which is strictly worse than a wrong
one, and a viewport unit means the same thing under `fixed` and `absolute` alike — which is
the property the rule needed and never had.

**4. Accessible names for four buttons** (`library.rs`, `reader.rs`, `toc.rs`,
`settings.rs`). Found by Step 2a and, as that step noted, not a mobile bug — the book
cover and the reader's close/contents/settings buttons contain an `<img>` or an inline
SVG icon and no text, so they had no accessible name on any platform. `aria_label` on
each. The cover interpolates the title (`"Open {book.title}"`), because a shelf of
buttons all named "Open" is the same defect one rename later.

### Forks taken

- **The const moved into the function, not `#[cfg]`-gated in place.** Gating it would work
  and reads as the smaller diff, but it duplicates the `target_os = "macos"` condition —
  and a duplicated condition is one that can drift. Scope is the mechanism that cannot
  drift.
- **`100dvw`, not deleting the declaration.** Step 5g explicitly parked "repair it" here
  and the temptation is to call a wrong bound a dead bound. But the two are not the same:
  the settings popover on a phone has `min-width: 140px` and no other ceiling, so removing
  the `max-width` would let long content push it off the right edge. Bounding to the
  viewport is what the author meant, written so that it survives the positioning switch.
  Note this is the pass's **one behaviour change** — the rest are pure refactors.
- **Named the contents trigger `"Table of contents"`, the same string the `nav` landmark
  inside the panel already carries.** Considered hoisting to a shared const; rejected,
  because they name different things — a control and a landmark — and a const would imply
  they must stay equal. A screen reader announcing "Table of contents, button" and then
  "Table of contents, navigation" is correct, not duplicated.

### Verified — on the iPhone 17 simulator

| | before | after |
|---|---|---|
| accessible names | cover / close / contents / settings all anonymous | **"Open The Adventures of Sherlock Holmes"**, **"Close book"**, **"Table of contents"**, **"Reading settings"** |
| reader theme after dropping the inline styles | sepia | **sepia** — unchanged; `html` carries it |
| settings panel geometry | `x=246, w=140` | `x=246, w=140` — the `min-width` was always the binding constraint |
| iOS clippy warnings | 1 (`FRAME_AUTOSAVE_NAME` never used) | **0** |
| tests | 133 | **134 green**, clippy clean on both targets |

### Scope note

- **Not the commented-out `/* min-width: 200px; */`** left in `popover/style.css`. It is
  dead scaffolding and a candidate for a sweep, but it is the user's own comment and this
  step does not delete those.
- **Not `settings/mod.rs`'s `no_stack_quotes_a_family_with_double_quotes` comment**, which
  justifies itself by naming `inline_styles()`. The test is still live — the stack also
  travels through `bootstrap_js`, into a JS string literal that a `"` would close early —
  but its stated reason is now the wrong one. Worth a one-line rewrite by hand.
- **Not the chapter title's missing `text-overflow`.** It still clips mid-word under the
  chrome buttons on a phone (visible as *"The Adventures of Sherlock Holr"*). Pre-existing,
  cosmetic, and it belongs to whichever step next touches the reader's header — a candidate
  for Phase 10 Step 3, which re-lays that row out anyway.
- **Not landscape, not the iPad, not Android.** Same standing gaps the phase has carried
  throughout.
