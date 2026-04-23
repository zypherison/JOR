import React, { useState, useEffect, useRef } from 'react';
import { Search, Cpu, Zap, ArrowRight, CornerDownLeft } from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export default function JORApp() {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isIndexing, setIsIndexing] = useState(false);
  
  // Mock data for initial design validation
  const results = [
    { id: 1, name: "Google Chrome", subtitle: "C:\\Program Files\\Google\\Chrome", kind: "APP", acc: "EXE" },
    { id: 2, name: "Visual Studio Code", subtitle: "C:\\Users\\AppData\\Local\\Programs\\VSCode", kind: "APP", acc: "EXE" },
    { id: 3, name: "Projects", subtitle: "C:\\Users\\Documents\\Projects", kind: "DIR", acc: "DIR" },
    { id: 4, name: "index.cache", subtitle: "C:\\Users\\AppData\\Local\\jor\\index.cache", kind: "FILE", acc: "BIN" },
  ];

  return (
    <div className="w-[800px] h-[450px] brutalist-card flex flex-col border-[#ffffff30] border-2">
      {/* Header / Search Section */}
      <div className="p-6 border-b border-[#ffffff10] flex items-center gap-4">
        <span className="text-[#96f8ff] font-mono font-bold text-xl">$</span>
        <input 
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="SEARCH_KERNEL..."
          className="brutalist-input flex-1"
        />
        <div className="blink-cursor"></div>
      </div>

      {/* Results Section */}
      <div className="flex-1 overflow-y-auto custom-scrollbar">
        {/* Category Header */}
        <div className="px-4 py-1.5 bg-[#ffffff05] text-[10px] font-mono text-[#ffffff40] uppercase tracking-[0.2em] border-b border-[#ffffff10]">
          PROCESS_QUEUE / ALL_TASKS
        </div>

        {results.map((item, idx) => (
          <div 
            key={item.id}
            className={cn(
              "flex items-center justify-between px-6 py-3 border-b border-[#ffffff05] cursor-pointer group transition-none",
              idx === selectedIndex && "item-active"
            )}
            onMouseEnter={() => setSelectedIndex(idx)}
          >
            <div className="flex items-center gap-4">
              <span className={cn(
                "brutalist-badge",
                idx === selectedIndex && "border-black text-black"
              )}>
                {item.kind}
              </span>
              <div className="min-w-0">
                <div className="font-bold text-base truncate tracking-tight">{item.name}</div>
                <div className={cn(
                  "font-mono text-[10px] truncate max-w-[400px] opacity-60",
                  idx === selectedIndex && "text-black/60"
                )}>
                  {item.subtitle}
                </div>
              </div>
            </div>

            <div className={cn(
              "flex items-center gap-3 opacity-0",
              idx === selectedIndex && "opacity-100"
            )}>
              <span className="font-mono text-[10px] font-bold tracking-widest border-b border-current">
                {item.acc} · EXECUTE
              </span>
              <CornerDownLeft size={18} />
            </div>
          </div>
        ))}
      </div>

      {/* Footer / Telemetry */}
      <div className="px-6 py-3 bg-[#000] border-t border-[#ffffff15] flex items-center justify-between font-mono text-[10px] tracking-widest text-[#ffffff40]">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <span className={cn("w-2 h-2", isIndexing ? "bg-yellow-400 animate-pulse" : "bg-[#96f8ff]")}></span>
            <span className={isIndexing ? "text-yellow-400" : ""}>
              {isIndexing ? "MAPPING_FS..." : "SYSTEM_READY"}
            </span>
          </div>
          <div className="flex items-center gap-4 border-l border-[#ffffff15] pl-6">
            <div className="flex items-center gap-2">
              <Cpu size={12} />
              <span>MEM_14%</span>
            </div>
            <div className="flex items-center gap-2">
              <Zap size={12} />
              <span>CPU_2.4%</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2 text-[#96f8ff] font-bold">
          <span>JOR_V2.0.0</span>
        </div>
      </div>
    </div>
  );
}
