if (!window.ookGlassAngle) {
  window.ookGlassAngle = true;

  const root = document.documentElement;
  let queued = false;
  let x = 0;
  let y = 0;

  document.addEventListener("pointermove", function (e) {
    x = e.clientX;
    y = e.clientY;
    if (queued) return;
    queued = true;
    requestAnimationFrame(function () {
      queued = false;
      const dx = x - window.innerWidth / 2;
      const dy = y - window.innerHeight / 2;
      const deg = Math.atan2(dy, dx) * (180 / Math.PI) + 90;
      root.style.setProperty("--glass-angle", Math.round(deg) + "deg");
    });
  });
}
