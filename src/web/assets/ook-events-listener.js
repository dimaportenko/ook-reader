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
});
