import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { snapBottomRight } from "./snap.js";   // the helper above

const list   = document.getElementById("list");
const filter = document.getElementById("filter");
let   items  = [];               // local history

function render() {
  list.innerHTML = "";
  const needle = filter.value.toLowerCase();
  items
    .slice()                       // copy
    .reverse()                     // newest first
    .filter(e => {
      if (!needle) return true;
      return JSON.stringify(e.content).toLowerCase().includes(needle);
    })
    .forEach(addLi);
}

function addLi(entry) {
  const li = document.createElement("li");

  // preview
  if (entry.content.Text) {
    const span = document.createElement("span");
    span.className = "text";
    span.textContent = entry.content.Text.replace(/\s+/g, " ").trim();
    li.append(span);
  } else {
    const img = document.createElement("img");
    img.src = `data:image/png;base64,${entry.content.ImageBase64}`;
    li.append(img);
  }

  // click -> restore
  li.onclick = () => invoke("restore_clip", { entry });

  list.append(li);
}

/* ---------- bootstrap ---------- */
filter.oninput = render;
snapBottomRight();

// initial history
invoke("get_history").then(hist => {
  items = hist;
  render();
});

// live updates
listen("clip", evt => {
  items.push(evt.payload);
  render();
});
