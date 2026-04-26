// ─────────────────────────────────────────────────────────────
// JOR — Frontend Controller
// Handles real-time search, keyboard navigation, Tab
// autocomplete, math evaluation, web search, and
// directory browsing. Zero dependencies — pure vanilla JS
// talking to Tauri via window.__TAURI__.
// ─────────────────────────────────────────────────────────────

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { listen } = window.__TAURI__.event;

// ── DOM References ──────────────────────────────────────────

const input   = document.getElementById("search-input");
const results = document.getElementById("results");

// ── State ───────────────────────────────────────────────────

let entries       = [];   // Current result set
let selectedIndex = 0;    // Active selection index
let isExploring   = false; // True when in directory browse mode
let currentMode   = "standard"; // standard, clipboard
let searchTimer   = null;
let requestSeq    = 0;

// ── SVG Icon Library (Feather-style, inline) ────────────────

const ICONS = {
  app:      `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1.5"></rect><rect x="14" y="3" width="7" height="7" rx="1.5"></rect><rect x="3" y="14" width="7" height="7" rx="1.5"></rect><rect x="14" y="14" width="7" height="7" rx="1.5"></rect></svg>`,
  file:     `<svg viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline></svg>`,
  folder:   `<svg viewBox="0 0 24 24"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
  system:   `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`,
  web:      `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>`,
  math:     `<svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`,
  workflow: `<svg viewBox="0 0 24 24"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>`,
  clipboard: `<svg viewBox="0 0 24 24"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>`,
};

// ── Utility: Get icon SVG by EntryKind ──────────────────────

function getIcon(kind) {
  const map = { 0: "app", 1: "file", 2: "folder", 3: "system", 4: "web", 5: "math", 6: "workflow", 7: "clipboard" };
  return ICONS[map[kind]] || ICONS.file;
}

// ── Utility: Get human-readable type label ──────────────────

function getTypeLabel(kind) {
  const labels = { 0: "Application", 1: "File", 2: "Folder", 3: "System", 4: "Web Search", 5: "Calculator", 6: "Workflow", 7: "Clipboard" };
  return labels[kind] || "Item";
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
// Called on every input event (real-time as-you-type).

function scheduleSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  const delay = input.value.trim().length <= 1 ? 0 : 55;
  const reqId = ++requestSeq;
  searchTimer = setTimeout(() => {
    performSearch(reqId);
  }, delay);
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

    // Standard-only features: Math and Explicit Web Search
    // Math and Explicit Web Search
    const math = evaluateMath(query);
    if (math) extra.push(math);

    if (query.startsWith("g ") || query.startsWith("? ")) {
      const ws = webSearchEntry(query, true);
      if (ws) extra.push(ws);
    }

    // Backend fuzzy search
    const backend = await invoke("search", { query, mode: currentMode });
    if (reqId !== requestSeq) return;

    entries = [...extra, ...backend];
    
    // Fallback: offer Google search
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

    li.innerHTML = `
      <div class="item-icon">${getIcon(entry.kind)}</div>
      <div class="item-text">
        <div class="item-name">${escapeHtml(entry.name)}</div>
        <div class="item-sub">${escapeHtml(sub)}</div>
      </div>
      ${badge ? `<span class="item-badge">${badge}</span>` : ""}
    `;

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

// ── Utility: Escape HTML to prevent injection ───────────────

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
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

// ── Event Listeners ─────────────────────────────────────────

window.addEventListener("DOMContentLoaded", async () => {
  const win = getCurrentWindow();

  // Listen for mode switches from Rust
  await listen("switch-mode", (event) => {
    currentMode = event.payload;
    if (currentMode === "clipboard") {
      input.placeholder = "Search Clipboard...";
    } else {
      input.placeholder = "Search apps, files, and more...";
    }
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
  input.addEventListener("input", scheduleSearch);

  // Keyboard navigation
  window.addEventListener("keydown", async (e) => {
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
