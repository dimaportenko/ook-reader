window.addEventListener("load", function () {
  whenSettled(function () {
    report();
    reportFragmentPage();
    if (!location.hash) {
      reportPosition(currentPage());
    }
    window.parent.postMessage({ kind: "ook-ready" }, "*");
  });
});
