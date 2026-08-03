function selectorFor(el) {
  const parts = [];
  while (el && el !== document.body) {
    const parent = el.parentElement;
    if (!parent) {
      return null;
    }

    const index = Array.prototype.indexOf.call(parent.children, el) + 1;
    parts.unshift(`${el.localName}:nth-child(${index})`);
    el = parent;
  }

  return ["body", ...parts].join(" > ");
}

function firstElementOnPage(page) {
  for (const el of document.body.getElementsByTagName("*")) {
    if (!el.getClientRects().length) {
      continue;
    }
    if (pageOf(el) === page) {
      return el;
    }
  }
  return null;
}

function reportPosition(page) {
  const el = firstElementOnPage(page);
  if (!el) {
    return;
  }

  const selector = selectorFor(el);
  if (!selector) {
    return;
  }

  window.parent.postMessage({ kind: "ook-position", selector }, "*");
}

window.addEventListener("load", () => reportPosition(currentPage()));
window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-page") {
    return;
  }
  reportPosition(e.data.page);
});
