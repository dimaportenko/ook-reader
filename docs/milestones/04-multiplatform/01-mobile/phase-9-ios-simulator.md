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
- [ ] **2. Launch it** — `dx serve --platform ios`, on an iPhone simulator and then an iPad.
      A **discovery step**: get to the library screen, then write down what actually happens.
      Eyeball, and a build-log entry that is mostly findings. *Findings landed; the gate is
      still open on two taps `simctl` cannot synthesize.* **No diff** — nothing needed
      changing. Launches on iPad Pro 13" (M5) and iPhone 17; sandbox persistence and
      `use_asset_handler` both confirmed working; top safe area already handled. Opening the
      reader and the import tap are **outstanding** — `simctl` cannot synthesize taps.
- [ ] **3. Get a book in** *(provisional)* — the import path under the sandbox.
      `ImportControl` uses `<input type="file">` and then `file.path()`; on iOS the picker
      returns a security-scoped URL and that path may not be one `fs::copy` can read. The
      likeliest step to split in two.
- [ ] **4. Turn pages by touch** *(provisional)* — tap zones and/or swipe. `pointer-listener.js`
      already exists, and `TODO.md` has wanted "change page on swipe" since before there was a
      phone to run it on.
- [ ] **5. Fit the device** *(provisional)* — safe-area insets for the notch and home
      indicator, and chrome sized for a thumb rather than a cursor.
- [ ] **5a. Give it an icon** *(provisional — found by running it)* — the springboard shows
      the default blank tile. `Dioxus.toml`'s `[bundle].icon` is a `dx bundle` key and dx
      0.7.9's iOS path never reads it, so this is a gap in the port rather than a setting
      that was missed. Eyeball on the home screen.
- [ ] **6. Review and refactor** — the phase-closing pass.

## Out of scope

**Android** (its own toolchain and its own surprises — the feature's second phase, if it
earns one), **physical devices and code signing**, **web/WASM** (Milestone 4's other
feature, and a genuinely harder port: no custom protocol, no native SQLite), **`dx bundle`
for the App Store**, and any attempt at **feature parity with the desktop build**.
