const RESIZE_SETTLE_MS = 150;

let lastAnchor = null;
let resizeTimer = null;
let pendingReflowPage = null;

function rememberAnchor(selector) {
  lastAnchor = selector;
}

function isReflowEcho(page) {
  if (pendingReflowPage === null) {
    return false;
  }

  const echo = pendingReflowPage === page;
  pendingReflowPage = null;
  return echo;
}

function reflowFrom(anchor, before) {
  report();

  const moved = anchor && document.querySelector(anchor);
  if (!moved) {
    ookWarn(`anchor did not survive the reflow from page ${before}: ${anchor}`);
    return;
  }

  const page = pageOf(moved);
  if (page !== before) {
    pendingReflowPage = page;
    window.parent.postMessage({ kind: "ook-reflow", page }, "*");
  }
}

function reanchor(mutate) {
  const before = currentPage();
  const el = firstElementOnPage(before);
  const anchor = el && selectorFor(el);

  mutate();

  reflowFrom(anchor, before);
}

window.addEventListener("resize", function () {
  window.clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(function () {
    reflowFrom(lastAnchor, currentPage());
  }, RESIZE_SETTLE_MS);
});
