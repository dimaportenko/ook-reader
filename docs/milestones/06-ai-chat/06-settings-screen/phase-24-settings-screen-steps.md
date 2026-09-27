# Phase 24 — Settings screen — build log

[← Phase 24](phase-24-settings-screen.md) · the phase doc holds the goal, decisions and the step
index; this file holds the test → code → why for each step, newest at the bottom.

## The crux

Settings is an overlay owned by the library, not a new route: a `Signal<bool>` and a
`position: fixed` layer, like the chat drawer, which means it pays for the safe area itself.
The chosen section is one `Signal<Option<SettingsSection>>`. A phone reads `None` as "show
the list", a wide window shows the sidebar next to the first section, and the difference
between the two is CSS alone.

## Step plan

1. ~~The gear and the empty screen~~ — an icon button in `LibraryBooks`, and a full-screen `SettingsScreen` with a header and a close button. **Done** — `7f44663`.
2. ~~Sections in a sidebar~~ — `SettingsSection`, sidebar buttons, and the chosen section's heading on the right. **Done** — `85b50a4`.
3. **Reader theme section.** The existing controls reused inside it.
4. **AI section: Gemini.** The key row and model picker move out of the reader popover.
5. **The phone layout.** List → section → back under a width breakpoint, checked on the iOS simulator.
6. **Review and refactor.**

**Why this order.** Each step ends with something to click:

- **Steps 1–2** are the empty frame and its navigation. Their failures (the safe area, a
  sidebar that doesn't switch) are cheapest to see while the screen has nothing in it.
- **Steps 3–4** fill it by *moving* existing controls, so they add no new logic.
- **Step 5** is last because it reshapes a layout that must already work wide. It's also
  the step that has to be checked on a phone, so the simulator run lands once, on the
  finished screen.

## Step 1 — The gear and the empty screen

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `7f44663` (207 tests green, 2 ignored; clippy clean; `dx serve` and iOS simulator checks confirmed by eye).

**What it is.** A gear button sits in the library's action bar next to *Edit*. Tapping it
covers the whole window with a *Settings* view: a header with the title and a close (✕)
button, and an empty body. Closing it shows the library exactly as it was.

### Runnable check first

**Kind:** `dx serve` eyeball, plus the iOS simulator for the notch. No `#[test]`, because
this step is pure markup and CSS with no logic to assert. `cargo clippy` stays clean.

1. The library's action bar shows a gear button labelled *Settings* (accessible name
   "Settings"), next to the edit (pencil) button.
2. Click it. A full-window view with the theme's background covers the library: *Settings*
   on the left of a header row and ✕ (accessible name "Close settings") on the right. No
   book covers show through.
3. Click ✕. The library is back and unchanged. If *Edit* mode was on before opening
   settings, it's off now, the same way the import button turns it off.
4. Switch the theme to dark in the reader popover, go back to the library and open
   settings. The screen is dark too, because it paints `--USER__backgroundColor`, not a
   hard-coded colour.
5. **iOS simulator** (a haiku subagent drives it, per memory): open settings on a notched
   iPhone. Using `agent-device snapshot -i --json`, the header's ✕ button `rect` sits
   **below** the status bar / notch and is pressable; tapping it closes the screen.

### Minimal implementation

**A new module `src/ui/settings_screen.rs`**, registered with `pub mod settings_screen;` in `src/ui/mod.rs`:

```rust
use dioxus::prelude::*;

use crate::ui::components::icon::{self, Icon};

#[css_module("/src/ui/settings_screen.css")]
struct Styles;

#[component]
pub(crate) fn SettingsScreen(mut open: Signal<bool>) -> Element {
    if !open() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "{Styles::settings_screen}",
            role: "dialog",
            aria_label: "Settings",
            header {
                class: "{Styles::settings_screen__header}",
                h1 { "Settings" }
                button {
                    class: "icon-button",
                    aria_label: "Close settings",
                    onclick: move |_| open.set(false),
                    Icon { icon: icon::CLOSE }
                }
            }
        }
    }
}
```

**`src/ui/settings_screen.css`:**

The class names use underscores, as `chat.css` does, because `css_module` turns each class
into a Rust identifier (`Styles::settings_screen`).

```css
.settings_screen {
  position: fixed;
  inset: 0;
  z-index: 3;
  display: flex;
  flex-direction: column;
  padding: env(safe-area-inset-top) env(safe-area-inset-right)
    env(safe-area-inset-bottom) env(safe-area-inset-left);
  background-color: var(--USER__backgroundColor);
  color: var(--USER__textColor);
}

.settings_screen__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
}
```

**`LibraryBooks` in `src/ui/library.rs`:**
- Add `let mut settings_open = use_signal(|| false);` next to `edit_mode`.
- In `library-books__actions`, after the edit button:

```rust
button {
    class: "icon-button",
    aria_label: "Settings",
    onclick: move |_| {
        edit_mode.set(false);
        settings_open.set(true);
    },
    Icon { icon: icon::SETTINGS }
}
```

- Render `SettingsScreen { open: settings_open }` as the last child of the root `div`.

### Why it works

- **`mut open: Signal<bool>` as a prop** is the drawer's pattern. A `Signal` is `Copy`, so
  the parent and the screen share one cell: the gear writes `true`, the ✕ writes `false`,
  and both components re-render because each one read it. The `mut` is only there because
  `.set()` takes `&mut self` on the local copy. It changes the shared value, not a copy of
  it.
- **The early `return rsx! {}`** means a closed screen renders nothing at all, so there's
  no hidden DOM and nothing for screen readers to find. The drawer instead stays mounted
  and slides, because it animates. Settings doesn't animate yet, so it has no reason to
  stay mounted.
- **`position: fixed; inset: 0`** sizes the layer to the viewport, not to its parent. That
  is why it covers the library no matter where it sits in the tree. But it also means
  `body`'s `env(safe-area-inset-*)` padding no longer applies to it, because the padding is
  on an ancestor it has escaped. So the layer adds its own, exactly as `.drawer` does.
  Leave that line out and the ✕ ends up under the notch. This is the Phase 9 bug again,
  and the simulator `rect` check is there to catch it.
- **`z-index: 3`** puts it above the drawer (`2`) and its backdrop. The library has no
  drawer, but the number states the layering on purpose instead of by accident.
- **Turning edit mode off on open** keeps one rule: any other library action ends
  editing. The import button already follows it.

### Scope note

- The body is empty. **Step 2** adds the sections.
- No Escape-to-close, no focus trap, no open/close animation. A dialog should get the first
  two eventually. Note them for the review step rather than adding them now.
- Settings opens only from the library. The reader keeps its popover (see the phase
  decisions).

### Review notes (from the `simplify` pass)

Four review angles ran over the diff (reuse, simplification, efficiency, altitude). Nothing
was changed. Each finding was skipped or deferred:

- **Reuse `Drawer` instead of a new overlay.** *Skipped.* `Drawer` is a side panel with
  swipe-to-close, and this screen is meant to be full-screen. Adding a "full" variant to
  `Drawer` is a possible later refactor, not this step.
- **The safe-area padding is now copied three times** (`body`, `.drawer`, `.settings_screen`).
  *Deferred to Step 6.* Pulling it into one shared class touches `drawer.css`, which is
  outside this step.
- **`settings_open` and `edit_mode` are two booleans that must not both be true.** *Deferred
  to Step 6.* One `enum` for the library's mode would make that impossible by construction.
  It becomes worth it if a third mode appears.
- **`edit_mode.set(false)` re-renders the library list even when edit mode is already off.**
  *Skipped.* It costs one diff per click, and it matches the import button, which does the
  same.
- **Early `return rsx! {}` versus wrapping the body in `if open() { … }`.** *Skipped.* Both
  are fine. The early return keeps the rendered markup unindented.


## Step 2 — Sections in a sidebar

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `85b50a4` (209 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

**What it is.** The empty body of the settings screen becomes two columns. On the left, a
sidebar lists *Reader theme* and *AI*. On the right, the chosen section's heading. Clicking a
sidebar entry switches the heading and highlights the entry. The sections are still empty.
Steps 3 and 4 fill them.

### Runnable check first

**Kind:** `#[test]` for the section list, plus a `dx serve` eyeball for the switching.

In `src/ui/settings_screen.rs`:

```rust
#[test]
fn the_sections_are_reader_theme_then_ai() {
    assert_eq!(
        SettingsSection::ALL.map(SettingsSection::label),
        ["Reader theme", "AI"]
    );
}

#[test]
fn settings_open_on_the_reader_theme() {
    assert_eq!(SettingsSection::default(), SettingsSection::ReaderTheme);
}
```

Red first: `cannot find type SettingsSection in this scope` (4 errors).

**The eyeball, under `dx serve`:**
1. Open settings. The sidebar shows *Reader theme* (bold, highlighted) and *AI*, and the
   right side is headed **Reader theme**.
2. Click *AI*. The heading becomes **AI** and the highlight moves.
3. Close and reopen settings. It is still on *AI*.
4. Tab to a sidebar entry. It shows a focus ring.

### Minimal implementation

**`SettingsSection`** in `settings_screen.rs`, following the `AiModel` / `Theme` idiom
(`Copy`, `ALL`, `label`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsSection {
    #[default]
    ReaderTheme,
    Ai,
}
```

**`SettingsScreen`:**
- Add `let mut section = use_signal(SettingsSection::default);` as its first line, *above*
  the early return.
- After the early return, add `let current = section();`.
- Under the header, add a body `div` holding two things:
  - a `nav` with one `button` per `SettingsSection::ALL`, each with
    `aria_current: if item == current { "page" }` and `onclick: move |_| section.set(item)`;
  - a `section` with `h2 { {current.label()} }`.

**CSS:**
- `.settings_screen__body` is a flex row with `flex: 1; min-height: 0`.
- `.settings_screen__sidebar` is a 14rem column.
- `.settings_screen__section_button` has the same look as a table-of-contents entry, with
  `[aria-current="page"]` bold and filled.
- `.settings_screen__section` is `flex: 1; overflow-y: auto`.

### Why it works

- **The hook goes above the early return.** Dioxus identifies a component's hooks by *call
  order*. If `use_signal` ran only when `open()` is true, the hook list would change length
  between renders. Keeping every hook above any `return` makes the order the same on every
  render.
- **The section survives close and reopen.** Closing only makes `SettingsScreen` *render
  nothing*. The component stays mounted in `LibraryBooks`, so its signal keeps its value.
- **`let current = section();` reads once per render.** The sidebar comparisons and the
  heading then can't disagree, and the component subscribes once. `toc.rs` does the same
  with its `current`.
- **`move |_| section.set(item)`** captures a *copy* of `item` for each button, because
  `SettingsSection` is `Copy`. Each closure owns its own variant, with no borrow of the loop
  variable.
- **`aria_current: if … { "page" }`** leaves the attribute out entirely when the condition is
  false. The highlight is keyed off that attribute, so screen readers and the CSS read the
  same state.
- **`min-height: 0` on the flex body** lets the section scroll inside the screen, instead of
  stretching the fixed layer past the viewport once Steps 3–4 add long content.

### Scope note

- The sections have headings only. **Step 3** fills *Reader theme*, and **Step 4** fills
  *AI*.
- The layout is desktop-only, with no breakpoint. **Step 5** makes `section` an
  `Option<SettingsSection>` and adds the phone list → section → back flow.

### Review notes (from the `simplify` pass)

- **Applied:** read `section()` once into `current` instead of three times per render.
- **Deferred to Step 6:** the sidebar button CSS repeats `toc.css`'s `.contents-popover__entry`
  (base, hover, focus ring, `aria-current`). Because CSS modules are scoped per file, sharing
  it means moving the rule into `assets/main.css` or into a small shared component.
