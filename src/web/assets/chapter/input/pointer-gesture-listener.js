let swipeFrom = null;
let pendingTap = null;

const LONG_PRESS_MS = 500;
const DOUBLE_TAP_MS = 300;
const DOUBLE_TAP_DISTANCE_PX = 24;

document.addEventListener("pointerdown", function (e) {
  const selection = window.getSelection();
  const selectedAtStart = !!selection && !selection.isCollapsed;
  const isDoubleTap =
    pendingTap &&
    e.timeStamp - pendingTap.at <= DOUBLE_TAP_MS &&
    Math.hypot(e.clientX - pendingTap.x, e.clientY - pendingTap.y) <=
      DOUBLE_TAP_DISTANCE_PX;

  if (isDoubleTap) {
    clearTimeout(pendingTap.timer);
    pendingTap = null;
  }

  swipeFrom = {
    id: e.pointerId,
    x: e.clientX,
    y: e.clientY,
    at: e.timeStamp,
    moved: false,
    selectedAtStart,
    isDoubleTap,
  };
});

document.addEventListener("pointermove", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  if (
    Math.abs(e.clientX !== swipeFrom.x) > 2 ||
    Math.abs(e.clientY - swipeFrom.y) > 2
  ) {
    swipeFrom.moved = true;
  }
});

document.addEventListener("pointerup", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  const dx = Math.round(e.clientX - swipeFrom.x);
  const dy = Math.round(e.clientY - swipeFrom.y);
  const moved = swipeFrom.moved;
  const duration = e.timeStamp - swipeFrom.at;
  const selectedAtStart = swipeFrom.selectedAtStart;
  const isDoubleTap = swipeFrom.isDoubleTap;
  swipeFrom = null;
  const selection = window.getSelection();
  const selected = !!selection && !selection.isCollapsed;
  if (
    !moved &&
    !selectedAtStart &&
    !selected &&
    !isDoubleTap &&
    duration < LONG_PRESS_MS
  ) {
    const tap = { x: e.clientX, y: e.clientY, at: e.timeStamp };
    tap.timer = setTimeout(function () {
      if (pendingTap === tap) {
        pendingTap = null;
        window.parent.postMessage({ kind: "ook-tap" }, "*");
      }
    }, DOUBLE_TAP_MS);
    pendingTap = tap;
    return;
  }
  if (dx === 0 && dy === 0) return;
  window.parent.postMessage({ kind: "ook-swipe", dx, dy, selected }, "*");
});

document.addEventListener("pointercancel", function (e) {
  if (swipeFrom && e.pointerId === swipeFrom.id) {
    swipeFrom = null;
  }
});
