/**
 * Handle ook-set-page event
 */
window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-page") {
    return;
  }
  if (typeof settlePage === "function") {
    settlePage(e.data.page, e.data.animate === true);
  } else {
    document.documentElement.style.setProperty("--ook-page", e.data.page);
  }
});
