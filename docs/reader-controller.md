# Reader Controller

[`reader-controller.js`](../src/web/assets/host/reader/reader-controller.js) implements a
two-iframe chapter buffer. One iframe displays the current chapter while the other can
preload or animate in another chapter.

The central distinction is:

- Slots are long-lived, reusable containers.
- Chapters are temporary contents of slots.
- `pending` is a short-lived navigation transaction between active states.

## Core Entities

| Name | Meaning |
| --- | --- |
| `frames` | The raw `<iframe class="reader-frame">` DOM elements. |
| `slots` | Reader-owned wrappers around the iframes, including chapter and pagination state. |
| `reader` | The persistent controller stored in `window.__ookReader`. |
| `active` | The index of the currently visible slot, not the slot itself. |
| `pending` | An in-progress navigation transaction. `null` means navigation is settled. |
| `target` | The slot selected to receive the requested chapter during `load()`. |
| `incoming` | The pending target slot once navigation is ready to finish. |
| `outgoing` | The currently active slot while it is being animated away. |
| `standby` | Whichever slot is not currently active. It may contain a preloaded chapter. |
| `existing` | A slot that already contains the requested URL, avoiding another fetch. |

The names describe roles that change over the lifetime of one navigation:

```text
target during loading
    |
    v
incoming during transition
    |
    v
active after completion

old active
    |
    v
outgoing during transition
    |
    v
standby after completion
```

## Chapter Slots

Each `ChapterSlot` represents one reusable iframe:

| Field | Meaning |
| --- | --- |
| `index` | Stable position in the `slots` array. |
| `frame` | The actual iframe element. |
| `blobUrl` | Temporary object URL created from the fetched chapter. |
| `url` | Original chapter URL currently assigned to the slot. |
| `spineIndex` | Chapter position in the EPUB spine. |
| `generation` | Request version used to reject stale asynchronous fetches. |
| `ready` | Whether the chapter has loaded, laid itself out, and sent `ook-ready`. |
| `pageCount` | Number of horizontal pages in the chapter. |
| `page` | Current page within the chapter. |

`generation` protects a reused slot from an older asynchronous fetch. `fetchInto()`
increments it before fetching and captures that value locally. If the slot has been
assigned again by the time the response arrives, the captured generation no longer
matches and the stale response is discarded.

## Active Slot

`reader.active` stores the index of the visible slot:

```js
const active = this.slots[this.active];
```

`setActive()` updates both the state and the iframe presentation. It controls opacity,
pointer events, accessibility state, inertness, and the active and standby CSS classes.

## Pending Navigation

`pending` is not a pending chapter object. It is the state of an unfinished navigation
operation:

```js
this.pending = {
  slot: target.index,
  spineIndex: targetSpine,
  direction,
  seekLast: targetLast,
  awaitScroll: Boolean(targetFragment),
  finishing: false,
};
```

| Field | Meaning |
| --- | --- |
| `slot` | Index of the destination slot. |
| `spineIndex` | Requested EPUB chapter index. |
| `direction` | `1` for forward, `-1` for backward, and `0` for the initial load. |
| `seekLast` | Whether to open the final page, usually when moving backward into the previous chapter. |
| `awaitScroll` | Whether fragment navigation must report its resulting page before the transition can finish. |
| `finishing` | Prevents `finishNavigation()` from starting twice when several iframe messages arrive close together. |

While `pending` exists, normal input from the active iframe is mostly suppressed. This
prevents gestures and position events from interfering with a chapter transition.

There is also a Rust variable named `pending` in
[`src/ui/reader.rs`](../src/ui/reader.rs). The two names have different scopes and
meanings:

| Name | Meaning |
| --- | --- |
| Rust `pending` | Navigation intent, such as fragment navigation or opening the previous chapter's last page. |
| JavaScript `reader.pending` | The iframe navigation transaction currently executing that intent. |

## Target, Incoming, and Outgoing

`target` is used while deciding where the requested chapter should be loaded:

```js
const target = existing || (active.url ? this.slots[1 - this.active] : active);
```

The controller applies these rules:

- Reuse `existing` if the chapter is already loaded.
- Use the other slot if an active chapter already exists.
- Use the active slot itself for the initial load.

Inside `finishNavigation()`, that destination is called `incoming`:

```js
const incoming = this.slots[pending.slot];
```

The old visible slot becomes `outgoing`:

```js
const outgoing = this.slots[this.active];
```

These names emphasize the slots' animation roles. `incoming` moves onto the screen while
`outgoing` moves off it. Once the transition completes, `setActive(incoming.index)` commits
the role change.

## Existing and Standby Slots

`existing` is found by chapter URL:

```js
const existing = this.slots.find((slot) => slot.url === targetUrl);
```

This is how preloading pays off. If the non-active slot already contains the requested
chapter, navigation can skip `fetch()`.

`standby` means only the slot opposite `active`:

```js
const standby = this.slots[1 - this.active];
```

During a drag, the controller reveals it only if it is ready and its chapter lies in the
gesture's navigation direction.

## Load Request

Rust sends the controller a `LoadRequest` tuple:

```js
const [kind, url, fragment, spineIndex, seekLast] = await dioxusEval.recv();
```

| Value | Meaning |
| --- | --- |
| `kind` | Either `"navigate"` or `"preload"`. |
| `url` | Fetch URL for the chapter document. |
| `fragment` | Optional XHTML anchor without the `#`. |
| `spineIndex` | Chapter index in EPUB reading order. |
| `seekLast` | Whether to land on the chapter's last page. |

Inside `load()`, these become `targetUrl`, `targetFragment`, `targetSpine`, and
`targetLast`. The `target` prefix means requested destination and distinguishes those
values from the chapter metadata currently committed to a slot.

## Navigation Lifecycle

1. Rust sends either a `"navigate"` or `"preload"` request.
2. `load()` selects a target slot.
3. Navigation creates `pending`; preloading does not.
4. `fetchInto()` fetches the chapter and assigns a blob URL to the target iframe.
5. The iframe reports page count, fragment scrolling, and readiness through frame messages.
6. `finishNavigation()` waits until `incoming.ready` is true and any fragment scroll has completed.
7. `incoming` and `outgoing` animate.
8. `incoming` becomes `active`.
9. `pending` is cleared and `ready:` is sent back to Rust.

The preload path stops after filling the standby slot. A later navigation finds that slot
as `existing`, skips the fetch, and can proceed as soon as its readiness gates are
satisfied.
