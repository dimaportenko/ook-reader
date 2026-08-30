const SELECTOR_PREFIX = "ook-sel:";

function elementFor(target) {
  if (!target.startsWith(SELECTOR_PREFIX)) {
    return document.getElementById(target);
  }

  try {
    return document.querySelector(target.slice(SELECTOR_PREFIX.length));
  } catch {
    return null;
  }
}

function reportFragmentPage() {
  const id = decodeURIComponent(location.hash.slice(1));
  if (!id) return; // no hash → inert, so inject unconditionally

  const el = elementFor(id);
  // A broken internal link — the fragment names an id this document does not
  // have. Report the page we are already on rather than staying silent: this
  // message is also what clears `Pending::Fragment` on the Rust side, and a
  // fragment left pending gets re-applied to whatever chapter comes next.
  const page = el ? pageOf(el) : currentPage();

  if (!el) {
    ookWarn(`fragment did not resolve, staying on page ${page}: ${id}`);
  }

  document.documentElement.scrollLeft = 0; // undo the browser's native anchor scroll
  window.parent.postMessage({ kind: "ook-scroll", page: page }, "*");
}

window.addEventListener("hashchange", reportFragmentPage);
