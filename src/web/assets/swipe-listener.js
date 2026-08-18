let swipeFrom = null;

document.addEventListener("pointerdown", function (e) {
  swipeFrom = { id: e.pointerId, x: e.clientX, y: e.clientY };
});

document.addEventListener("pointerup", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  const dx = Math.round(e.clientX - swipeFrom.x);
  const dy = Math.round(e.clientY - swipeFrom.y);
  swipeFrom = null;
  if (dx === 0 && dy === 0) return;
  window.parent.postMessage({ kind: "ook-swipe", dx, dy }, "*");
});
