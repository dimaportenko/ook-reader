# App Icon — Generation Brief

A ready-to-use brief for generating the **Ook Reader** app icon with an AI image tool.

## Project idea (context for the icon)

**Ook Reader** is a cross-platform **EPUB reader** built in Rust + Dioxus 0.7,
desktop-first (macOS/Windows/Linux) with mobile and web to follow. The identity is
*reading a book*. The name "Ook" nods to Terry Pratchett's orangutan Librarian (who only
ever says "Ook") — so an optional secondary motif is a subtle orangutan/librarian nod, but
the **primary, must-read-instantly symbol is a book**. It should look clean and modern like
a native app icon, not skeuomorphic.

## Design constraints (for small-size legibility)

- **One primary shape** (an open book) — legible down to 16×16.
- **Flat, geometric figures**, bold silhouette, no thin lines or fine detail.
- **2–3 colors max** plus a background; high contrast.
- **Generous margins**, centered, works on a rounded-square app tile.
- No text, no gradients-as-detail, no photorealism.

## Prompt (paste into the image tool)

> Flat vector app icon for an EPUB e-book reader called "Ook Reader". A single bold,
> minimal **open book** rendered as simple geometric flat shapes, viewed head-on and
> centered on a rounded-square tile. Two clean pages forming a soft V, a clear spine down
> the middle, subtle page edges — no fine detail, no text. Strong silhouette that stays
> recognizable at 16×16 pixels. Modern flat design, solid fills, no gradients, no shadows,
> no outlines-as-detail. Limited palette: a warm amber/orange book on a deep indigo
> background (max 2–3 colors), high contrast. Balanced margins, minimalist, professional
> native-app icon style. Vector, crisp edges, iOS/macOS icon aesthetic.

## Negative prompt (if supported)

> photorealism, 3D render, skeuomorphic leather, gradients, drop shadows, gloss, thin
> lines, tiny text, letters, clutter, busy background, realistic pages, stacks of many
> books, hands, faces.

## Variants worth generating

- **Variant A — pure book** (the prompt above): safest, most instantly legible.
- **Variant B — book + subtle "Ook" nod:** append to the prompt: *"an orange orangutan
  silhouette subtly integrated as the book's ribbon bookmark or peeking over the top edge,
  still reading clearly as a book first."* Only use this if it survives the small-size test;
  otherwise keep A.

## Chosen icon (in the repo)

Generated with **Variant B** — an orangutan (the "Ook" nod) above an open book with a
bookmark ribbon, amber on deep indigo. The image was turned into an icon set and wired into
the app bundle.

- **Source art:** `assets/icons/icon.png` (1024²).
- **Generated sizes:** `assets/icons/32x32.png`, `128x128.png`, `128x128@2x.png`,
  `icon.icns` (macOS), `icon.ico` (Windows).
- **Webview favicon:** `assets/favicon.ico`, regenerated from the same art.
- **Bundle wiring:** `Dioxus.toml` `[bundle]` `icon = [...]` — `dx bundle` embeds it into
  the installable `.app`/`.dmg` (and `.exe` on Windows).

The **bundled** app shows the icon. In `dx serve` **dev mode** the Dock still shows the
generic icon: macOS takes the Dock icon from the `.app` bundle, and the only runtime
override (`NSApplication::setApplicationIconImage` via AppKit) wasn't worth the extra
macOS-only dependencies for a dev-only nicety.
