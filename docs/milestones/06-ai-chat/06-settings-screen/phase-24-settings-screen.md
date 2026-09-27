# Phase 24 — Settings screen

[← Feature: Settings screen](README.md) · **Status:** 🚧 in progress — opened 2026-09-27 ·
build log: [`phase-24-settings-screen-steps.md`](phase-24-settings-screen-steps.md)

## Goal

A gear icon button in the library's action bar opens a full-screen *Settings* view with a
close button. Sections: **Reader theme** and **AI**.

- **Desktop / wide window:** a sidebar on the left lists the sections, and the chosen one
  fills the right.
- **Phone / narrow window:** the section list is the whole screen. Tapping a section pushes
  it full width with a *back* button, like iOS Settings.

The AI section has one block per provider. Only **Gemini** exists now, with its key row and
model picker, moved out of the reader's popover. The phase closes when the screen works on
desktop and on the iOS simulator, and the AI rows are gone from the reader popover.

> **Inserted ahead of Phase 23** on 2026-09-27, at the learner's call, before Phase 23's
> Step 1 was written. Phase 23 needs a home for a second key and a model list you choose
> from, and the popover has no room for either.

## The crux

**A screen without a router.** The app switches views with one signal: `open_book` decides
whether `App` renders `Reader` or `LibraryBooks`. Settings doesn't need to be a third
top-level view. It's an **overlay owned by the library**: a `Signal<bool>` in
`LibraryBooks` and a `position: fixed` layer on top. That's the same pattern as the chat
`Drawer`, so it comes with the same safe-area lesson: a fixed layer escapes `body`'s
safe-area padding and has to pay for the notch itself.

The one real design idea is **one state, two layouts.** The chosen section is
`Signal<Option<SettingsSection>>`. On a phone, `None` means "show the list" and `Some` means
"show that section". On desktop the sidebar is always visible and `None` just shows the
first section. So Rust holds one piece of state that works on both, and the only per-width
decision is CSS (a media query on a `data-` attribute), which is the one layer that knows the
window width anyway.

## Design decisions (recorded up front)

- **Overlay, not a route.** No `dioxus-router`. One screen isn't worth adding a router to
  the app. Revisit if a third screen shows up.
- **The reader popover keeps the reading controls.** Font size, line height, margins, line
  length, font and theme stay a tap away while reading. The *Reader theme* section **reuses
  the same components**, so there is one control and two places to find it, not two
  implementations. Only the **AI** rows move out of the popover for good.
- **Width, not platform, picks the layout.** A breakpoint in CSS, not `cfg(target_os)`. An
  iPad in landscape gets the sidebar, a narrow desktop window gets the list, and both are
  correct.
- **The Gemini block is the pattern Phase 23 copies:** a heading with the provider's name,
  the key row, then the models. Phase 23 adds an OpenCode Zen block beside it.

## Planned steps

- [x] 1. The gear and the empty screen — icon button in `LibraryBooks`, full-screen `SettingsScreen` overlay with a header and close
- [x] 2. Sections in a sidebar — `SettingsSection`, sidebar buttons, the chosen section's heading on the right
- [ ] 3. Reader theme section — the existing controls reused inside it
- [ ] 4. AI section: Gemini — key row and model picker moved out of the reader popover
- [ ] 5. The phone layout — list → section → back, under a width breakpoint, checked on the iOS simulator
- [ ] 6. Review and refactor
