import { QuickPathPicker } from "./QuickPathPicker";

interface TopBarProps {
  watching: {
    is_watching: boolean;
    paths: string[];
    pending_changes: number;
    cached_files: number;
    prefetch_total: number;
    prefetch_truncated: boolean;
  } | null;
  watchPath: string;
  onWatchPathChange: (path: string) => void;
  snapshotDesc: string;
  onSnapshotDescChange: (desc: string) => void;
  loading: boolean;
  onStartWatching: () => void;
  onStopWatching: () => void;
  onCreateSnapshot: () => void;
}

export function TopBar({
  watching,
  watchPath,
  onWatchPathChange,
  snapshotDesc,
  onSnapshotDescChange,
  loading,
  onStartWatching,
  onStopWatching,
  onCreateSnapshot,
}: TopBarProps) {
  const isActive = watching?.is_watching ?? false;
  const paths = watching?.paths ?? [];

  return (
    <header className="shrink-0 border-b border-emerald-900/40 bg-slate-950/90 backdrop-blur-sm">
      <div className="flex flex-wrap items-center gap-4 px-5 py-3">
        <div className="flex items-center gap-3">
          <div className="relative flex h-9 w-9 items-center justify-center rounded border border-emerald-500/30 bg-emerald-950/50">
            <span className="font-mono text-xs font-bold text-emerald-400">OR</span>
            <span
              className={`absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full ${
                isActive ? "animate-pulse bg-emerald-400 shadow-[0_0_8px_#34d399]" : "bg-slate-600"
              }`}
            />
          </div>
          <div>
            <h1 className="font-mono text-sm font-bold tracking-wider text-emerald-300">
              OPENROLLBACK
            </h1>
            <p className="font-mono text-[10px] uppercase tracking-widest text-slate-500">
              Time Machine v0.1
            </p>
          </div>
        </div>

        <div className="hidden h-8 w-px bg-slate-800 sm:block" />

        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2 font-mono text-xs">
            <span className="text-slate-500">STATUS</span>
            <span
              className={`rounded px-2 py-0.5 ${
                isActive
                  ? "bg-emerald-950 text-emerald-400 ring-1 ring-emerald-500/30"
                  : "bg-slate-900 text-slate-400 ring-1 ring-slate-700"
              }`}
            >
              {isActive ? "WATCHING" : "IDLE"}
            </span>
            {isActive && (
              <span className="truncate text-cyan-400/80">
                {paths.join(" · ") || watchPath}
              </span>
            )}
            {isActive && watching && watching.cached_files > 0 && (
              <span
                className={`rounded px-2 py-0.5 ring-1 ${
                  watching.prefetch_truncated
                    ? "bg-amber-950 text-amber-400 ring-amber-800/40"
                    : "bg-cyan-950 text-cyan-400/90 ring-cyan-800/40"
                }`}
                title={
                  watching.prefetch_truncated
                    ? `仅预缓存了 ${watching.cached_files}/${watching.prefetch_total} 个文件`
                    : `${watching.cached_files} 个文件可回滚删除`
                }
              >
                {watching.prefetch_truncated
                  ? `${watching.cached_files}/${watching.prefetch_total} protected`
                  : `${watching.cached_files} protected`}
              </span>
            )}
            {watching && watching.pending_changes > 0 && (
              <span className="rounded bg-amber-950 px-2 py-0.5 text-amber-400 ring-1 ring-amber-500/30">
                {watching.pending_changes} pending
              </span>
            )}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <input
            type="text"
            value={watchPath}
            onChange={(e) => onWatchPathChange(e.target.value)}
            placeholder="监控路径"
            className="w-36 rounded border border-slate-700 bg-slate-900 px-2 py-1.5 font-mono text-xs text-slate-300 placeholder:text-slate-600 focus:border-emerald-600 focus:outline-none sm:w-56"
          />
          <button
            type="button"
            disabled={loading}
            onClick={isActive ? onStopWatching : onStartWatching}
            className={`rounded border px-3 py-1.5 font-mono text-xs transition-colors disabled:opacity-50 ${
              isActive
                ? "border-slate-600 text-slate-300 hover:bg-slate-800"
                : "border-emerald-600/50 text-emerald-400 hover:bg-emerald-950"
            }`}
          >
            {isActive ? "STOP" : "WATCH"}
          </button>

          <input
            type="text"
            value={snapshotDesc}
            onChange={(e) => onSnapshotDescChange(e.target.value)}
            placeholder="快照描述（可选）"
            className="w-36 rounded border border-slate-700 bg-slate-900 px-2 py-1.5 font-mono text-xs text-slate-300 placeholder:text-slate-600 focus:border-cyan-600 focus:outline-none sm:w-44"
          />
          <button
            type="button"
            disabled={loading || !isActive || (watching?.pending_changes ?? 0) === 0}
            title={
              !isActive
                ? "请先点击 WATCH 启动监控"
                : (watching?.pending_changes ?? 0) === 0
                  ? "请先在监控目录中修改文件"
                  : `提交 ${watching?.pending_changes ?? 0} 个待处理变更`
            }
            onClick={onCreateSnapshot}
            className="rounded border border-cyan-500/50 bg-cyan-950/50 px-4 py-1.5 font-mono text-xs font-semibold text-cyan-300 shadow-[0_0_12px_rgba(34,211,238,0.15)] transition-all hover:bg-cyan-900/50 hover:shadow-[0_0_16px_rgba(34,211,238,0.25)] disabled:opacity-50"
          >
            + 创建快照
            {isActive && (watching?.pending_changes ?? 0) > 0
              ? ` (${watching?.pending_changes})`
              : ""}
          </button>
        </div>
      </div>

      {/* 快捷路径选择栏 */}
      <div className="border-t border-slate-800/60 px-5 py-2">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-mono text-[10px] text-slate-500">快捷目录</span>
          <QuickPathPicker
            watchPath={watchPath}
            onSelect={onWatchPathChange}
            disabled={loading}
          />
        </div>
      </div>
    </header>
  );
}
