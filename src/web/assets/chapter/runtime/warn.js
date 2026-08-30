function ookWarn(message) {
  window.parent.postMessage({ kind: "ook-warn", message: message }, "*");
}

function ookGeom() {
  const body = getComputedStyle(document.body);
  return [
    `w=${window.innerWidth}`,
    `h=${window.innerHeight}`,
    `col=${body.columnWidth}`,
    `sw=${document.body.scrollWidth}`,
    `fs=${body.fontSize}`,
    `lh=${body.lineHeight}`,
    `ff=${body.fontFamily.split(",")[0]}`,
  ].join(" ");
}
