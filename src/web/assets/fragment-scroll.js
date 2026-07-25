function reportFragmentPage() {
  const id = decodeURIComponent(location.hash.slice(1));
  if (!id) return; // no hash → inert, so inject unconditionally
  const el = document.getElementById(id);
  if (!el) return;
  const page = Math.round(el.offsetLeft / window.innerWidth);
  document.documentElement.scrollLeft = 0; // undo the browser's native anchor scroll
  window.parent.postMessage({ kind: "ook-scroll", page: page }, "*");
}
window.addEventListener("load", reportFragmentPage);
window.addEventListener("hashchange", reportFragmentPage);
