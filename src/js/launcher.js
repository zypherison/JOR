// ─────────────────────────────────────────────────────────────
// JOR — Launcher Window Controller
// Real-time search with debounce + stale-response guarding,
// keyboard navigation, Tab autocomplete, math evaluation, web
// search, directory browsing, real app icons, and theme sync.
// Pure vanilla JS talking to Tauri via window.__TAURI__.
// ─────────────────────────────────────────────────────────────

import { getIcon, getTypeLabel, escapeHtml, applyTheme } from "./shared.js";

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { listen } = window.__TAURI__.event;

// ── DOM References ──────────────────────────────────────────

const input = document.getElementById("search-input");
const results = document.getElementById("results");
const refreshModal = document.getElementById("refresh-modal");

// ── State ───────────────────────────────────────────────────

let entries = [];          // Current result set
let selectedIndex = 0;     // Active selection index
let isExploring = false;   // True while in directory browse mode
let currentMode = "standard"; // standard | clipboard
let searchTimer = null;
let requestSeq = 0;        // Guards against out-of-order async responses

const iconCache = new Map();
const pendingIcons = new Set();

// ── Theme Syncing ───────────────────────────────────────────

listen("theme-changed", (event) => applyTheme(event.payload));

(async () => {
  try {
    const settings = await invoke("get_settings");
    applyTheme(settings.theme);
  } catch (_) {
    // Keep the CSS defaults if settings can't be loaded yet.
  }
})();

// ── Real App Icons ──────────────────────────────────────────

async function loadRealIcon(path, container) {
  if (iconCache.has(path)) {
    container.innerHTML = `<img src="${iconCache.get(path)}" alt="">`;
    return;
  }
  if (pendingIcons.has(path)) return;
  pendingIcons.add(path);

  try {
    const b64 = await invoke("get_app_icon", { path });
    if (b64) {
      iconCache.set(path, b64);
      if (container.isConnected) {
        container.innerHTML = `<img src="${b64}" alt="">`;
      }
    }
  } catch (_) {
    // Keep the default SVG icon.
  } finally {
    pendingIcons.delete(path);
  }
}

// ── Math Evaluation (client-side, zero cost) ────────────────

function evaluateMath(query) {
  if (/^[\d\+\-\*\/\(\)\.\s\%]+$/.test(query) && /[\+\-\*\/\%]/.test(query)) {
    try {
      const result = new Function(`return (${query})`)();
      if (result !== undefined && isFinite(result)) {
        return {
          name: `= ${result}`,
          name_lower: "",
          path: String(result),
          subtitle: query,
          kind: 5,
          score: 1000,
        };
      }
    } catch (_) {}
  }
  return null;
}

// ── Web Search (explicit or fallback) ───────────────────────

function webSearchEntry(query, isExplicit) {
  const clean = isExplicit ? query.substring(2).trim() : query.trim();
  if (!clean) return null;
  return {
    name: `Search "${clean}" on Google`,
    name_lower: "",
    path: `https://google.com/search?q=${encodeURIComponent(clean)}`,
    subtitle: "google.com",
    kind: 4,
    score: isExplicit ? 900 : 0,
  };
}

// ── Core: Perform Search ────────────────────────────────────
// Debounced on every keystroke. Each scheduled search gets a
// sequence id so stale responses are discarded.

function scheduleSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  const delay = input.value.trim().length <= 1 ? 0 : 55;
  const reqId = ++requestSeq;
  searchTimer = setTimeout(() => performSearch(reqId), delay);
}

async function performSearch(reqId = ++requestSeq) {
  const query = input.value;
  selectedIndex = 0;

  try {
    let extra = [];

    // Check if user is browsing a path
    isExploring = /^[a-zA-Z]:[\\\/]/.test(query) || query.startsWith("~/") || query.startsWith("/");

    if (isExploring) {
      const directoryEntries = await invoke("list_directory", { path: query });
      if (reqId !== requestSeq) return;
      entries = directoryEntries;
      renderResults();
      return;
    }

    // Client-side math
    const math = evaluateMath(query);
    if (math) extra.push(math);

    // Explicit web search (prefix: "g " or "? ")
    if (query.startsWith("g ") || query.startsWith("? ")) {
      const ws = webSearchEntry(query, true);
      if (ws) extra.push(ws);
    }

    // Backend fuzzy search + plugins
    const backend = await invoke("search", { query, mode: currentMode });
    if (reqId !== requestSeq) return;

    entries = [...extra, ...backend];

    // Fallback: if nothing found, offer a Google search
    if (entries.length === 0 && query.trim().length > 0) {
      const ws = webSearchEntry(query, false);
      if (ws) entries.push(ws);
    }

    renderResults();
  } catch (err) {
    console.error("Search error:", err);
  }
}

// ── Core: Render Results ────────────────────────────────────

function renderResults() {
  results.innerHTML = "";

  if (entries.length === 0) {
    if (input.value.length > 0) {
      results.innerHTML = `<li class="empty">No results found</li>`;
    }
    return;
  }

  entries.forEach((entry, i) => {
    const li = document.createElement("li");
    li.className = `item${i === selectedIndex ? " active" : ""}`;

    // Badge text
    let badge = "";
    if (i === selectedIndex) {
      badge = entry.kind === 5 ? "copy" : "open";
    }

    // Subtitle: use entry's subtitle, or fallback to type label
    const sub = entry.subtitle || getTypeLabel(entry.kind);

    const iconContainer = document.createElement("div");
    iconContainer.className = "item-icon";
    iconContainer.innerHTML = getIcon(entry.kind);

    // Load real app icons for apps, files, and system entries
    if ((entry.kind === 0 || entry.kind === 1 || entry.kind === 3) && entry.path) {
      loadRealIcon(entry.path, iconContainer);
    }

    const textContainer = document.createElement("div");
    textContainer.className = "item-text";
    textContainer.innerHTML = `
      <div class="item-name">${escapeHtml(entry.name)}</div>
      <div class="item-sub">${escapeHtml(sub)}</div>
    `;

    li.appendChild(iconContainer);
    li.appendChild(textContainer);

    if (badge) {
      const badgeSpan = document.createElement("span");
      badgeSpan.className = "item-badge";
      badgeSpan.innerText = badge;
      li.appendChild(badgeSpan);
    }

    li.addEventListener("click", () => {
      selectedIndex = i;
      launchEntry(entry);
    });

    results.appendChild(li);
  });

  // Scroll active item into view
  const active = results.querySelector(".active");
  if (active) active.scrollIntoView({ block: "nearest", behavior: "smooth" });
}

// ── Core: Launch Entry ──────────────────────────────────────

async function launchEntry(entry) {
  try {
    if (entry.kind === 5) {
      // Math result → copy to clipboard
      await navigator.clipboard.writeText(entry.path);
    } else if (entry.kind === 2 && isExploring) {
      // Folder in explore mode → drill down
      let p = entry.path;
      if (!p.endsWith("\\") && !p.endsWith("/")) p += "\\";
      input.value = p;
      scheduleSearch();
      return;
    } else {
      // Everything else → dispatch to Rust
      await invoke("launch", { entry });
    }

    // Reset after action
    input.value = "";
    entries = [];
    renderResults();
  } catch (err) {
    console.error("Launch error:", err);
  }
}

// ── Tab Autocomplete ────────────────────────────────────────
// If the top result is a folder or app, Tab fills the input
// with its path for quick drilling / refinement.

function handleTab() {
  if (entries.length === 0) return;
  const top = entries[selectedIndex];

  if (top.kind === 2) {
    // Folder: drill into it
    let p = top.path;
    if (!p.endsWith("\\") && !p.endsWith("/")) p += "\\";
    input.value = p;
    isExploring = true;
    scheduleSearch();
  } else {
    // Anything else: autocomplete the name
    input.value = top.name;
    scheduleSearch();
  }
}

// ── Data Refresh (factory reset) ────────────────────────────

function checkRefresh(query) {
  const keywords = ["refresh jor", "reset jor", "factory reset"];
  if (keywords.some((k) => query.toLowerCase() === k)) {
    refreshModal.style.display = "flex";
  }
}

function closeRefresh() {
  refreshModal.style.display = "none";
  input.value = "";
}

async function handleRefresh() {
  await invoke("refresh_jor_data");
}

// Expose for inline onclick handlers in index.html
window.closeRefresh = closeRefresh;
window.handleRefresh = handleRefresh;

// ── Event Listeners ─────────────────────────────────────────

window.addEventListener("DOMContentLoaded", async () => {
  const win = getCurrentWindow();

  // Mode switches come from the Rust side (launcher vs clipboard focus).
  await listen("switch-mode", (event) => {
    currentMode = event.payload;
    input.placeholder = currentMode === "clipboard"
      ? "Search Clipboard..."
      : "Search apps, files, and more...";
    input.value = "";
    performSearch(++requestSeq);
  });

  // On focus: reset + refresh
  win.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      input.value = "";
      isExploring = false;
      performSearch(++requestSeq);
      setTimeout(() => input.focus(), 10);
    }
  });

  // Real-time search on every keystroke
  input.addEventListener("input", () => {
    selectedIndex = 0;
    checkRefresh(input.value);
    scheduleSearch();
  });

  // Keyboard navigation
  window.addEventListener("keydown", (e) => {
    // Prevent the Windows system menu from Alt+Space while JOR is focused
    if (e.altKey && e.code === "Space") {
      e.preventDefault();
      return;
    }

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (entries.length > 0) {
          selectedIndex = (selectedIndex + 1) % entries.length;
          renderResults();
        }
        break;

      case "ArrowUp":
        e.preventDefault();
        if (entries.length > 0) {
          selectedIndex = (selectedIndex - 1 + entries.length) % entries.length;
          renderResults();
        }
        break;

      case "Enter":
        e.preventDefault();
        if (entries.length > 0 && entries[selectedIndex]) {
          launchEntry(entries[selectedIndex]);
        }
        break;

      case "Tab":
        e.preventDefault();
        handleTab();
        break;

      case "Escape":
        e.preventDefault();
        if (input.value.length > 0) {
          // First Escape clears the input
          input.value = "";
          isExploring = false;
          performSearch(++requestSeq);
        } else {
          // Second Escape hides the window
          invoke("hide_window");
        }
        break;

      case "Backspace":
        // In explore mode, Backspace on empty input goes up one dir
        if (isExploring && input.value.endsWith("\\")) {
          e.preventDefault();
          const parts = input.value.slice(0, -1).split("\\");
          parts.pop();
          if (parts.length > 0) {
            input.value = parts.join("\\") + "\\";
            scheduleSearch();
          }
        }
        break;
    }
  });

  // Initial state
  input.focus();
  performSearch(++requestSeq);
});
