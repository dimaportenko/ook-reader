window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-theme") {
    return;
  }

  const before = currentPage();
  const anchorEl = firstElementOnPage(before);
  const anchor = anchorEl && selectorFor(anchorEl);

  for (const [name, value] of e.data.vars) {
    if (value) {
      document.documentElement.style.setProperty(name, value);
    } else {
      document.documentElement.style.removeProperty(name);
    }
  }

  report();

  const moved = anchor && document.querySelector(anchor);
  if (!moved) {
    return;
  }

  const page = pageOf(moved);
  if (page !== before) {
    window.parent.postMessage({ kind: "ook-reflow", page }, "*");
  }
});
