document.addEventListener("click", function (e) {
  const a = e.target.closest && e.target.closest("a[href]");
  if (!a) return;
  e.preventDefault();
  window.parent.postMessage(
    { kind: "ook-link", raw: a.getAttribute("href") },
    "*",
  );
});
