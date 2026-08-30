document.addEventListener("keydown", function (e) {
  if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
  e.preventDefault();
  window.parent.postMessage({ kind: "ook-key", key: e.key }, "*");
});
