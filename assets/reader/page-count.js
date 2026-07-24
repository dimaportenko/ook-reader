const report = function () {
  var count = Math.max(
    1,
    Math.ceil(document.body.scrollWidth / window.innerWidth),
  );
  window.parent.postMessage({ kind: "ook-pages", count: count }, "*");
};
window.addEventListener("load", report);
window.addEventListener("resize", report);
