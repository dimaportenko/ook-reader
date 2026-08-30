const vars = await dioxus.recv();
const frame = document.getElementById("reader-frame");

frame?.contentWindow?.postMessage({ kind: "ook-set-theme", vars }, "*");
