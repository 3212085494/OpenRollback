import { useEffect } from "react";
import type { Snapshot } from "../types";

interface ConfirmationModalProps {
  snapshot: Snapshot | null;
  loading: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmationModal({
  snapshot,
  loading,
  onConfirm,
  onCancel,
}: ConfirmationModalProps) {
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

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rollback-title"
    >
      {/* 背景遮罩 */}
      <button
        type="button"
        aria-label="关闭"
        className="absolute inset-0 bg-black/70 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={onCancel}
      />

      {/* 弹窗主体 */}
      <div className="relative w-full max-w-md animate-in zoom-in-95 fade-in slide-in-from-bottom-4 duration-300">
        <div className="overflow-hidden rounded-xl border border-red-900/50 bg-slate-950 shadow-[0_0_40px_rgba(239,68,68,0.15)]">
          <div className="border-b border-red-900/30 bg-red-950/30 px-5 py-3">
            <p className="font-mono text-[10px] tracking-widest text-red-400/80">
              // CONFIRM_ROLLBACK
            </p>
            <h3 id="rollback-title" className="mt-1 font-mono text-lg font-bold text-red-300">
              确认回滚操作？
            </h3>
          </div>

          <div className="space-y-3 px-5 py-4 font-mono text-sm">
            <p className="text-slate-400">
              将撤销快照{" "}
              <span className="text-emerald-400">#{snapshot.id}</span> 记录的所有文件变更。
            </p>
            <div className="rounded border border-slate-800 bg-slate-900/80 p-3 text-xs">
              <p className="text-slate-500">描述</p>
              <p className="mt-1 text-slate-300">{snapshot.description || "(无)"}</p>
              <p className="mt-2 text-slate-500">影响文件数</p>
              <p className="mt-1 text-cyan-400">{snapshot.file_count}</p>
            </div>
            <p className="text-xs text-slate-500">
              回滚含义：撤销该快照记录的操作。
              <span className="text-cyan-400"> 删除</span> → 恢复文件；
              <span className="text-cyan-400"> 新建</span> → 删除文件；
              <span className="text-cyan-400"> 修改</span> → 还原内容。
            </p>
            <p className="text-xs text-amber-500/90">
              ⚠ 仅恢复此快照创建时记录的操作，之后的手动变更不在范围内。
            </p>
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
              className="flex-1 rounded border border-red-600/60 bg-red-950/50 py-2 font-mono text-xs font-semibold text-red-300 transition-all hover:bg-red-900/40 hover:shadow-[0_0_16px_rgba(239,68,68,0.25)] disabled:opacity-50"
            >
              {loading ? "回滚中…" : "确认回滚"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
