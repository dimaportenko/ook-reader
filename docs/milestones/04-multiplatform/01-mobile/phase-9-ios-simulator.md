# Phase 9 — Run on the iOS simulator

[← Feature: Mobile](README.md) · **Status:** 🚧 in progress ·
build log: [`phase-9-ios-simulator-steps.md`](phase-9-ios-simulator-steps.md)

## Goal

Open a book and read it on an iPhone and an iPad simulator, turning pages by touch. The
phase closes on a vertical slice, per [ADR-0001](../../../adr/0001-walking-skeleton-vertical-slices.md):
not "it compiles for iOS", but *a book, on a tablet, that you can read.*

## The crux

**The UI ports for free; the app's assumptions about its host do not.** The long version is
in the [feature README](README.md). The short version, and the reason this phase is shaped
the way it is:

`dioxus::desktop` and `dioxus::mobile` are the same crate — `dioxus-desktop` — re-exported
under two feature-gated names in `dioxus`'s `lib.rs`. `dx` builds iOS with
`--cfg feature="mobile"` and *without* `desktop`, so the code behind the name is present and
only the **path to it** disappears. That is why the first iOS build failed on five
identifiers and nothing else.

The consequence for planning: **the compiler can only find one class of problem here, and it
is the small one.** Everything expensive — how a file gets into the sandbox, what a tap does,
where the home indicator sits — is invisible until the app is running. So Step 1 is the whole
compiler-guided part of the phase, Step 2 is the first launch, and the steps after it are
written provisionally because Step 2 is expected to rewrite them.

## The evidence this phase starts from

Established before the phase was planned, so the steps are not guesses:

| | |
|---|---|
| Xcode | 26.6, with iPhone and iPad simulators available |
| `dx` | 0.7.9 — supports `--platform ios` and `--device` |
| Rust targets | `aarch64-apple-ios`, `aarch64-apple-ios-sim` installed |
| dependency graph | cross-compiles clean, **including bundled `rusqlite`** against the iPhoneSimulator SDK |
| `directories` v6 | supports `target_os = "ios"`, resolving into the app sandbox |
| `objc2-app-kit` usage | already `cfg(target_os = "macos")`-gated, correctly excluded |
| compile blockers | **5 errors in 4 files**, every one the name `dioxus::desktop` |

## Design decisions (recorded up front)

- **Simulator only — no device, no signing.** A provisioning profile is an Apple-account
  errand, not a Rust lesson, and nothing in the code changes for it. If the phase ends and
  you want it on your actual iPad, that is a separate, boring step.
- **iPadOS is not a second build.** The same `aarch64-apple-ios-sim` binary runs on both;
  `--device` picks the simulator. "iOS and iPadOS" is one port and two eyeball checks — but
  they are *two* checks, because the layout questions differ and an iPad is the device this
  reader is actually for.
- **Alias the renderer; do not union the features.** Making `mobile` also enable `desktop`
  would fix the build in one line of `Cargo.toml`, and it works today. It is rejected because
  it makes the feature flags lie about which platform is being built — see Step 1's *why*.
- **A readable book is the bar, not feature parity.** Desktop affordances that are meaningless
  or broken on a phone — window-frame memory, keyboard paging, hover states — get **noted in
  the build log, not fixed**. Chasing them would turn a port into a redesign.
- **The device is driven by a tool, not by a hand** *(added after Step 2)*. Verification here
  is `agent-device` — CLI or MCP — rather than `simctl` plus a person tapping the Simulator
  window. Two reasons, and neither is convenience: `simctl` cannot synthesize a touch at all,
  and a human eyeball cannot distinguish "the swipe handler is broken" from "you swiped in the
  wrong place." Installing it stays a **user-owned** step, per the tool's own instruction to
  agents. See Step 2a.
- **Steps 3–5 are provisional.** They are the problems the crux predicts, in the order that
  seems likely. Step 2 is a discovery step, and per [ADR-0002](../../../adr/0002-dogfood-driven-prioritization.md)
  what it finds gets to re-order what follows. Re-deriving them with `lbb:refine` after Step 2
  is the expected path, not a failure of planning.

## Planned steps

Detail for each lives in
[`phase-9-ios-simulator-steps.md`](phase-9-ios-simulator-steps.md).

- [x] **1. Name the renderer once** — replace the five `dioxus::desktop` references with one
      cfg-gated alias, so the crate builds for `aarch64-apple-ios-sim` *and* still builds for
      desktop. The check is both targets, because the failure mode is fixing one and breaking
      the other. Two-target build. — `1fc64eb`
- [x] **2. Launch it** — `dx serve --platform ios`, on an iPhone simulator and then an iPad.
      A **discovery step**: get to the library screen, then write down what actually happens.
      Eyeball, and a build-log entry that is mostly findings. *Findings landed; the gate is
      still open on two taps `simctl` cannot synthesize.* **No diff** — nothing needed
      changing. Launches on iPad Pro 13" (M5) and iPhone 17; sandbox persistence and
      `use_asset_handler` both confirmed working; top safe area already handled. Opening the
      reader and the import tap are **outstanding** — `simctl` cannot synthesize taps.
- [x] **2a. Drive it by tap** *(added by `lbb:refine` — the plan noticing it had no check)* —
      stand up [`agent-device`](https://github.com/callstack/agent-device) and spend it on the
      two observations Step 2 could not make. The phase's missing third verification tool, not
      a feature. **No `src/` diff.** The open question — does a **WKWebView** publish an
      addressable accessibility tree? — is **answered yes**: healthy tree, 64 nodes in the
      reader. It then rewrote most of the steps below. Findings: the reader opens and pages by
      tap; **the top safe area is *not* handled** (Step 2 was wrong) and the nav bar is
      off-screen at rest; **`<input type="file">` is inert on iOS**; four buttons have no
      accessible name.
- [x] **3. Get a book in** — ~~the import path under the sandbox~~ **a native import channel.**
      *Rewritten by Step 2a, then defused by reading tao.* The "wry puts the webview in no
      `UIViewController`" hypothesis is **wrong**: tao builds a `TaoUIViewController`, makes it
      the window's root, and exposes it as `WindowExtIOS::ui_view_controller()`. So the step is
      a `UIDocumentPickerViewController` presented from it, via `objc2` — written, 118 tests
      green, and driven end to end on the iPad. And the inert `<input type="file">` is
      explained too: `dioxus-desktop` routes file inputs to `rfd`, which has **no iOS
      backend**, so Dioxus was returning an empty file list from its own stub. — `badd372`
- [x] **4. Turn pages by touch** — the `Next`/`Prev` buttons already repaginated under tap
      (Step 2a), so the open half was **swipe**, and that is what landed: a new
      `swipe-listener.js` reports the pointer delta out of the frame and `Turn::of_swipe`
      decides in Rust, on the same seam `key-listener.js` and `Turn::of` already use. Driven on
      the iPhone 17 simulator — forward, back, and three gestures that correctly do nothing. — `bb18938`
- [x] **5. Fit the device** — **grown, and now load-bearing.** *Not* provisional any more:
      `src/ui/reader.rs:164`'s `height: 100vh` is 32pt taller than the usable viewport, because
      the document is offset below the status bar without the viewport being shrunk. At rest on
      an iPad the nav bar is off the bottom of the screen and the book cannot be paged. Needs
      `viewport-fit=cover` plus `env(safe-area-inset-*)`, or dropping `100vh`. *Split on
      implementation:* the viewport is one idea and the chrome's sizing is another, so this step
      is the viewport — `viewport-fit=cover`, `env()` insets on `body`, and `100vh` → `100%`
      down an unbroken chain. Driven on the iPhone 17: `62 + 778 + 34 = 874`, zero scroll range,
      both bars reachable at once. — `5e0f82e`
- [x] **5a. Give it an icon** *(provisional — found by running it)* — the springboard showed
      the default blank tile, and the gap is **a missing stage, not a missing setting**.
      Four builds established it: `[ios].icon` and `[ios].resources` both parse and are
      ignored, `dx bundle --package-types ios` is a passthrough, and the macOS pair on disk
      shows why — `dx build` installs no icon on *any* Apple platform, only `dx bundle` does,
      and it has no iOS path. `[ios.plist]` carries the `CFBundleIcons` declaration; the PNGs
      need a copy into the bundle root that a rebuild wipes, so it lives in a new
      `just install-ios` and `serve-ios` keeps hot reload and the blank tile. Verified on the
      iPhone 17 springboard and an iPad Pro 13" (M5). No `src/` diff; 127 tests unchanged. — `e2f0f94`
- [x] **5e. Install it on a real device** *(added at the user's request, mid-5a)* — **corrects
      5a's inference that dx has no signing pipeline.** It has one: `--device` selects the
      `aarch64-apple-ios` triple *and* implies `--codesign` (`request.rs:726`), after which dx
      picks a provisioning profile, embeds it as `embedded.mobileprovision`, and signs. Two
      things it gets wrong — it copies a wildcard profile's `application-identifier` (`TEAM.*`)
      verbatim where iOS wants the concrete bundle id, and it signs *before* 5a's icon copy,
      which the signature seals. Both are fixed by re-signing with entitlements read back out of
      the bundle, in a new `just install-device [query]` fronted by a `pick-device` recipe that
      matches a paired device by name, model or UDID and otherwise prompts. **Both devices now
      run the release build.** Getting there cost three rejections that look alike and are not:
      `10005` (Developer Mode off), `0xe8008015` (the signing *certificate* is not in the
      profile — the wildcard profile carried one that expired 2026-03-04), and `0xe8008012`
      (this *device* is not in the profile). Two findings worth keeping: App IDs are globally
      unique across teams, and this one is held by a **free** personal team that has no portal
      entry to release it from — so the profile expires every **seven days**; and the signing
      identity must be *derived from the embedded profile*, not taken as the first
      `Apple Development` line, because three teams are valid on this keychain and dx guesses
      wrong. No `src/` diff; 127 tests unchanged. — `e2f0f94`
- [x] **5c. Let a reader select text** *(bug, reported from use — regression on Step 4)* —
      **dragging to select text turns the page.** A selection drag and a page swipe are the
      same pointer event to `swipe-listener.js`: long, horizontal, one pointer id — so it
      clears `SWIPE_MIN_PX`, wins the horizontal-vs-vertical tiebreak, and posts a `swipe:`
      that `Turn::of_swipe` faithfully honours. The gesture is genuinely ambiguous at
      `pointerdown`; what disambiguates it is what the document did in between, so the fix
      is a `pointerup`-time question (did this gesture leave a non-collapsed
      `getSelection()`?) rather than a new threshold. Note it costs the reader the page they
      were on as well as the selection, which is why it reads as worse than a missed
      gesture. ~~Not yet reproduced under `agent-device`~~ — `longpress` **does** start a
      real selection, and the driven A/B on the fixed build separates a vetoed drag from
      two swipes of the same shape. The pre-fix repro was defeated by `simctl install`
      dropping the imported book, not by the gesture. — `be6cdcd`
- [x] **5d. Let the library survive a move** *(bug, diagnosed after 5c — scheduled at the
      user's request)* — **the imported book stops opening after a reinstall.** Nothing is
      deleted: `BookFiles::import` returned an absolute path and `books.path` stored it
      verbatim, and on iOS that path runs through the data container whose UUID iOS
      regenerates on every install while migrating the contents. The database is found by
      *recomputing* `Config::app_dir()` each launch and so is always found; the book file is
      found by a path *remembered* at import time, and only the remembered one can rot. The
      fix stores the file name alone and rejoins it with a fresh `books_dir` — the same trick
      the covers handler was already using, which is why covers kept rendering for books that
      would not open. Ships no migration for rows already written: the only such rows are the
      developer's own, repaired by hand, and there are no users yet. — `36052ba`
- [x] **5f. Paint the safe area** *(bug, reported from a screenshot)* — **the strip above the
      status bar and below the home indicator is white while the book is sepia.** Step 5 gave
      `body` the insets as padding, and an element's background fills its padding box — so the
      strip belongs to `body`, which owns no colour, while the theme sits two boxes in on
      `.reader-root`. No component can reach outside its own box, so the colour has to move to
      the one box that covers the insets: the canvas. `html` takes
      `var(--USER__backgroundColor)`, and the `:root` block `Settings::vars()` already builds is
      pushed into a `<style>` the app owns — a push and not `document::Style`, which inserts
      once and would freeze the canvas at the launch theme. — `da2cacd`
- [x] **5g. Fit the contents panel to the phone** *(bug, reported from use)* — **the contents
      popover hangs 50pt off the left edge** and clips every chapter title. Not a width
      mistake: `.dx-popover-content`'s base `max-width: calc(100% - 2rem)` is written for the
      `position: fixed` the base rule declares, and every `[data-side]` rule then flips it to
      `absolute`, so it silently measures the **40pt trigger** instead of the screen — while
      `min-width: 24rem` overrides it anyway. Aligned to the trigger's right edge at 334 on a
      402pt viewport, a 384pt panel starts at −50. Fixed by putting the panel back on the
      viewport below `40rem`: `position: fixed`, `env()` gutters on all four sides, the
      `min-width` floor reset, and a `75dvh` cap so the sheet stops clear of the chrome
      instead of burying its own trigger. Driven on the iPhone 17: `16 → 386`, top at 128,
      toggle and chapter-jump both good. 129 tests green. — `0a135fc`
- [ ] **6. Review and refactor** — the phase-closing pass. Carries four parked items: the
      `FRAME_AUTOSAVE_NAME` dead code under iOS (Step 1), **`Settings::inline_styles()`**, made
      redundant by Step 5f, **`.dx-popover-content`'s dead `max-width`** — authored for a
      `position: fixed` every `[data-side]` rule then discards, so it measures the trigger and
      no popover is viewport-bounded (Step 5g) — and **accessible names for four unnamed
      buttons** — the book cover, and the reader's close/contents/settings — found by Step 2a
      and not a mobile bug at all.

## Out of scope

**Android** (its own toolchain and its own surprises — the feature's second phase, if it
earns one), ~~**physical devices and code signing**~~ *(partly reclaimed by Step 5e — the
build and signing side is wired; obtaining a current provisioning profile is not)*, **web/WASM** (Milestone 4's other
feature, and a genuinely harder port: no custom protocol, no native SQLite), **`dx bundle`
for the App Store**, and any attempt at **feature parity with the desktop build**.
