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
    ookWarn(`no element on page ${page}, position not saved`);
    return;
  }

  const selector = selectorFor(el);
  if (!selector) {
    ookWarn(`no selector for <${el.localName}> on page ${page}, position not saved`);
    return;
  }

  rememberAnchor(selector);
  window.parent.postMessage({ kind: "ook-position", selector }, "*");
}

window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-page") {
    return;
  }
  if (isReflowEcho(e.data.page)) {
    return;
  }
  reportPosition(e.data.page);
});
