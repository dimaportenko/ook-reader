function pageOf(el) {
  return Math.round(el.offsetLeft / window.innerWidth);
}

// The page we are on right now, read back from the variable pagination.css
// paginates by. `page-listener.js` is the only thing that writes it.
function currentPage() {
  const style = getComputedStyle(document.documentElement);
  return Number(style.getPropertyValue("--ook-page")) || 0;
}
