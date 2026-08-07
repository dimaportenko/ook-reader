window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-theme") {
    return;
  }
  for (const [name, value] of e.data.vars) {
    document.documentElement.style.setProperty(name, value);
  }
});
