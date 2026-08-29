const css = await dioxus.recv();

let style = document.getElementById("ook-theme");

if (!style) {
  style = document.createElement("style");
  style.id = "ook-theme";
  document.head.append(style);
}

style.textContent = css;
