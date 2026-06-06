import { useDragScroll } from "../hooks/useDragScroll";
import type { Snapshot } from "../types";

interface TimeMachineProps {
  snapshots: Snapshot[];
  selectedId: number | null;
  loading: boolean;
  onSelect: (snapshot: Snapshot) => void;
  onRollback: (snapshot: Snapshot) => void;
  onDelete: (snapshot: Snapshot) => void;
  onRefresh: () => void;
}

function formatTime(iso: string) {
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

function nodeStyle(status: string) {
  switch (status) {
    case "active":
      return {
        ring: "ring-emerald-500/50",
        dot: "bg-emerald-400 shadow-[0_0_10px_#34d399]",
        border: "border-emerald-800/60",
        bg: "bg-emerald-950/20",
        label: "ACTIVE",
        labelColor: "text-emerald-400",
      };
    case "rolled_back":
      return {
        ring: "ring-red-500/40",
        dot: "bg-red-500 shadow-[0_0_10px_#ef4444]",
        border: "border-red-900/50",
        bg: "bg-red-950/20",
        label: "ROLLED",
        labelColor: "text-red-400",
      };
    default:
      return {
        ring: "ring-slate-600/40",
        dot: "bg-slate-500",
        border: "border-slate-700",
        bg: "bg-slate-900/40",
        label: status.toUpperCase(),
        labelColor: "text-slate-400",
      };
  }
}

export function TimeMachine({
  snapshots,
  selectedId,
  loading,
  onSelect,
  onRollback,
  onDelete,
  onRefresh,
}: TimeMachineProps) {
  const { ref, dragProps, wasDragged, scrollBy, scrollToStart, scrollToEnd } =
    useDragScroll<HTMLDivElement>();
  const sorted = [...snapshots].sort(
    (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
  );

  const handleSelect = (snap: Snapshot) => {
    if (wasDragged()) return;
    onSelect(snap);
  };

  return (
    <section className="relative flex h-full min-h-0 min-w-0 w-full flex-col overflow-hidden bg-slate-950/50">
      <div className="flex shrink-0 items-center justify-between border-b border-slate-800 px-5 py-2">
        <div className="flex items-center gap-3">
          <h2 className="font-mono text-xs font-semibold tracking-widest text-cyan-500/80">
            // TIMELINE
          </h2>
          {sorted.length > 0 && (
            <span className="font-mono text-[10px] text-slate-600">
              拖动 / 滚轮 / ◀▶ 浏览全部快照
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={loading}
          className="font-mono text-[10px] text-slate-500 transition-colors hover:text-cyan-400 disabled:opacity-50"
        >
          ↻ REFRESH
        </button>
      </div>

      {/* 左右滚动按钮 */}
      {sorted.length > 3 && (
        <>
          <button
            type="button"
            aria-label="向左滚动"
            onClick={() => scrollBy(-280)}
            className="absolute left-2 top-1/2 z-20 flex h-10 w-8 -translate-y-1/2 items-center justify-center rounded border border-slate-700 bg-slate-900/90 font-mono text-cyan-400 shadow-lg backdrop-blur hover:bg-slate-800"
          >
            ◀
          </button>
          <button
            type="button"
            aria-label="向右滚动"
            onClick={() => scrollBy(280)}
            className="absolute right-2 top-1/2 z-20 flex h-10 w-8 -translate-y-1/2 items-center justify-center rounded border border-slate-700 bg-slate-900/90 font-mono text-cyan-400 shadow-lg backdrop-blur hover:bg-slate-800"
          >
            ▶
          </button>
        </>
      )}

      <div
        ref={ref}
        {...dragProps}
        className="timeline-scroll min-h-0 min-w-0 w-full flex-1 cursor-grab overflow-x-auto overflow-y-hidden overscroll-x-contain p-6 active:cursor-grabbing"
        style={{ touchAction: "pan-x" }}
      >
        {sorted.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
            <div className="h-px w-32 bg-gradient-to-r from-transparent via-emerald-800 to-transparent" />
            <p className="font-mono text-sm text-slate-500">时间轴为空</p>
            <p className="max-w-xs font-mono text-xs text-slate-600">
              启动监控 → 修改文件 → 创建快照，节点将出现在此处
            </p>
          </div>
        ) : (
          <div className="relative inline-block min-w-max pb-4 pr-8">
            <div className="absolute left-8 right-8 top-6 h-px bg-gradient-to-r from-emerald-900/20 via-emerald-600/40 to-emerald-900/20" />

            <ol className="flex items-start">
              {sorted.map((snap, index) => {
                const style = nodeStyle(snap.status);
                const canRollback = snap.status === "active";
                const isSelected = selectedId === snap.id;

                return (
                  <li
                    key={snap.id}
                    className="relative flex w-52 shrink-0 flex-col items-center px-2"
                  >
                    <div className="relative z-10 mb-3 flex flex-col items-center">
                      <div
                        className={`h-3 w-3 rounded-full ring-2 ${style.ring} ${style.dot}`}
                      />
                      <span
                        className={`mt-1 font-mono text-[9px] tracking-wider ${style.labelColor}`}
                      >
                        {style.label}
                      </span>
                    </div>

                    <article
                      role="button"
                      tabIndex={0}
                      onClick={() => handleSelect(snap)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") handleSelect(snap);
                      }}
                      className={`w-full cursor-pointer rounded-lg border ${style.border} ${style.bg} p-3 backdrop-blur-sm transition-all hover:scale-[1.02] ${
                        isSelected
                          ? "ring-2 ring-cyan-500/60 shadow-[0_0_16px_rgba(34,211,238,0.2)]"
                          : ""
                      }`}
                    >
                      <div className="mb-2 flex items-start justify-between gap-1">
                        <span className="font-mono text-lg font-bold text-slate-200">
                          #{snap.id}
                        </span>
                        <span className="rounded bg-slate-900/80 px-1.5 py-0.5 font-mono text-[10px] text-cyan-400/90">
                          {snap.file_count} files
                        </span>
                      </div>

                      <time className="block font-mono text-[10px] text-slate-500">
                        {formatTime(snap.timestamp)}
                      </time>

                      <p className="mt-2 line-clamp-2 font-mono text-xs leading-snug text-slate-300">
                        {snap.description || "(无描述)"}
                      </p>

                      <div className="mt-3 flex gap-1.5">
                        <button
                          type="button"
                          disabled={!canRollback || loading}
                          onPointerDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (wasDragged()) return;
                            onRollback(snap);
                          }}
                          className={`flex-1 rounded border py-1.5 font-mono text-[10px] tracking-wider transition-all disabled:cursor-not-allowed disabled:opacity-30 ${
                            canRollback
                              ? "border-amber-600/50 text-amber-400 hover:bg-amber-950/50 hover:shadow-[0_0_10px_rgba(251,191,36,0.2)]"
                              : "border-slate-700 text-slate-600"
                          }`}
                        >
                          {canRollback ? "⏪ 回滚" : "已回滚"}
                        </button>
                        <button
                          type="button"
                          disabled={loading}
                          onPointerDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (wasDragged()) return;
                            onDelete(snap);
                          }}
                          className="rounded border border-slate-700 px-2 py-1.5 font-mono text-[10px] text-slate-500 transition-colors hover:border-red-800 hover:bg-red-950/40 hover:text-red-400 disabled:opacity-30"
                          title="删除快照"
                        >
                          🗑
                        </button>
                      </div>
                    </article>

                    {index < sorted.length - 1 && (
                      <span className="absolute -right-1 top-5 font-mono text-[10px] text-emerald-800">
                        →
                      </span>
                    )}
                  </li>
                );
              })}
            </ol>
          </div>
        )}
      </div>

      {/* 底部快捷跳转 */}
      {sorted.length > 3 && (
        <div className="flex shrink-0 items-center justify-center gap-2 border-t border-slate-800 py-1.5">
          <button
            type="button"
            onClick={scrollToStart}
            className="rounded px-2 py-0.5 font-mono text-[10px] text-slate-500 hover:text-cyan-400"
          >
            |◀ 最早
          </button>
          <span className="font-mono text-[10px] text-slate-600">
            共 {sorted.length} 个快照
          </span>
          <button
            type="button"
            onClick={scrollToEnd}
            className="rounded px-2 py-0.5 font-mono text-[10px] text-slate-500 hover:text-cyan-400"
          >
            最新 ▶|
          </button>
        </div>
      )}
    </section>
  );
}
