const [url, fragment] = await dioxus.recv();
const frame = document.getElementById("reader-frame");

// Same chapter as the one on screen? Then only the hash can have changed.
// Move it in place: a same-document navigation, so no refetch and no reload.
// `fragment-scroll.js` is already listening for `hashchange`.
if (frame.dataset.chapterUrl === url) {
  // `fragment` is null when `on_scroll` cleared it — that is not a navigation.
  if (fragment) {
    const win = frame.contentWindow;
    // Assigning the hash it already has fires no `hashchange`, so clear it
    // first; otherwise clicking the same anchor twice does nothing the second
    // time. The empty hash is inert — `reportFragmentPage` returns on no id.
    win.location.hash = "";
    win.location.hash = encodeURIComponent(fragment);
  }
  return;
}

// Claim the frame before awaiting, so a chapter change that starts while this
// fetch is in flight can tell us to stand down.
frame.dataset.pendingUrl = url;

const response = await fetch(url);
if (!response.ok) {
  console.error(`ook: ${response.status} loading ${url}`);
  return;
}

// .blob(), not .text(): no UTF-16 round trip, and the response's Content-Type
// rides along as the Blob's own type — which is what decides whether WebKit
// renders the frame or treats it as a download.
const blob = await response.blob();

// A newer load overwrote the claim while we were awaiting. Drop this one
// rather than fighting over the frame and showing the wrong chapter.
if (frame.dataset.pendingUrl !== url) {
  return;
}

// Without this the webview keeps every chapter you have visited.
//
// The live blob is parked on `window`, not on the frame's dataset like the two
// URLs above, because `blob-cleanup.js` has to reach it at unmount — by then
// the frame element may already be out of the DOM, and a dataset that goes with
// it takes the only handle on the blob along.
if (window.__ookBlobUrl) {
  URL.revokeObjectURL(window.__ookBlobUrl);
}

const next = URL.createObjectURL(blob);
window.__ookBlobUrl = next;
frame.dataset.chapterUrl = url;
frame.src = fragment ? `${next}#${encodeURIComponent(fragment)}` : next;
