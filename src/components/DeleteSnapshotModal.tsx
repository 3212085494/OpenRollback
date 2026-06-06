import { useEffect } from "react";
import type { Snapshot } from "../types";

interface DeleteSnapshotModalProps {
  snapshot: Snapshot | null;
  loading: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DeleteSnapshotModal({
  snapshot,
  loading,
  onConfirm,
  onCancel,
}: DeleteSnapshotModalProps) {
  const open = snapshot !== null;

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onCancel]);

  if (!open || !snapshot) return null;

  const isActive = snapshot.status === "active";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-title"
    >
      <button
        type="button"
        aria-label="关闭"
        className="absolute inset-0 bg-black/70 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={onCancel}
      />

      <div className="relative w-full max-w-md animate-in zoom-in-95 fade-in slide-in-from-bottom-4 duration-300">
        <div className="overflow-hidden rounded-xl border border-slate-700 bg-slate-950 shadow-[0_0_40px_rgba(100,116,139,0.2)]">
          <div className="border-b border-slate-800 bg-slate-900/50 px-5 py-3">
            <p className="font-mono text-[10px] tracking-widest text-slate-500">
              // CONFIRM_DELETE
            </p>
            <h3 id="delete-title" className="mt-1 font-mono text-lg font-bold text-slate-200">
              删除快照 #{snapshot.id}？
            </h3>
          </div>

          <div className="space-y-3 px-5 py-4 font-mono text-sm">
            <p className="text-slate-400">
              将永久删除此快照的数据库记录和备份文件，且无法恢复。
            </p>
            <div className="rounded border border-slate-800 bg-slate-900/80 p-3 text-xs">
              <p className="text-slate-500">描述</p>
              <p className="mt-1 text-slate-300">{snapshot.description || "(无)"}</p>
              <p className="mt-2 text-slate-500">状态 / 文件数</p>
              <p className="mt-1 text-cyan-400">
                {snapshot.status} · {snapshot.file_count} files
              </p>
            </div>
            {isActive ? (
              <p className="text-xs text-amber-500/90">
                ⚠ 删除不会撤销已发生的文件变更；若需恢复文件请使用「回滚」。
              </p>
            ) : (
              <p className="text-xs text-slate-500">
                已回滚的快照可安全删除以释放备份空间。
              </p>
            )}
          </div>

          <div className="flex gap-2 border-t border-slate-800 px-5 py-3">
            <button
              type="button"
              disabled={loading}
              onClick={onCancel}
              className="flex-1 rounded border border-slate-700 py-2 font-mono text-xs text-slate-400 transition-colors hover:bg-slate-900 disabled:opacity-50"
            >
              取消
            </button>
            <button
              type="button"
              disabled={loading}
              onClick={onConfirm}
              className="flex-1 rounded border border-red-700/60 bg-slate-900 py-2 font-mono text-xs font-semibold text-red-300 transition-all hover:bg-red-950/60 disabled:opacity-50"
            >
              {loading ? "删除中…" : "确认删除"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
