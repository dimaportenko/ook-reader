const SETTLE_TIMEOUT_MS = 2000;

function whenSettled(fn) {
  const fonts = document.fonts;
  if (!fonts || !fonts.ready) {
    fn();
    return;
  }

  let done = false;
  const once = function (timedOut) {
    if (done) return;
    done = true;
    if (timedOut) {
      ookWarn(
        `fonts unfinished after ${SETTLE_TIMEOUT_MS}ms, measuring anyway ${ookGeom()}`,
      );
    }
    fn();
  };

  fonts.ready.then(() => once(false));
  window.setTimeout(() => once(true), SETTLE_TIMEOUT_MS);
}
