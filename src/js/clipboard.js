// ─────────────────────────────────────────────────────────────
// JOR — Clipboard Window Controller
// Searchable clipboard history with keyboard navigation and
// one-click paste. Debounced, race-guarded, vanilla JS.
// ─────────────────────────────────────────────────────────────

import { getIcon, escapeHtml, debounce } from "./shared.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const input = document.getElementById("search-input");
const resultsList = document.getElementById("results");
const clearBtn = document.getElementById("clear-btn");

// Cross-window navigation: gear icon opens the settings dashboard.
document.getElementById("open-settings")?.addEventListener("click", () => {
  invoke("show_window", { label: "settings" });
});

let selectedIndex = 0;
let entries = [];
let requestSeq = 0;
let prevListEmpty = true; // Gates the entrance animation to fresh lists only

// All clipboard entries use EntryKind 8 (Clipboard) → clipboard icon.
const performSearch = debounce(async () => {
  const reqId = ++requestSeq;
  const query = input.value;
  try {
    const result = await invoke("search", { query, mode: "clipboard" });
    if (reqId !== requestSeq) return;
    entries = result;
    selectedIndex = 0;
    renderResults();
  } catch (err) {
    console.error("Clipboard search error:", err);
  }
}, 40);

function renderResults() {
  resultsList.innerHTML = "";

  if (entries.length === 0) {
    resultsList.innerHTML = `<div class="empty">No history found.</div>`;
    prevListEmpty = true;
    return;
  }

  // Animate only when the list was empty before (window open), not on every
  // keystroke re-render.
  const animate = prevListEmpty;
  prevListEmpty = false;

  const fragment = document.createDocumentFragment();
  entries.forEach((res, i) => {
    const li = document.createElement("li");
    li.className = "item" + (animate ? " animate" : "") + (i === selectedIndex ? " active" : "");

    const icon = document.createElement("div");
    icon.className = "item-icon";
    icon.innerHTML = getIcon(res.kind);

    const text = document.createElement("div");
    text.className = "item-text";
    text.innerHTML = `
      <div class="item-name">${escapeHtml(res.name)}</div>
      <div class="item-sub">${escapeHtml(res.subtitle)}</div>
    `;

    li.appendChild(icon);
    li.appendChild(text);
    li.addEventListener("click", () => executeEntry(res));
    fragment.appendChild(li);
  });

  resultsList.appendChild(fragment);

  const activeItem = resultsList.querySelector(".active");
  if (activeItem) activeItem.scrollIntoView({ block: "nearest" });
}

async function executeEntry(entry) {
  try {
    await invoke("launch", { entry });
  } catch (err) {
    console.error("Paste failed:", err);
  }
}

// ── Event Listeners ─────────────────────────────────────────

input.addEventListener("input", () => {
  selectedIndex = 0;
  performSearch();
});

clearBtn.addEventListener("click", async () => {
  if (confirm("Clear all clipboard history?")) {
    await invoke("clear_clipboard_history");
    performSearch();
  }
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    invoke("hide_window");
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (entries.length > 0) {
      selectedIndex = (selectedIndex + 1) % entries.length;
      renderResults();
    }
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (entries.length > 0) {
      selectedIndex = (selectedIndex - 1 + entries.length) % entries.length;
      renderResults();
    }
  } else if (e.key === "Enter") {
    e.preventDefault();
    if (entries[selectedIndex]) executeEntry(entries[selectedIndex]);
  }
});

// Refocus + refresh whenever the window gains focus.
listen("tauri://focus", () => {
  input.value = "";
  performSearch();
  input.focus();
});

performSearch();
