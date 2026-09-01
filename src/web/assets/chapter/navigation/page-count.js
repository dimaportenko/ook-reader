const report = function () {
  const count = Math.max(
    1,
    Math.ceil(document.body.scrollWidth / window.innerWidth),
  );
  window.ookPageCount = count;
  window.parent.postMessage({ kind: "ook-pages", count: count }, "*");
};
