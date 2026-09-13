# Phase 18 — API key storage + settings row

[← Feature: Key storage](README.md) · **Status:** 🚧 in progress — opened 2026-09-13 ·
build log: [`phase-18-key-storage-steps.md`](phase-18-key-storage-steps.md)

## Goal

The Gemini key is entered once in the settings popover, stored in the OS keychain, and read
back on launch into a `Gemini` the rest of the app can reach through context. The settings
row never shows the key again — only *key set* / *not set* — and a model picker chooses
between Flash-Lite and Flash. The phase closes when a fresh launch, with a key entered on
the previous one, has a provider ready without any env var, on macOS and on the iOS
simulator.

## The crux

**A secret is a setting with a different home, and the home is the only thing the app may
not test.** Every other setting lives in the `settings` row in SQLite — a plain file. A key
there is a key on disk. So it goes to the keychain, and the keychain is exactly like the
network in Phase 17: a system service `cargo test` must not touch. The same cut applies:
one thin trait (`SecretStore`) with an in-memory implementation for tests and one real
implementation behind an `#[ignore]` test. Everything the UI needs — *is a key set?*, *save
this one*, *forget it* — is written against the trait, so it runs in the suite.

The second insight is **what the UI is allowed to know**. The row shows a status, not the
key: the key goes *in* through an input and never comes *out* to the screen. That keeps
`String`s holding the secret confined to two places — the store and the `Gemini` value —
and the settings signal keeps carrying `Copy` data as it does today.

## Design decisions (recorded up front)

- **A new module, `src/secrets/`**, owning the trait, the `Memory` store, and the
  `Keychain` store. `ai` does not know about it; `main.rs` joins them, as it joins `Db`
  and `Settings` today.
- **`keyring-core` + `apple-native-keyring-store` for the keychain** (not the `keyring` `v1` wrapper, whose default store refuses iOS — see Step 2), service name `com.dimaportenko.ook-reader`,
  one entry per secret name. On iOS the Apple store needs its data-protection (`protected`)
  feature and the app's keychain entitlement — the step that adds the crate ends with
  `dx build --platform ios`, the same tripwire Phase 17 used for TLS. If the crate turns
  out to fight iOS, the fallback is `objc2` against Security.framework, in the same file
  behind the same trait; the trait is what makes that swap cheap.
- **Model choice is an ordinary setting.** `AiModel` is a `Copy` enum with a slug, a new
  column in the `settings` row, and a picker next to the theme picker. It is not a secret
  and it is not provider-specific plumbing: `Gemini` takes a model name at construction.
- **The provider reaches the UI as `Signal<Option<Gemini>>`** through context. `None` until
  a key exists; the chat panel in Phase 19 reads it and shows "add a key in settings" on
  `None`. No generic provider type yet — one real provider, per Phase 17's decision.
- **The key is validated by using it, not by pattern.** No regex on the key format; the
  first real request in Phase 19 reports `ChatError::Api` if it is wrong.

## Planned steps

Detail for each lives in [`phase-18-key-storage-steps.md`](phase-18-key-storage-steps.md).

- [x] **1. The secret boundary** — `src/secrets/mod.rs`: `SecretStore` trait (`get`, `set`,
      `forget`), `SecretError`, the `GEMINI_API_KEY` name, and a `Memory` store; `#[test]`.
- [x] **2. The keychain behind the trait** — `keyring-core` +
      `apple-native-keyring-store`, `Keychain` store,
      `#[ignore]` round trip against the real keychain, `dx build --platform ios`.
- [x] **3a. The model value** — `AiModel` enum with stable slugs and Gemini API names;
      `#[test]` on the slug round trip, fallback and provider names.
- [ ] **3b-i. Add and backfill the model column** — migrate an existing singleton
      `settings` row exactly once; `#[test]` on the old on-disk shape.
- [ ] **3b-ii. Persist the model value** — add the model to `Settings`, `save_settings`,
      and `settings()`; `#[test]` on the db round trip.
- [ ] **3c. Give Gemini the chosen model** — add `Gemini::with_model`; `#[test]` that the
      chosen model reaches the endpoint.
- [ ] **4. The provider in context** — `main.rs` reads the key on launch into
      `Signal<Option<Gemini>>`; a helper that rebuilds it when the key or model changes;
      `#[test]` on the helper, `dx serve` for the wiring.
- [ ] **5. The settings row** — key input + *set / not set* + forget button + model picker
      in `SettingsPopover`; `dx serve` eyeball on desktop, simulator check on iOS.
- [ ] **6. Review and refactor** — punch-list over `src/secrets/` and the touched files,
      suite green, clippy clean.
