// ─────────────────────────────────────────────────────────────
// JOR — Shared frontend helpers
// Icons, type labels, HTML escaping, debounce, and theme syncing.
// Imported as an ES module by the launcher and clipboard windows.
// ─────────────────────────────────────────────────────────────

// Feather-style inline SVG icon set. EntryKind values map to these.
export const ICONS = {
  app: `<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1.5"></rect><rect x="14" y="3" width="7" height="7" rx="1.5"></rect><rect x="3" y="14" width="7" height="7" rx="1.5"></rect><rect x="14" y="14" width="7" height="7" rx="1.5"></rect></svg>`,
  file: `<svg viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline></svg>`,
  folder: `<svg viewBox="0 0 24 24"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
  system: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`,
  web: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>`,
  math: `<svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`,
  workflow: `<svg viewBox="0 0 24 24"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>`,
  plugin: `<svg viewBox="0 0 24 24"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`,
  clipboard: `<svg viewBox="0 0 24 24"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>`,
};

const ICON_BY_KIND = { 0: "app", 1: "file", 2: "folder", 3: "system", 4: "web", 5: "math", 6: "workflow", 7: "plugin", 8: "clipboard" };
const LABEL_BY_KIND = { 0: "Application", 1: "File", 2: "Folder", 3: "System", 4: "Web Search", 5: "Calculator", 6: "Workflow", 7: "Plugin", 8: "Clipboard" };

/** Icon SVG for an EntryKind integer (falls back to a file icon). */
export function getIcon(kind) {
  return ICONS[ICON_BY_KIND[kind]] || ICONS.file;
}

/** Human-readable type label for an EntryKind integer. */
export function getTypeLabel(kind) {
  return LABEL_BY_KIND[kind] || "Item";
}

/** Escape a string for safe innerHTML injection. */
export function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

/** Returns a debounced wrapper that fires `fn` after `delay` ms of silence. */
export function debounce(fn, delay) {
  let timer = null;
  return (...args) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn(...args);
    }, delay);
  };
}

/** Apply a ThemeConfig object to the CSS custom properties of the page. */
export function applyTheme(theme) {
  if (!theme) return;
  const root = document.documentElement.style;
  root.setProperty("--bg-top", theme.bg_top);
  root.setProperty("--bg-mid", theme.bg_mid);
  root.setProperty("--bg-bottom", theme.bg_bottom);
  root.setProperty("--accent", theme.accent);
  root.setProperty("--panel-border", theme.panel_border);
}

/** Show a transient toast notification (reuses a single shared element). */
export function showToast(message, type = "success", duration = 2400) {
  let toast = document.querySelector(".notification-toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.className = "notification-toast";
    document.body.appendChild(toast);
  }
  toast.className = `notification-toast ${type}`;
  toast.textContent = message;
  requestAnimationFrame(() => toast.classList.add("show"));
  clearTimeout(toast._hideTimer);
  toast._hideTimer = setTimeout(() => toast.classList.remove("show"), duration);
}
