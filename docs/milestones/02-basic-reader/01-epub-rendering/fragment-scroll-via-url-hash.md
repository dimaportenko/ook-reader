# Fragment scroll via the URL hash

[← EPUB Rendering](README.md) · chapter-transport rework, 1 of 3

Drafted: 2026-07-25. **Status: implemented** in `30e4b0c` ("fix: support encoded EPUB
fragments and externalize bridge scripts"), 32 tests green. See
[As shipped](#as-shipped) for the deltas from this sketch.

A design sketch for moving `inject_fragment_scroll` out of a Rust string literal and
into a static JS asset — the same treatment `pagination.css` and `page-listener.js`
already got.

> Line references in the sections below describe the code **as it was before**
> `30e4b0c`; they are kept so the reasoning still reads in context.

## Why this one was awkward

The other injected snippets are constants; this one is the only **parameterized**
script, built with `format!` so the anchor id can be interpolated:

```rust
var el = document.getElementById("{fragment}");
window.parent.postMessage({{ kind: 'ook-scroll', page: page }}, '*');
```

Three costs follow from that:

1. **Escaping noise.** Every JS brace has to be doubled (`{{`, `}}`) to survive
   `format!`. It obscures the code and no JS tooling can read the string.
2. **`format!` can't consume an asset.** `format!` requires a *literal* format
   string, so you cannot `format!` an `include_str!` constant. Moving the file
   means giving up interpolation and doing a runtime `.replace("__OOK_FRAGMENT__",
   …)` instead — which loses the compile-time guarantee that the placeholder
   exists. A typo in either file then ships a script that looks up an element
   literally named `__OOK_FRAGMENT__`.
3. **It's an injection hole.** `fragment` comes from the book's own `href` via
   `resolve_internal_link` (`src/epub.rs:99-102`), so untrusted content is dropped
   unescaped inside a JS string literal. A fragment containing `"` breaks the
   script; one containing `");…//` runs arbitrary code in the iframe. Low severity
   (the attacker has to be the EPUB you chose to open) but real, and a plain
   `.replace` into an asset file carries it over unchanged.

All three vanish if the script takes **no parameter at all**.

## The approach

Pass the anchor id the way the web already passes anchor ids: in the URL fragment.
`render_document_url` already returns a `data:` URL — append `#chap01` to it and the
iframe document gets a real `location.hash`. The script reads it and stays static.

```
data:application/xhtml+xml;base64,PGh0bWw…#chap01
                                          ^^^^^^^ not part of the base64 payload
```

The `#` terminates the data portion, so the decoder never sees the fragment. The
document body becomes **identical** whether or not a fragment was requested — which
is what makes the next section work.

## The gotcha that makes or breaks it

Because the payload no longer depends on the fragment, changing the fragment changes
**only the hash**. A URL that differs from the current one only after the `#` is a
*same-document navigation*: the browser does not reload the iframe, and **`load`
never fires again**. A script listening only for `load` would work on the first TOC
click and silently do nothing on the second.

So the script listens for `hashchange` as well as `load`.

Once that's wired, the same-document behavior turns from hazard into benefit. Today
a single TOC jump reloads the iframe **twice**:

| step | `pending_fragment` | document body | today | with the hash |
| --- | --- | --- | --- | --- |
| `follow_link` (`src/nav.rs:87`) | `Some("chap01")` | gains the scroll script | full reload | full reload (chapter changed) |
| `on_scroll` (`src/nav.rs:93`) | `None` | loses the scroll script | **full reload again** | hash-only → no reload |

That second reload re-parses the chapter and re-fires the page-count probe purely to
remove a script tag. The hash version makes it free.

Removing the hash also fires `hashchange` (`"#chap01"` → `""`), which is why the
script must bail out on an empty hash rather than assume it has work to do.

## The check first

One test carries most of the weight — it pins the hash *and* proves the payload is
fragment-independent, which is the property the whole design rests on:

```rust
#[test]
fn fragment_rides_in_the_url_hash_not_the_document() {
    let doc = SpineDoc {
        href: "c1.xhtml".into(),
        xhtml: r#"<html><head></head><body><p id="chap01">Hi</p></body></html>"#.into(),
    };

    let with = render_document_url(&doc, Some("chap01"));
    let without = render_document_url(&doc, None);

    assert_eq!(with, format!("{without}#chap01"));
}
```

`injects_fragment_scroll_before_head_close` (`src/epub.rs:414-428`) gets rewritten to
assert on the static asset instead of an interpolated string. `hashchange` is the
assertion worth having — it's the one whose absence you'd otherwise only discover by
clicking twice:

```rust
let js = get_wrapped_js(FRAGMENT_SCROLL_JS);
assert!(js.contains("hashchange"));
assert!(js.contains("location.hash"));
assert!(js.contains("ook-scroll"));
```

And one thing tests can't cover — under `dx serve`:

- Open Contents, click a chapter → it lands on the right page. (The `load` path.)
- Open Contents again and click a **different anchor in that same chapter** → it
  moves. (The `hashchange` path. This is the click that fails if `hashchange` is
  missing.)
- Watch for the page being shifted sideways. Setting a hash makes the browser scroll
  the anchor into view on its own, and `html { overflow: hidden }`
  (`src/web/assets/pagination.css:4-8`) does not prevent *programmatic* scrolling —
  that offset would stack on top of the `--ook-page` transform.

## The sketch

`src/web/assets/fragment-scroll.js` — fully static, no placeholder, nothing to escape:

```js
function reportFragmentPage() {
  var id = decodeURIComponent(location.hash.slice(1));
  if (!id) return;                         // no hash → inert, so inject unconditionally
  var el = document.getElementById(id);
  if (!el) return;
  var page = Math.round(el.offsetLeft / window.innerWidth);
  document.documentElement.scrollLeft = 0; // undo the browser's native anchor scroll
  window.parent.postMessage({ kind: "ook-scroll", page: page }, "*");
}
window.addEventListener("load", reportFragmentPage);
window.addEventListener("hashchange", reportFragmentPage);
```

`offsetLeft` is a layout value, so the `translateX` on `body` doesn't distort it —
same as the current implementation.

Rust side:

- `const FRAGMENT_SCROLL_JS: &str = include_str!("./assets/fragment-scroll.js");`
- In `render_document_url`, inject it alongside the other
  snippets — **unconditionally**, since it no-ops without a hash. `inject_fragment_scroll`
  is deleted, and so is the `match fragment` around the injection steps.
- Append the hash to the finished data URL instead:
  `Some(f) => format!("{url}#{encoded}")`, `None => url`.
- Encode with `percent_encoding::utf8_percent_encode` (already a dependency, used at
  `src/epub.rs:114`) against a fragment-safe set, matching the `decodeURIComponent`
  on the JS side.

## Loose end worth fixing in the same pass

`resolve_internal_link` splits the fragment off the **raw** href
(`src/epub.rs:99-102`), so `LinkTarget.fragment` still holds it percent-*encoded*,
while the path right below it gets decoded. An href like `#s%20a` therefore looks up
the literal id `s%20a` today — a latent bug, rare in real books. If you encode into
the hash and `decodeURIComponent` in JS without addressing this, you'd double-encode
it.

Cleanest is to decode the fragment in `resolve_internal_link` the same way the path
is, so `LinkTarget.fragment` always holds a real element id. Then
encode-into-hash → decode-in-JS is a clean round trip.

## Alternatives considered

- **Placeholder + `.replace`.** Keeps the current data flow, moves the file. Loses
  compile-time checking of the placeholder and keeps the injection hole. Simplest
  diff, weakest result.
- **Inline `var OOK_FRAGMENT = …;` next to a static asset.** Logic lives in the
  asset; only the *data* stays inline, escaped properly via `serde_json::to_string`.
  Safe, but leaves a second injection site in `epub.rs` and doesn't get the
  no-reload benefit, since the body still changes with the fragment.
- **An `ook-goto-fragment` postMessage** — the option floated in
  [`performance-review-2026-07.md`](../performance-review-2026-07.md) under "Fragment links reload the
  whole chapter". It reaches the same no-reload outcome through the existing bridge
  rather than the URL. Heavier: the fragment has to be delivered *after* the iframe
  is ready, so it needs the same load-ordering handling that `ook-set-page` already
  deals with (`src/ui/reader.rs:66-81`). The hash approach gets the ordering for
  free, because the hash is present before the document parses.

## As shipped

Landed in `30e4b0c`, essentially as sketched. What differs, and where it lives now:

- **One concatenated asset blob, not five injection calls.** `2a2b5f8` had already
  consolidated the injected snippets: `src/web/assets.rs` builds a single
  `INJECTED_ASSETS` const at compile time via the `wrap_css!` / `wrap_js!` macros
  (`include_str!` inside `concat!`, each wrapped in its CDATA-guarded tag), and
  `fragment-scroll.js` simply joined the list. So there is no per-snippet
  `get_wrapped_js` call — `render_document_url` (`src/epub.rs:118-130`) does one
  `insert_before_head_close` and then appends the hash. `inject_fragment_scroll` and
  the `match fragment` around the injection steps are gone.
- **Assets moved with the code.** `1643d7e` relocated the injected files from
  `assets/reader/` to `src/web/assets/`, next to the module that includes them.
  `ook-events-listener.js` (the *parent*-side bridge, `dioxus.send(…)`) also came out
  of a string literal in `src/ui/reader.rs:11`, though it is `document::eval`'d rather
  than injected.
- **Encoding round trip.** `render_document_url` encodes with
  `utf8_percent_encode(frag, NON_ALPHANUMERIC)`; the asset decodes with
  `decodeURIComponent`.
- **The loose end was fixed in the same pass.** `resolve_internal_link`
  (`src/epub.rs:77-112`) now percent-decodes the fragment the same way it decodes the
  path, so `LinkTarget.fragment` always holds a real element id.
- **Tests.** `fragment_rides_in_the_url_hash_not_the_document` landed verbatim from
  this doc. The rewritten assertion test is `fragment_scroll_asset_reacts_to_hash_changes`
  and asserts against `INJECTED_ASSETS` rather than a per-snippet wrapper. The decode
  fix added `resolves_a_percent_encoded_href_to_a_decoded_target` and
  `resolves_a_bare_fragment_against_the_current_chapter`.
- **Not done here:** the sideways-shift check under `dx serve` is worth re-eyeballing
  on an image-heavy book — `scrollLeft = 0` handles the horizontal case the script
  causes, but the sketch's warning about stacking on the `--ook-page` transform was
  never formally verified.
