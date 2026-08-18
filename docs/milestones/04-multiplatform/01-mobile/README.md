# Feature: Mobile (iOS / iPadOS)

[← Milestone 4: Multi-platform](../README.md)

**Outcome:** the reader you built on the desktop, running on an iPhone and an iPad, with a
book open and pages turning under your thumb. **Status:** 🚧 in progress.

## Why now

Milestone 4's deferral had a stated gate — *"deferred (until the desktop reader works)"* —
and the desktop reader works: Milestone 2 shipped the reader, and Milestone 3 has closed
Themes & Typography and ToC & Navigation. The gate is met, so this is not a queue-jump over
Milestone 3's remaining candidates so much as the deferral expiring.

It is also the dogfooding rule doing its job. [ADR-0002](../../../adr/0002-dogfood-driven-prioritization.md)
says the seed ordering is *not a contract* and the next-most-annoying real problem picks the
next slice — and the next-most-annoying real problem is that the book you want to read is on
a machine you do not read in bed with.

## The crux

**The UI ports for free. The assumptions do not.**

The free half is real and worth understanding rather than just enjoying. `dioxus::desktop`
and `dioxus::mobile` are not two renderers — they are *the same crate*, `dioxus-desktop`,
re-exported under two feature-gated names. Underneath, `wry` wraps `WKWebView` on iOS much
as it wraps `WKWebView` on macOS. So every `rsx!` block, every signal, the whole injected
CSS/JS reading system in `src/web/`, and even bundled `rusqlite` cross-compile untouched.
The evidence is in [Phase 9](phase-9-ios-simulator.md): the first iOS build got through the
entire dependency graph and failed on **five identifiers in four files**, all of them the
name `dioxus::desktop`.

What does *not* port is everything the app quietly assumes about being **a process on a
desktop, sharing a filesystem with its user**:

| the desktop assumption | what iOS actually is |
|---|---|
| point at a file the user already has | every app owns a sandbox; files arrive through a picker that hands back a *security-scoped* URL, not a path |
| a window whose size is worth remembering | one window, and it is the screen |
| pages turn on arrow keys and clicks | no keyboard, and a tap is not a click |
| the viewport is a rectangle | a rectangle minus a notch, minus a home indicator |

The compiler catches exactly the first category — the five names — and none of the second.
**So the phase is shaped around that split:** the compiler-guided part is one small step, and
everything after it is discovered by running the app and using it.

## Scope

**Simulator only.** Running on physical hardware needs a provisioning profile and a signing
identity, which is an Apple-account errand rather than a Rust or Dioxus lesson, and changes
nothing about the code. **Android is not in this feature's first phase** either: it is named
in the milestone's table alongside iOS, but it needs a second toolchain (SDK, NDK, CMake)
and its own crop of runtime surprises. One platform at a time.

## How this gets verified

Neither of the project's two checks reaches a phone. `cargo test` cannot see a running app, and
`dx serve` plus an eyeball assumes a pointer you are already holding and a failure you can
*see* — where the mobile failures are interaction failures: a tap that lands on nothing, a
swipe the JS never receives, a picker that returns a security-scoped URL instead of a path.

So this feature adds a third tool: [`agent-device`](https://github.com/callstack/agent-device),
which opens an app on a simulator, snapshots its accessibility tree, and presses elements by
reference. The mechanism carries a lesson beyond the tooling — **a UI driver reads the same
tree VoiceOver reads**, so an app that cannot be driven is an app that cannot be used with a
screen reader. Whether a WKWebView publishes a useful tree at all was the open question;
[Step 2a](phase-9-ios-simulator-steps.md#step-2a--drive-it-by-tap) answered **yes** — a healthy
tree, 64 nodes deep in the reader.

It earned its place immediately, and not by tapping. The tree carries **geometry**, and two
numbers out of it explained a layout bug that screenshots had been read as evidence *against*:
the reader is laid out 32pt below the status bar at a full `100vh`, so its nav bar hangs off
the bottom of the iPad and the book cannot be paged at rest. It also refused to press an
off-screen button rather than helpfully scrolling to it — which is the only reason the bug was
visible at all.

## Phases

| # | Phase | Outcome | Status |
|---|-------|---------|--------|
| 9 | [Run on the iOS simulator](phase-9-ios-simulator.md) | It builds, launches, opens a book, and turns pages by touch on iPhone and iPad | 🚧 |

## Reference

[`dx serve --platform ios`](https://dioxuslabs.com/learn/0.7/CLI/) ·
[wry on iOS](https://github.com/tauri-apps/wry) ·
[`env(safe-area-inset-*)`](https://developer.mozilla.org/en-US/docs/Web/CSS/env) ·
[UIDocumentPickerViewController](https://developer.apple.com/documentation/uikit/uidocumentpickerviewcontroller) ·
[`agent-device`](https://github.com/callstack/agent-device) ·
[its iOS Simulator skill](https://github.com/callstack/agent-device/blob/main/skills/ios-simulator/SKILL.md)
