window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-page") {
    return;
  }
  document.documentElement.style.setProperty("--ook-page", e.data.page);
});
