// The reader is unmounting. Revoke the chapter blob it left behind.
//
// Nothing else would: a blob URL lives until it is revoked or until the
// document that created it unloads, and the document here is the app shell,
// which survives closing a book. Without this, every book you open and close
// leaves one chapter's bytes resident for the life of the process.
if (window.__ookBlobUrl) {
  URL.revokeObjectURL(window.__ookBlobUrl);
  window.__ookBlobUrl = null;
}
