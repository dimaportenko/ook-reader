# Phase 1 — Learn Rust + Dioxus

[← Feature: Rust + Dioxus Fundamentals](README.md) · **Status:** ⬜ planned (parallel)

Per-step build log (crux, step plan, test → code → why):
[`phase-1-learn-rust-dioxus-steps.md`](phase-1-learn-rust-dioxus-steps.md).

## Goal

Get comfortable with Rust syntax and idioms, and the Dioxus component/state model —
enough to read and write reader code without fighting the language or framework.

## Rust topics

- [ ] Ownership, borrowing, lifetimes — the borrow checker mental model
- [ ] `struct` / `enum` (with data); pattern matching, exhaustive `match`
- [ ] `Option` / `Result`, the `?` operator, error handling
- [ ] Traits & generics; trait objects (`dyn`) — relevant for a persistence trait
- [ ] Closures, iterators (`map`/`filter`/`collect`)
- [ ] `async`/`await`, `Future`, the basics of a runtime
- [ ] Modules, crates, `Cargo.toml` features

## Dioxus topics

- [ ] Components (`#[component]`, `Element`), the `rsx!` macro
- [ ] Props (`#[derive(Props)]`), event handlers
- [ ] State: `use_signal`, `Store` (nested state), `use_resource` (async), `use_effect`
- [ ] Context (`use_context_provider` / `use_context`) for app-wide settings
- [ ] Routing: `#[derive(Routable)]`, `Link`, `Outlet`, layouts
- [ ] Assets/styling: the `asset!` macro, `document::Stylesheet`

## Resources

- [The Rust Programming Language ("the book")](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Dioxus 0.7 guide](https://dioxuslabs.com/learn/0.7/) +
  [tutorial](https://dioxuslabs.com/learn/0.7/tutorial/)
- [Dioxus 0.7 release notes](https://dioxuslabs.com/blog/release-070/) (Stores, Subsecond)

## Notes

The webview renders the book's XHTML/CSS; most reader logic in Rust is parsing (`rbook`),
state, persistence, and serving resources to the webview. See
[`RESEARCH.md`](../../../../RESEARCH.md) §3, §6.
</content>
