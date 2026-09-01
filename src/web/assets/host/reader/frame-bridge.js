window.__ookReaderSend = function (message) {
  dioxus.send(message);
};

if (window.__ookFrameBridgeListener) {
  window.removeEventListener("message", window.__ookFrameBridgeListener);
}

window.__ookFrameBridgeListener = function (e) {
  window.__ookReader?.handleMessage(e.source, e.data);
};

window.addEventListener("message", window.__ookFrameBridgeListener);
