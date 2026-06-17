# Phase 1 — Learn Rust + Dioxus — Build Log

[← Phase doc](phase-1-learn-rust-dioxus.md)

Per-step build log for the fundamentals track: the crux, the step plan, and for each step
the runnable check → minimal implementation → why it works. The phase doc holds the
high-level topic checklist; this file is the detailed trail. Newest step appended at the
bottom.

## The crux (Dioxus state)

A Dioxus component is a plain function that Dioxus **re-runs top-to-bottom every time it
needs to repaint**. A normal `let count = 0;` is therefore useless for state — it resets on
every render. `use_signal` hands you a *handle* to a value that lives **outside** the
function body, in Dioxus's runtime. The whole reactive model is two rules:

- **Reading** a signal (e.g. interpolating `{count}`) **subscribes** this component —
  "re-run me when this changes."
- **Writing** a signal **schedules a re-render** of every subscriber.

## Step plan (smallest-first, one idea each)

1. ✅ **`use_signal` counter** — state survives re-renders; a button mutates it. *(reactivity core)*
2. **`use_memo`** — a derived value that recomputes only when its dependency changes.
3. **Props** — a `#[component]` fn taking owned args; the `PartialEq` re-render rule.
4. **List rendering** — a `for` loop in `rsx!` over a `Vec` held in a signal.
5. **Context** — `use_context_provider` / `use_context` for app-wide settings.
6. **Routing** — a `Route` enum (`Library`, `Reader`), `Router`, layout + `Outlet`
   (also finishes Phase 2's shell).
7. **`use_resource`** — async state; the bridge into loading an `.epub` (Milestone 2).

---

## Step 1 — a `use_signal` counter

> **Status:** done — committed in `279c4f2`. Visual step: build + `cargo clippy`
> clean, behavior confirmed under `dx serve` (Count: 0 → Increment advances it).
> No unit tests (UI step).

### Runnable check (eyeball under `dx serve`)

This is Dioxus UI, so the check is a visual one, not `cargo test` (component rendering
isn't unit-testable without extra harness; later pure-Rust steps get real `#[test]`s).

```
dx serve --platform desktop
```

Expect **"Count: 0"** and an **Increment** button; clicking advances `1, 2, 3…` and the
text updates each click. That visible update proves the signal holds state across
re-renders *and* that writing it triggers a re-render. Also run `cargo clippy` (expect
clean).

### Minimal implementation

A new `Counter` component, rendered from `App` (drop `Hero {}` or add `Counter {}` above
it).

```rust
#[component]
fn Counter() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            h1 { "Count: {count}" }
            button {
                onclick: move |_| *count.write() += 1,
                "Increment"
            }
        }
    }
}
```

### Why it works

- **`use_signal(|| 0)`** — the closure produces the initial value *once*, on first mount.
  Every later re-render returns the *same* signal, which is why the count isn't reset. The
  returned handle is `Copy`.
- **`"Count: {count}"`** — `{count}` *reads* the signal, and a read during render
  **subscribes** the component: "re-run me when `count` changes."
- **`onclick: move |_| ...`** — the closure must be `move` because it captures `count` and
  outlives the render that created it. The handle is `Copy`, so `move` copies the handle,
  not the value — `count` is still readable in the `h1`.
- **`*count.write() += 1`** — `.write()` returns a mutable guard borrowing the inner value;
  `*` derefs it to the `i32` so `+= 1` mutates in place. Dropping the guard marks the
  signal dirty → Dioxus re-renders every subscriber → the `h1` updates.

### Scope note

No derived value (Step 2's `use_memo`) and no props (Step 3). One self-contained component
so the only thing exercised is the read-subscribe / write-rerender loop.
