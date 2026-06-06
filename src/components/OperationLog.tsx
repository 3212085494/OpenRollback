import type { RefObject } from "react";
import type { LogEntry } from "../types";

interface OperationLogProps {
  logs: LogEntry[];
  logEndRef: RefObject<HTMLDivElement | null>;
}

const levelStyles: Record<LogEntry["level"], string> = {
  info: "text-slate-400",
  success: "text-emerald-400",
  warn: "text-amber-400",
  error: "text-red-400",
};

const levelPrefix: Record<LogEntry["level"], string> = {
  info: "INF",
  success: " OK",
  warn: "WRN",
  error: "ERR",
};

export function OperationLog({ logs, logEndRef }: OperationLogProps) {
  return (
    <aside className="flex h-full min-h-0 flex-col border-r border-emerald-900/30 bg-slate-950/80">
      <div className="flex shrink-0 items-center justify-between border-b border-slate-800 px-4 py-2">
        <h2 className="font-mono text-xs font-semibold tracking-widest text-emerald-500/80">
          // LIVE_LOG
        </h2>
        <span className="font-mono text-[10px] text-slate-600">{logs.length} events</span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto scroll-smooth px-3 py-2">
        {logs.length === 0 ? (
          <p className="font-mono text-xs text-slate-600">
            <span className="animate-pulse">_</span> 等待系统事件…
          </p>
        ) : (
          <ul className="space-y-0.5">
            {logs.map((entry, i) => (
              <li
                key={entry.id}
                className="group flex gap-2 font-mono text-[11px] leading-relaxed animate-in fade-in slide-in-from-left-2 duration-300"
                style={{ animationDelay: `${Math.min(i, 5) * 30}ms` }}
              >
                <span className="shrink-0 text-slate-600">{entry.time}</span>
                <span className={`shrink-0 ${levelStyles[entry.level]}`}>
                  [{levelPrefix[entry.level]}]
                </span>
                <span className={`min-w-0 break-all ${levelStyles[entry.level]}`}>
                  {entry.message}
                </span>
              </li>
            ))}
          </ul>
        )}
        <div ref={logEndRef} />
      </div>
    </aside>
  );
}
