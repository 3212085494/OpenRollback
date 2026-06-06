import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useDragScroll } from "../hooks/useDragScroll";
import type { FileChange, Snapshot } from "../types";

interface SnapshotFilePanelProps {
  snapshot: Snapshot | null;
  onRollback: (snapshot: Snapshot) => void;
  onDelete: (snapshot: Snapshot) => void;
  loading: boolean;
}

const actionStyle: Record<string, string> = {
  create: "bg-emerald-950 text-emerald-400 ring-emerald-800",
  modify: "bg-amber-950 text-amber-400 ring-amber-800",
  delete: "bg-red-950 text-red-400 ring-red-800",
};

function fileName(path: string) {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || path;
}

export function SnapshotFilePanel({
  snapshot,
  onRollback,
  onDelete,
  loading,
}: SnapshotFilePanelProps) {
  const [files, setFiles] = useState<FileChange[]>([]);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const { ref: fileListRef, dragProps: fileListDragProps } = useDragScroll<HTMLDivElement>();

  useEffect(() => {
    if (!snapshot) {
      setFiles([]);
      return;
    }

    setLoadingFiles(true);
    invoke<FileChange[]>("get_snapshot_files", { snapshotId: snapshot.id })
      .then(setFiles)
      .catch(() => setFiles([]))
      .finally(() => setLoadingFiles(false));
  }, [snapshot]);

  if (!snapshot) {
    return (
      <aside className="flex h-full min-h-0 w-full flex-col border-t border-slate-800 bg-slate-950/80 lg:w-72 lg:border-l lg:border-t-0">
        <div className="border-b border-slate-800 px-4 py-2">
          <h2 className="font-mono text-xs font-semibold tracking-widest text-slate-500">
            // SNAPSHOT_FILES
          </h2>
        </div>
        <p className="flex flex-1 items-center justify-center px-4 text-center font-mono text-xs text-slate-600">
          点击时间轴节点查看快照文件
        </p>
      </aside>
    );
  }

  const canRollback = snapshot.status === "active";

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-t border-slate-800 bg-slate-950/80 lg:w-72 lg:border-l lg:border-t-0">
      <div className="shrink-0 border-b border-slate-800 px-4 py-2">
        <h2 className="font-mono text-xs font-semibold tracking-widest text-cyan-500/80">
          // SNAPSHOT #{snapshot.id}
        </h2>
        <p className="mt-1 truncate font-mono text-[10px] text-slate-500">
          {snapshot.description || "(无描述)"}
        </p>
      </div>

      <div
        ref={fileListRef}
        {...fileListDragProps}
        className="timeline-scroll min-h-0 flex-1 cursor-grab overflow-y-auto overscroll-y-contain px-3 py-2 active:cursor-grabbing"
      >
        {loadingFiles ? (
          <p className="font-mono text-xs text-slate-500">加载中…</p>
        ) : files.length === 0 ? (
          <p className="font-mono text-xs text-slate-600">此快照无文件变更记录</p>
        ) : (
          <ul className="space-y-1.5">
            {files.map((f) => (
              <li
                key={f.id}
                className="rounded border border-slate-800 bg-slate-900/60 px-2 py-1.5"
              >
                <div className="flex items-center gap-2">
                  <span
                    className={`shrink-0 rounded px-1 py-0.5 font-mono text-[9px] ring-1 ${
                      actionStyle[f.action] ?? "bg-slate-800 text-slate-400 ring-slate-700"
                    }`}
                  >
                    {f.action.toUpperCase()}
                  </span>
                  <span
                    className="min-w-0 truncate font-mono text-[10px] text-slate-300"
                    title={f.path}
                  >
                    {fileName(f.path)}
                  </span>
                </div>
                <p className="mt-0.5 truncate font-mono text-[9px] text-slate-600" title={f.path}>
                  {f.path}
                </p>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="shrink-0 space-y-2 border-t border-slate-800 p-3">
        {canRollback && (
          <button
            type="button"
            disabled={loading}
            onClick={() => onRollback(snapshot)}
            className="w-full rounded border border-amber-600/50 py-2 font-mono text-xs text-amber-400 transition-colors hover:bg-amber-950/50 disabled:opacity-50"
          >
            ⏪ 回滚此快照
          </button>
        )}
        <button
          type="button"
          disabled={loading}
          onClick={() => onDelete(snapshot)}
          className="w-full rounded border border-slate-700 py-2 font-mono text-xs text-slate-500 transition-colors hover:border-red-800 hover:bg-red-950/40 hover:text-red-400 disabled:opacity-50"
        >
          🗑 删除此快照
        </button>
      </div>
    </aside>
  );
}
