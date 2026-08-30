document.addEventListener("pointerdown", function () {
  window.parent.postMessage({ kind: "ook-pointerdown" }, "*");
});
