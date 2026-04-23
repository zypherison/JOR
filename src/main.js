const { invoke } = window.__TAURI__.core;

// ── Application State ───────────────────────────────────────

let entries = [];
let selectedIndex = 0;
let requestSeq = 0;

const input = document.getElementById("search-input");
const resultsList = document.getElementById("results-list");
const statusIcon = document.getElementById("status-icon");
const statusText = document.getElementById("status-text");
const memUsage = document.getElementById("mem-usage");
const cpuUsage = document.getElementById("cpu-usage");
const memIndicator = document.getElementById("mem-indicator");

// ── Category Mapping ────────────────────────────────────────

const CATEGORIES = [
    { name: "Process_Queue / Applications", kinds: [0] },
    { name: "Path_Mapper / Directories", kinds: [2] },
    { name: "I/O_Streams / Files", kinds: [1] },
    { name: "Kernel_Interface / System Operations", kinds: [3, 4, 5, 6, 7] }
];

const BADGES = {
    0: "APP",
    1: "FILE",
    2: "DIR",
    3: "CMD",
    4: "WEB",
    5: "CALC",
    6: "BOLT",
    7: "AUTO"
};

// ── System Stats Simulation ─────────────────────────────────

function updateStats() {
    const mem = Math.floor(Math.random() * 5 + 12);
    const cpu = (Math.random() * 2 + 1).toFixed(1);
    memUsage.textContent = `${mem}%`;
    cpuUsage.textContent = `${cpu}%`;
}
setInterval(updateStats, 5000);
updateStats();

// ── Core: Search & Rendering ────────────────────────────────

input.addEventListener("input", () => {
    const reqId = ++requestSeq;
    performSearch(reqId);
});

async function performSearch(reqId) {
    const query = input.value.trim();
    if (!query) {
        entries = [];
        renderResults();
        return;
    }

    try {
        const extra = [];
        if (query.toLowerCase() === "refresh" || query.toLowerCase() === "reindex") {
            extra.push({
                name: "Refresh Search Index",
                name_lower: "refresh",
                path: "REFRESH_INDEX_CMD",
                subtitle: "REBUILD_GLOBAL_MAP",
                kind: 3,
                score: 1000,
            });
        }

        const backend = await invoke("search", { query });
        if (reqId !== requestSeq) return;

        entries = [...extra, ...backend];
        renderResults();
    } catch (err) {
        console.error(err);
    }
}

function renderResults() {
    resultsList.innerHTML = "";
    selectedIndex = Math.min(selectedIndex, Math.max(0, entries.length - 1));

    if (entries.length === 0 && input.value.length > 0) {
        resultsList.innerHTML = `<div class="p-12 text-center font-mono text-xs text-outline tracking-widest uppercase">Target_Not_Found: 0x404</div>`;
        return;
    }

    // Grouping logic for rendering
    CATEGORIES.forEach(cat => {
        const itemsInCat = entries.filter(e => cat.kinds.includes(e.kind));
        if (itemsInCat.length === 0) return;

        // Header
        const header = document.createElement("div");
        header.className = "px-4 py-1.5 bg-surface-container text-[10px] font-mono text-on-surface-variant uppercase tracking-[0.2em] border-b border-[#ffffff15]";
        header.textContent = cat.name;
        resultsList.appendChild(header);

        // Items
        itemsInCat.forEach(entry => {
            const globalIdx = entries.indexOf(entry);
            const isActive = globalIdx === selectedIndex;
            
            const item = document.createElement("div");
            item.className = `flex items-center justify-between px-6 py-3 transition-colors group border-b border-[#ffffff05] cursor-pointer ${isActive ? 'item-active' : 'hover:bg-surface-container-low'}`;
            
            const badge = BADGES[entry.kind] || "ITEM";
            const badgeClass = entry.kind === 3 ? "text-error border-error/30" : (entry.kind === 2 ? "text-tertiary-fixed border-[#ffffff15]" : "text-on-surface-variant border-[#ffffff15]");
            
            // Platform Redesign: Accessories support
            const accText = (entry.accessories && entry.accessories.length > 0) 
                ? entry.accessories.join(" · ") 
                : (isActive ? 'EXECUTE' : 'READY');

            item.innerHTML = `
                <div class="flex items-center gap-4">
                    <span class="font-mono text-xs font-bold px-1.5 py-0.5 border ${isActive ? 'border-black' : badgeClass}">${badge}</span>
                    <div class="min-w-0">
                        <div class="font-headline font-bold text-base truncate">${entry.name}</div>
                        <div class="font-mono text-[10px] truncate max-w-[320px] item-path opacity-60">${entry.subtitle}</div>
                    </div>
                </div>
                <div class="flex items-center gap-3 opacity-0 group-hover:opacity-100 ${isActive ? 'opacity-100' : ''}">
                    <span class="font-mono text-[10px] font-bold tracking-widest item-action border-b border-white/20">${accText}</span>
                    <span class="material-symbols-outlined text-xl">subdirectory_arrow_left</span>
                </div>
            `;

            item.onclick = () => {
                selectedIndex = globalIdx;
                launchEntry(entry);
            };
            resultsList.appendChild(item);

            if (isActive) {
                item.scrollIntoView({ block: "nearest", behavior: "smooth" });
            }
        });
    });
}

// ── Core: Launch Logic ──────────────────────────────────────

async function launchEntry(entry) {
    if (entry.path === "REFRESH_INDEX_CMD") {
        statusText.textContent = "INDEXING_CORE...";
        statusIcon.textContent = "sync";
        memIndicator.classList.add("pulse-active");
        
        try {
            const [count, ms] = await invoke("refresh_index");
            statusText.textContent = `MAP_COMPLETE: ${count}_ITEMS (${(ms/1000).toFixed(1)}s)`;
        } catch (err) {
            statusText.textContent = "CORE_ERROR: 0xERR";
        }
        
        setTimeout(() => {
            statusText.textContent = "SYSTEM_READY";
            statusIcon.textContent = "check_circle";
            memIndicator.classList.remove("pulse-active");
        }, 5000);
        return;
    }

    await invoke("launch_entry", { entry });
    await invoke("hide_window");
}

// ── Keyboard Interface ──────────────────────────────────────

window.addEventListener("keydown", (e) => {
    if (entries.length === 0) return;

    if (e.key === "ArrowDown") {
        e.preventDefault();
        selectedIndex = (selectedIndex + 1) % entries.length;
        renderResults();
    } else if (e.key === "ArrowUp") {
        e.preventDefault();
        selectedIndex = (selectedIndex - 1 + entries.length) % entries.length;
        renderResults();
    } else if (e.key === "Enter") {
        e.preventDefault();
        if (entries[selectedIndex]) {
            launchEntry(entries[selectedIndex]);
        }
    } else if (e.key === "Escape") {
        invoke("hide_window");
    }
});

// Auto-focus input on show (Tauri event)
window.addEventListener("focus", () => {
    input.focus();
});
