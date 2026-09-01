window.__ookReader?.destroy();
window.__ookReaderSend = null;
if (window.__ookFrameBridgeListener) {
  window.removeEventListener("message", window.__ookFrameBridgeListener);
  window.__ookFrameBridgeListener = null;
}
