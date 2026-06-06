import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { QuickPath } from "../types";

const RECENT_KEY = "openrollback_recent_paths";
const MAX_RECENT = 6;

interface QuickPathPickerProps {
  watchPath: string;
  onSelect: (path: string) => void;
  disabled?: boolean;
}

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    return raw ? (JSON.parse(raw) as string[]) : [];
  } catch {
    return [];
  }
}

function saveRecent(path: string) {
  const recent = loadRecent().filter((p) => p !== path);
  recent.unshift(path);
  localStorage.setItem(RECENT_KEY, JSON.stringify(recent.slice(0, MAX_RECENT)));
}

export function QuickPathPicker({ watchPath, onSelect, disabled }: QuickPathPickerProps) {
  const [quickPaths, setQuickPaths] = useState<QuickPath[]>([]);
  const [recentPaths, setRecentPaths] = useState<string[]>([]);
  const [openMenu, setOpenMenu] = useState(false);

  useEffect(() => {
    invoke<QuickPath[]>("get_quick_paths")
      .then(setQuickPaths)
      .catch(() => setQuickPaths([]));
    setRecentPaths(loadRecent());
  }, []);

  const handleSelect = useCallback(
    (path: string) => {
      onSelect(path);
      saveRecent(path);
      setRecentPaths(loadRecent());
      setOpenMenu(false);
    },
    [onSelect],
  );

  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择要监控的目录",
        defaultPath: watchPath || undefined,
      });
      if (selected && typeof selected === "string") {
        handleSelect(selected);
      }
    } catch {
      // 用户取消或环境不支持
    }
  }, [watchPath, handleSelect]);

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <button
        type="button"
        disabled={disabled}
        onClick={handleBrowse}
        title="浏览选择目录"
        className="rounded border border-cyan-700/50 bg-cyan-950/40 px-2 py-1.5 font-mono text-[10px] text-cyan-300 transition-colors hover:bg-cyan-900/50 disabled:opacity-50"
      >
        📁 浏览
      </button>

      {quickPaths.map((qp) => (
        <button
          key={qp.path}
          type="button"
          disabled={disabled}
          onClick={() => handleSelect(qp.path)}
          title={qp.path}
          className={`rounded border px-2 py-1 font-mono text-[10px] transition-colors disabled:opacity-50 ${
            watchPath === qp.path
              ? "border-emerald-500/60 bg-emerald-950/60 text-emerald-300"
              : "border-slate-700 text-slate-400 hover:border-slate-600 hover:text-slate-200"
          }`}
        >
          {qp.label}
        </button>
      ))}

      {recentPaths.length > 0 && (
        <div className="relative">
          <button
            type="button"
            disabled={disabled}
            onClick={() => setOpenMenu((v) => !v)}
            className="rounded border border-slate-700 px-2 py-1 font-mono text-[10px] text-slate-400 hover:text-slate-200 disabled:opacity-50"
          >
            最近 ▾
          </button>
          {openMenu && (
            <ul className="absolute left-0 top-full z-20 mt-1 max-h-40 min-w-[200px] overflow-y-auto rounded border border-slate-700 bg-slate-900 py-1 shadow-lg">
              {recentPaths.map((p) => (
                <li key={p}>
                  <button
                    type="button"
                    className="block w-full truncate px-3 py-1.5 text-left font-mono text-[10px] text-slate-300 hover:bg-slate-800"
                    title={p}
                    onClick={() => handleSelect(p)}
                  >
                    {p}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
