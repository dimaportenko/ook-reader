let swipeFrom = null;
let pendingTap = null;
let dragX = 0;
let dragFrame = null;
let wheelX = 0;
let wheelTimer = null;
let wheelHandled = false;

const LONG_PRESS_MS = 500;
const DOUBLE_TAP_MS = 300;
const DOUBLE_TAP_DISTANCE_PX = 24;
const DRAG_SLOP_PX = 6;

function paintDrag(x) {
  dragX = x;
  if (dragFrame !== null) return;
  dragFrame = requestAnimationFrame(function () {
    document.documentElement.style.setProperty("--ook-drag-x", `${dragX}px`);
    dragFrame = null;
  });
}

function finishLocalDrag() {
  document.documentElement.classList.add("ook-page-settling");
  paintDrag(0);
  window.setTimeout(function () {
    document.documentElement.classList.remove("ook-page-settling");
  }, 260);
}

function settlePage(page, animate) {
  const before = currentPage();
  const root = document.documentElement;
  if (!animate) {
    root.classList.remove("ook-page-settling");
    root.style.setProperty("--ook-page", page);
    paintDrag(0);
    return;
  }

  const preserved = dragX + (page - before) * window.innerWidth;
  root.style.setProperty("--ook-page", page);
  root.style.setProperty("--ook-drag-x", `${preserved}px`);
  dragX = preserved;
  void document.body.offsetWidth;
  root.classList.add("ook-page-settling");
  paintDrag(0);
  window.setTimeout(function () {
    root.classList.remove("ook-page-settling");
  }, 260);
}

function isBoundaryDrag(dx) {
  const page = currentPage();
  const count = window.ookPageCount || 1;
  return (dx > 0 && page === 0) || (dx < 0 && page + 1 >= count);
}

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
    horizontal: false,
    boundary: false,
    swipeEnabled: e.pointerType !== "mouse",
    selectedAtStart,
    isDoubleTap,
  };
  if (swipeFrom.swipeEnabled) e.target.setPointerCapture?.(e.pointerId);
});

document.addEventListener("pointermove", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  const dx = e.clientX - swipeFrom.x;
  const dy = e.clientY - swipeFrom.y;
  if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
    swipeFrom.moved = true;
  }
  if (!swipeFrom.swipeEnabled) return;
  if (!swipeFrom.horizontal) {
    if (Math.abs(dx) < DRAG_SLOP_PX || Math.abs(dx) <= Math.abs(dy)) return;
    if (swipeFrom.selectedAtStart) return;
    swipeFrom.horizontal = true;
  }

  swipeFrom.boundary = isBoundaryDrag(dx);
  if (swipeFrom.boundary) {
    paintDrag(0);
    window.parent.postMessage({ kind: "ook-drag", dx }, "*");
  } else {
    paintDrag(dx);
  }
});

document.addEventListener("pointerup", function (e) {
  if (!swipeFrom || e.pointerId !== swipeFrom.id) return;
  const dx = Math.round(e.clientX - swipeFrom.x);
  const dy = Math.round(e.clientY - swipeFrom.y);
  const moved = swipeFrom.moved;
  const boundary = swipeFrom.boundary;
  const swipeEnabled = swipeFrom.swipeEnabled;
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
  if (!swipeEnabled) return;
  if (selected || Math.abs(dx) < 40 || Math.abs(dx) <= Math.abs(dy)) {
    if (boundary) {
      window.parent.postMessage({ kind: "ook-drag-cancel" }, "*");
    } else {
      finishLocalDrag();
    }
  }
  window.parent.postMessage({ kind: "ook-swipe", dx, dy, selected }, "*");
});

document.addEventListener("pointercancel", function (e) {
  if (swipeFrom && e.pointerId === swipeFrom.id) {
    if (!swipeFrom.swipeEnabled) {
      swipeFrom = null;
      return;
    } else if (swipeFrom.boundary) {
      window.parent.postMessage({ kind: "ook-drag-cancel" }, "*");
    } else {
      finishLocalDrag();
    }
    swipeFrom = null;
  }
});

window.addEventListener("message", function (e) {
  if (e.data?.kind === "ook-cancel-swipe") finishLocalDrag();
});

document.addEventListener(
  "wheel",
  function (e) {
    if (!window.matchMedia("(hover: hover) and (pointer: fine)").matches) return;
    if (Math.abs(e.deltaX) <= Math.abs(e.deltaY)) return;
    e.preventDefault();

    const scale = e.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? 1 : 16;
    wheelX += e.deltaX * scale;
    window.clearTimeout(wheelTimer);
    wheelTimer = window.setTimeout(function () {
      wheelX = 0;
      wheelHandled = false;
    }, 160);

    if (wheelHandled || Math.abs(wheelX) < 40) return;
    wheelHandled = true;
    const dx = wheelX > 0 ? -40 : 40;
    window.parent.postMessage({ kind: "ook-swipe", dx, dy: 0, selected: false }, "*");
  },
  { passive: false },
);
