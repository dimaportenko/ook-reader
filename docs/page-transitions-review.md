# Page transitions — review findings

Code review of the buffered two-iframe chapter transition work (working tree, 2026-09-01).
Companion to [`page-transitions.md`](page-transitions.md), which describes how the system is
*meant* to work; this file records where the current implementation diverges from that.

Line numbers refer to the working tree at review time and will drift as the files change.

## Summary

| # | Severity | File | Symptom |
| --- | --- | --- | --- |
| 1 | High | `reader-controller.js:147` | A failed chapter fetch leaves `pending` set and bricks the reader |
| 2 | High | `reader-controller.js:297` | `complete()` wipes a newer `pending` installed mid-animation |
| 3 | Medium | `reader-controller.js:196` | `ook-swipe` dropped after the iframe already committed to the gesture |
| 4 | Medium | `pointer-gesture-listener.js:102` | `boundary` flipping true→false is never reported to the host |
| 5 | Medium | `reader-controller.js:178` | `ook-scroll` does not record the page on the slot |
| 6 | Medium | `reader-controller.js:264` | Position for a newly entered chapter is never saved |
| 7 | Medium | `reader-controller.js:100` | Spine dedupe swallows a fragment-only change |
| 8 | Medium | `reader-controller.js:148` | Hash written to a slot whose document may still be in flight |
| 9 | Medium/low | `reader-controller.js:229` | `cancelDrag`'s settle timer can blank the frame that just became active |
| 10 | Low | `pointer-gesture-listener.js:141` | Zero-delta drag returns without cleanup |
| 11 | Low | `pagination.css:13` | `touch-action: pan-y` disables pinch-zoom |

Findings 1, 5, and 11 are one- or two-line fixes.

## High

### 1. A failed chapter fetch permanently bricks the reader

`reader-controller.js:147` (with `:80`)

`fetchInto` returns `false` when `!response.ok`, and `load` then returns at `if (!loaded)
return;` with `this.pending` still set from line 129. Nothing else ever clears it.

Consequences, all permanent until the book is closed:

- Line 196 drops every message from the active frame — `ook-tap`, `ook-key`, `ook-swipe`,
  `ook-position`, `ook-link`.
- `ready:` is never sent, so Rust stays in `Phase::Loading` and `page_prev` / `page_next`
  return `false` ([`nav.rs:88`](../src/nav.rs), `:95`).
- Preloading is blocked by `if (existing || this.pending) return` at line 117.

A thrown `fetch` (line 79) is the same failure by a different route: the rejection
propagates through the top-level `await` at line 323 and is unhandled.

The fix has two halves: clear `pending` on every failure path, and tell Rust the
navigation failed so it can leave `Loading`.

### 2. `complete()` nulls a newer `pending`

`reader-controller.js:297`

`complete()` sets `this.pending = null` unconditionally, without checking that the pending
it is clearing is still the one it started. `follow_link` has no `Phase::Loading` guard
([`reader.rs:152`](../src/ui/reader.rs) → [`nav.rs:147`](../src/nav.rs)), so tapping a TOC
entry during the ~240 ms chapter slide installs a new pending pointing at the slot that is
mid-animation. When `complete()` fires it calls `setActive` on that slot and drops the new
pending; the newly requested chapter's `ook-ready` then arrives with `isTarget === false`,
`finishNavigation` never runs, `ready:` never arrives, and the reader is frozen in
`Loading`.

Either capture the pending at animation start and only clear it if unchanged, or guard
`follow_link` against `Phase::Loading` — preferably both.

## Medium

### 3. `ook-swipe` is dropped after the iframe has already committed

`reader-controller.js:196`

`ook-swipe` is routed below the `if (!isActive || this.pending) return` guard, but by the
time it is posted the iframe has already committed to the gesture: for an accepted swipe,
`pointerup` deliberately skips `finishLocalDrag()` and waits for the host's
`resolveGesture` ([`pointer-gesture-listener.js:143-150`](../src/web/assets/chapter/input/pointer-gesture-listener.js)).

Swipe during a chapter transition and the reply never comes, so `--ook-drag-x` stays at the
drag offset for that document forever — the outgoing chapter renders shifted, and stays
shifted if that slot is later re-activated through the `existing` path.

Reject the swipe explicitly instead of silently dropping it.

### 4. `boundary` flipping true→false is never reported

`pointer-gesture-listener.js:102`

`swipeFrom.boundary` is recomputed on every `pointermove`, but only the true case notifies
the host. Repro on page 0 of a multi-page chapter:

1. Drag right ~50 px. `boundary` is true, so the host gets `ook-drag`, shifts both frames,
   and shows the previous chapter peeking in.
2. Drag left past the origin. `boundary` is now false (there is a next page in-chapter), so
   the iframe silently switches to painting a local drag.
3. Release with `|dx| < 40`.

At `pointerup` `boundary` is false, so only `finishLocalDrag()` runs inside the iframe. No
`ook-drag-cancel` is posted, and nothing resets `frame.style.transform` — only `setActive`
does, and only on the next chapter change. The reader stays visibly offset with a
neighbouring chapter peeking in.

Post `ook-drag-cancel` on the true→false transition, or make the host's transform reset
unconditional at gesture end.

### 5. `ook-scroll` does not record the page on the slot

`reader-controller.js:178`

The `ook-scroll` branch forwards the page to Rust and then calls `finishNavigation()`
synchronously on the next line, before Rust can echo anything back. `finishNavigation`
(line 263) therefore reads `incoming.page`, still `0` from `fetchInto` line 76, and forces
the incoming chapter to page 0. Following a link — or restoring a saved position — that
resolves to page 7 slides the chapter in at page 0 and then jumps.

Fix: `slot.page = data.page` in that branch.

### 6. The position for a newly entered chapter is never saved

`reader-controller.js:264`

`finishNavigation`'s `setPage` makes `page-position.js` post `ook-position`, but
`this.pending` is still set — it is cleared ~240 ms later in `complete()` — so line 196
drops it. The Rust `setPage` effect does not re-fire on `ready:`, so nothing saves the
position afterwards either. Open chapter N+1, close the book, and it reopens at chapter N.

### 7. The spine dedupe swallows a fragment-only change

`reader-controller.js:100`

`this.pending?.spineIndex === targetSpine` treats two navigations into the same chapter as
duplicates even when their fragments differ. Click TOC entry A, then entry B in the same
chapter before it loads: the second `load` returns immediately, no `ook-scroll` ever
arrives, `pending.awaitScroll` stays `true`, and `finishNavigation` (line 258) never runs.
Rust's `Pending::Fragment` is never cleared, leaving it stuck in `Loading` with
`is_settling()` true — which also disables position saving
([`reader.rs:371`](../src/ui/reader.rs)).

Compare the fragment as well as the spine index, and update the pending target when only
the fragment changed.

### 8. The hash can be written to a document that is still in flight

`reader-controller.js:148`

Writing `win.location.hash` on an `existing` slot assumes that slot's document is loaded.
It need not be: `slot.url` is set at `fetchInto` line 72, long before `frame.src` at line
90, so the `existing` lookup at line 98 can match a preload still in flight. The hash then
lands on the outgoing or blank document and is lost when `src` navigates. No `ook-scroll`
follows, `awaitScroll` never clears, and the navigation hangs exactly as in finding 7.

Gate the hash write on `slot.ready`, and otherwise defer the fragment until `ook-ready`.

### 9. `cancelDrag`'s settle timer can blank the new active frame

`reader-controller.js:229`

`cancelDrag` captures `active` and `standby` at call time, then 260 ms later forces
`standby.frame.style.opacity = "0"` and strips `reader-frame--settling` from both. Unlike
`ook-drag` (line 188), the `ook-drag-cancel` handler (line 192) has no `!this.pending`
guard, so a `setActive` swap landing inside that window blanks the frame that just became
active — both frames at opacity 0, an empty reader until the next chapter change.

Re-read the slots inside the timer, or drop the timer when a swap has happened.

## Low

### 10. A zero-delta drag returns without cleanup

`pointer-gesture-listener.js:141`

`if (dx === 0 && dy === 0) return;` sits above the settle branch. A drag that returns to
its start (after rounding) has `moved === true`, so the tap branch is skipped and this line
returns without `finishLocalDrag()` or `ook-drag-cancel` — the same stuck-offset frames as
finding 4, on a narrower path.

### 11. `touch-action: pan-y` disables pinch-zoom

`pagination.css:13`

`pan-y` on `html` does not imply `pinch-zoom`, so readers can no longer zoom a chapter on
touch devices. `touch-action: pan-y pinch-zoom` keeps the horizontal-drag intent without
the accessibility regression.

## Out of scope

`cargo test` fails on `ui::reader::test::the_page_label_waits_for_a_real_count` (expects
`"Page …"`, gets `"…"`). The failure is pre-existing at `HEAD` — `page_label` is untouched
by this diff — but the working tree is not green.
