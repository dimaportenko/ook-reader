// The page we are on right now, read back from the variable pagination.css
// paginates by. `page-listener.js` is the only thing that writes it.
function currentPage() {
  const style = getComputedStyle(document.documentElement);
  return Number(style.getPropertyValue("--ook-page")) || 0;
}

function reportFragmentPage() {
  const id = decodeURIComponent(location.hash.slice(1));
  if (!id) return; // no hash → inert, so inject unconditionally

  const el = document.getElementById(id);
  // A broken internal link — the fragment names an id this document does not
  // have. Report the page we are already on rather than staying silent: this
  // message is also what clears `pending_fragment` on the Rust side, and a
  // fragment left pending gets re-applied to whatever chapter comes next.
  const page = el ? Math.round(el.offsetLeft / window.innerWidth) : currentPage();

  document.documentElement.scrollLeft = 0; // undo the browser's native anchor scroll
  window.parent.postMessage({ kind: "ook-scroll", page: page }, "*");
}

window.addEventListener("load", reportFragmentPage);
window.addEventListener("hashchange", reportFragmentPage);
