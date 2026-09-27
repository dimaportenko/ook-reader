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
3. ~~Reader theme section~~ — the existing controls reused inside it. **Done** — `10e1751`.
4. ~~AI section: Gemini~~ — the key row and model picker move out of the reader popover. **Done** — `fcfe38b`.
5. ~~The phone layout~~ — list → section → back under a width breakpoint, plus a styling pass. **Done** — `ae4f1ab`.
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

## Step 3 — Reader theme section

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `10e1751` (209 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

**What it is.** The *Reader theme* section of the settings screen shows the same six
controls as the reader's popover: line height, font size, page margins, line length, font
and theme. They are the same components, not copies, so there is one set of controls in two
places. Changing one in either place changes the setting everywhere.

### Runnable check first

**Kind:** `dx serve` eyeball. There's no `#[test]`, because the step adds no logic. It
reuses components that already read and write `Signal<Settings>`. The existing suite stays
green (209 passed, 2 ignored) and `cargo clippy` stays clean.

1. Open settings from the library. The *Reader theme* section shows, below its heading and
   stacked in a column: the ↕ line-height buttons, A-/A+, the ↔ margin buttons, the ≡ line
   length buttons, a font `select` and a theme `select`.
2. Set the theme to `dark`. The settings screen itself turns dark straight away, because it
   paints `--USER__backgroundColor` and `App` pushes the new variables on every settings
   change.
3. Click A+ twice, then close settings and open a book. The reader's text is larger. Open
   the reader popover: it shows the same percentage and the dark theme.
4. Change something in the reader popover, go back to the library and reopen settings. The
   section shows the new value.
5. Click *AI* in the sidebar. The controls disappear and only the heading remains, because
   Step 4 fills it.
6. The reader popover looks exactly as it did before, with the AI rows still at the bottom.

### Minimal implementation

**`src/ui/settings.rs`.** Group the six controls in one component and use it in the
popover:

```rust
#[component]
pub(crate) fn ReaderThemeControls() -> Element {
    rsx! {
        LineHeightControl {}
        FontSizeControl {}
        PageMarginsControl {}
        MaxLineLengthControl {}
        FontFamilyPicker {}
        ThemePicker {}
    }
}
```

In `SettingsPopover`, the six lines become `ReaderThemeControls {}`, followed by the
`AiModelPicker {}` and `ApiKeyControl {}` it already had.

**`src/ui/settings_screen.rs`.** Import `crate::ui::settings::ReaderThemeControls`. Under
the section's `h2`:

```rust
div {
    class: "{Styles::settings_screen__controls}",
    match current {
        SettingsSection::ReaderTheme => rsx! { ReaderThemeControls {} },
        SettingsSection::Ai => rsx! {},
    }
}
```

**`src/ui/settings_screen.css`:**

```css
.settings_screen__controls {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
}
```

### Why it works

- **Context, not props.** Every control calls `use_context::<Signal<Settings>>()`, and
  `App` provides that signal above both the reader and the library. A context lookup walks
  *up* the tree from wherever the component is mounted. So the same component works inside
  the popover and inside the settings screen with no new wiring, and both places write to
  the same signal.
- **One write, three effects.** A click calls `settings.write()`. That re-renders every
  component that read the signal (both copies, if both were mounted), and it wakes `App`'s
  effects, which save the settings to the database and push the CSS variables. That is why
  the settings screen changes colour when you pick a theme.
- **`ReaderThemeControls` returns a fragment.** It has no wrapper element, so each parent
  keeps its own layout. The popover's inline flex column is unchanged, and the screen uses
  its own class. Neither place has to put up with the other's box.
- **`match` rather than `if`.** A `match` must cover every `SettingsSection`. The `Ai => rsx! {}`
  arm is a visible placeholder today. If a third section is ever added, the compiler lists
  this spot as unfinished.
- **`align-items: flex-start`** keeps each control at its natural width. The default,
  `stretch`, would widen the `select`s across the whole section.

### Scope note

- The *AI* section is still empty, and the popover still has the AI rows. **Step 4** moves
  them.
- The controls have no labels. The pickers show slugs (`dark`, `serif`), and the buttons
  show only glyphs. That was fine in a popover next to the page, but in a titled settings
  section it reads bare. Candidate for **Step 6** or a later polish pass.
- There's no phone layout yet. That is **Step 5**.

### Review notes (from the `simplify` pass)

Four review angles ran (reuse, simplification, efficiency, altitude). Nothing was changed.

- **Reuse:** `.settings_screen__controls` repeats the popover's inline
  `display: flex; flex-direction: column; gap: 0.5rem`. *Deferred to Step 6.* Sharing it
  means moving the column into `ReaderThemeControls` itself or into a shared class. Both
  change the popover, which is outside this step, and the two differ by `align-items`.
- **Simplification, efficiency, altitude:** no findings. The empty `Ai` arm was weighed and
  kept, for the reason above.

## Step 4 — AI section: Gemini

> **Written by:** `lbb:next-implement` — implementation and tests written by the agent,
> reviewed by hand.

> **Status:** done — committed in `fcfe38b` (209 tests green, 2 ignored; clippy clean; `dx serve` checks confirmed by eye).

**What it is.** The *AI* section gets its first provider block: a **Gemini** heading, the API
key row, then the model picker. Both rows move out of the reader's popover. The popover now
holds only the reading controls, and the AI settings live in one place.

### Runnable check first

**Kind:** `dx serve` eyeball. There's no `#[test]`, because the step moves two existing
components and adds no logic. `KeyStatus` keeps its tests. The suite stays green
(209 passed, 2 ignored) and `cargo clippy` stays clean.

1. Open a book and open the reader popover. It shows the six reading controls and nothing
   else: no *AI model* select and no *Gemini API key* row.
2. Go back to the library, open settings and click *AI*. It shows a **Gemini** heading, then
   the key row with its status (*Key set*, *Not set*, or *Secret store unavailable*), then
   the *AI model* select.
3. With a key saved, click *Forget*. The status turns *Not set*. Paste the key and click
   *Save*. The status turns *Key set* and the field clears.
4. Pick a different model, close settings, open a book and ask the chat something. The reply
   comes back, from the model you picked.
5. Reopen settings. It is still on *AI*, and the picker shows the model you chose.

### Minimal implementation

**`src/ui/settings.rs`.** One block per provider, in the order the phase doc sets out:
heading, key row, models.

```rust
#[component]
pub(crate) fn GeminiSettings() -> Element {
    rsx! {
        h3 { "Gemini" }
        ApiKeyControl {}
        AiModelPicker {}
    }
}
```

- Delete `AiModelPicker {}` and `ApiKeyControl {}` from `SettingsPopover`.
- `AiModelPicker` and `ApiKeyControl` lose their `pub(crate)`. They are only called from
  `GeminiSettings` now.

**`src/ui/settings_screen.rs`.** Import `settings::{GeminiSettings, ReaderThemeControls}`,
and fill the arm Step 3 left empty:

```rust
SettingsSection::Ai => rsx! { GeminiSettings {} },
```

### Why it works

- **Context again.** `ApiKeyControl` reads three contexts: the `Option<Rc<dyn SecretStore>>`,
  the `Signal<Option<Gemini>>` provider and `Signal<Settings>`. `App` provides all three
  above the library as well as the reader. So the key row works in its new place unchanged,
  and a key saved in settings is the same provider the reader's chat uses.
- **The key row's local state goes with it.** `draft` is a `use_signal` inside
  `ApiKeyControl`, so it belongs to that component instance. Switching sections unmounts it
  and clears a half-typed key. That's the right default for a secret.
- **The exhaustive `match` paid off.** Step 3's empty `Ai => rsx! {}` arm is where this step
  plugs in, and it's the only place that had to change in the screen.
- **Narrower visibility states the boundary.** With `pub(crate)` gone, the compiler enforces
  that the rest of the crate reaches the Gemini rows only through `GeminiSettings`. That is
  the seam Phase 23 copies with an OpenCode Zen block.

### Scope note

- The heading is only a label. There's no per-provider card, border or description yet.
  Phase 23 adds the second block, which is when a shared "provider block" shape would pay
  for itself.
- *Gemini API key* repeats the heading's "Gemini". The row's own label could shrink to *API
  key*. That is a candidate for **Step 6**.
- There's no phone layout yet. That is **Step 5**.

### Review notes (from the `simplify` pass)

Two review agents ran, one for reuse and simplification, one for efficiency and altitude.

- **Applied:** `AiModelPicker` and `ApiKeyControl` changed from `pub(crate)` to private,
  because the move left them used only inside `settings.rs`.
- **Deferred to Step 6:** the four reading controls from Step 3 (`LineHeightControl`,
  `FontSizeControl`, `PageMarginsControl`, `MaxLineLengthControl`) are also used only
  inside `settings.rs` now, behind `ReaderThemeControls`, but are still `pub(crate)`.
- **No other findings.** `GeminiSettings` adds no state or re-renders of its own, and a
  second provider is a sibling in the same `match` arm, not a special case.

## Step 5 — The phone layout, and a styling pass

> **Written by:** the agent, at the learner's explicit "write it" after a UI review.
> Implementation and tests written by the agent, reviewed by hand, then cleaned with a
> `simplify` pass.

> **Status:** done — committed in `ae4f1ab` (211 tests green, 2 ignored; clippy clean; iOS simulator checks passed; desktop `dx serve` not yet checked by eye).

**What it is.** Below `40rem` the settings screen is a list you tap into: the section list
fills the screen, tapping a row pushes that section full width, and a back button returns
to the list. Wider windows keep the sidebar. The step grew to include the styling review
that asked for the screen to be "prettier, aligned with app styles and mobile friendly", so
it also restyles every control on the screen and in the reader popover.

### Runnable check first

**Kind:** iOS simulator (driven by a haiku subagent with `agent-device`) plus `#[test]`s for
the new labels. 211 passed, 2 ignored; `cargo clippy` clean.

- `each_theme_has_a_reader_facing_label` and `each_font_family_has_a_reader_facing_label`
  pin the text the pickers now show instead of slugs. The theme test was watched to fail
  by mutating its expected value (`Night` → `Dusk`) and restored.
- On a 402pt iPhone: the list shows full width with no back button; *Reader theme* opens the
  section with a back button; stepper buttons are 40×40; the font-size stepper goes 100% →
  125%; the AI section's field, *Save* and model picker end at x = 370; back returns to the
  list; closing and reopening lands on the list again.
- The reader popover below `40rem` is a bottom sheet (x = 16, width 370) instead of a panel
  anchored to its trigger, which would have run off the left edge.

Not checked by eye yet: the desktop layout under `dx serve`, and the Sepia and Night themes.

### What changed

- **One state, two layouts.** `SettingsScreen` holds `Signal<Option<SettingsSection>>`.
  `None` shows the list on a phone and the first section on desktop. The root carries
  `data-section-chosen` only when a section is picked, and a single media query decides
  which pane to hide. Rust never learns the window width.
- **Mounted only while open.** `library.rs` renders `SettingsScreen` inside
  `if settings_open()`, so its local signal starts at `None` on every open. That replaced a
  manual reset in the close handler.
- **Grouped cards.** Rows sit in a rounded card with hairline dividers
  (`.settings_group > * + *`), labels on the left and controls on the right, at least
  48px tall.
- **`Stepper`** replaces the four copy-pasted `…Control` components. It renders its own
  `SettingRow`, so each setting's label is written once and also feeds the button names
  (*Decrease Font size*).
- **Labels, not slugs.** `Theme::label()` and `FontFamily::label()`; `SlugPicker` takes
  `(slug, label)` pairs and an accessible name, owns its styling in `picker.css`, and now
  also backs `AiModelPicker`.
- **Colours from the theme.** Five `--tint-*` custom properties on `:root` in `main.css`
  mix `currentColor` into transparency. An unregistered custom property substitutes its
  tokens where it is used, so `currentColor` resolves per element and Day, Sepia and Night
  all get matching tints without a dark-mode branch.
- **Gemini block.** An uppercase group title, an *API key* row with a status badge, a
  wrapping password field (16px font so iOS doesn't zoom) and pill *Save* / *Forget*.

### Review notes (from the `simplify` pass)

- **Applied:** `AiModelPicker` through `SlugPicker`; the stepper wrappers folded into
  `ReaderThemeControls`; tint variables; one disabled opacity; breakpoint aligned to
  `40rem`; mount-when-open; `key_set` and `chosen_now` read once; the popover's
  narrow-screen sheet.
- **Deferred:** a global pill-button class shared with the library delete dialog; moving
  the narrow-screen sheet and theme colours into the popover component so `toc.css` and
  `settings.css` stop out-specifying it with attribute selectors; `:where(.icon-button)` so
  the back button can hide with a single class.
- **Skipped:** formatting stepper values inside `Stepper` to save an allocation; hiding
  *Reader theme*'s `aria-current` on the phone list, which would need Rust to know the
  width.
- **Step 6 items already covered here:** labels for the controls, *API key* instead of
  *Gemini API key*, and the reading controls made private.
