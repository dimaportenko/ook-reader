window.addEventListener("message", (e) => {
  if (!e.data) return;
  if (e.data.kind === "ook-link") {
    dioxus.send("link:" + e.data.raw);
  }
  if (e.data.kind === "ook-scroll") {
    dioxus.send("scroll:" + e.data.page);
  }
  if (e.data.kind === "ook-pages") {
    dioxus.send("pages:" + e.data.count);
  }
  if (e.data.kind === "ook-position") {
    dioxus.send("position:" + e.data.selector);
  }
  if (e.data.kind === "ook-reflow") {
    dioxus.send("reflow:" + e.data.page);
  }
  if (e.data.kind === "ook-key") {
    dioxus.send("key:" + e.data.key);
  }
  if (e.data.kind === "ook-swipe") {
    dioxus.send("swipe:" + e.data.dx + "," + e.data.dy);
  }
  if (e.data.kind === "ook-pointerdown") {
    const frame = document.getElementById("reader-frame");
    if (frame) {
      frame.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    }
  }
  if (e.data.kind === "ook-ready") {
    dioxus.send("ready:");
  }
  if (e.data.kind === "ook-warn") {
    dioxus.send("warn:" + e.data.message);
  }
});
