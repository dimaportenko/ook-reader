window.addEventListener("message", function (e) {
  if (!e.data || e.data.kind !== "ook-set-theme") {
    return;
  }

  reanchor(function () {
    for (const [name, value] of e.data.vars) {
      if (value) {
        document.documentElement.style.setProperty(name, value);
      } else {
        document.documentElement.style.removeProperty(name);
      }
    }
  });
});
