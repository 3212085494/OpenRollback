import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  CreateSnapshotResult,
  RollbackResult,
  LogEntry,
  LogLevel,
  QuickPath,
  Snapshot,
  WatchingStatus,
} from "../types";

function makeLogId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

async function resolveWatchPath(path: string): Promise<string> {
  const trimmed = path.trim();
  if (!trimmed) {
    const quickPaths = await invoke<QuickPath[]>("get_quick_paths");
    const fallback =
      quickPaths.find((p) => p.label === "桌面") ??
      quickPaths.find((p) => p.label === "用户目录") ??
      quickPaths[0];
    if (!fallback?.path) {
      throw new Error("监控路径为空，请先选择目录");
    }
    return fallback.path;
  }
  return invoke<string>("normalize_watch_path", { path: trimmed });
}

export function useOpenRollback() {
  const [watchPath, setWatchPath] = useState("");
  const [snapshotDesc, setSnapshotDesc] = useState("");
  const [watching, setWatching] = useState<WatchingStatus | null>(null);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);

  const appendLog = useCallback((message: string, level: LogLevel = "info") => {
    const entry: LogEntry = {
      id: makeLogId(),
      time: new Date().toLocaleTimeString(),
      level,
      message,
    };
    setLogs((prev) => [...prev, entry].slice(-200));
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [ws, snaps] = await Promise.all([
        invoke<WatchingStatus>("get_watching_status"),
        invoke<Snapshot[]>("get_snapshots"),
      ]);
      setWatching(ws);
      setSnapshots(snaps);
    } catch (e) {
      appendLog(`刷新状态失败: ${e}`, "error");
    }
  }, [appendLog]);

  useEffect(() => {
    invoke<QuickPath[]>("get_quick_paths")
      .then((paths) => {
        const defaultPath =
          paths.find((p) => p.label === "桌面") ??
          paths.find((p) => p.label === "用户目录") ??
          paths[0];
        if (defaultPath?.path) {
          setWatchPath(defaultPath.path);
        }
      })
      .catch(() => {
        // 非 Tauri 环境或后端未就绪时忽略
      });
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 3000);
    return () => clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const startWatching = useCallback(async () => {
    setLoading(true);
    try {
      const resolvedPath = await resolveWatchPath(watchPath);
      setWatchPath(resolvedPath);
      await invoke("start_watching", { path: resolvedPath });
      const ws = await invoke<WatchingStatus>("get_watching_status");
      setWatching(ws);
      const prefetchMsg = ws.prefetch_truncated
        ? `已预缓存 ${ws.cached_files}/${ws.prefetch_total} 个文件（目录文件过多，建议监控子文件夹）`
        : `已预缓存 ${ws.cached_files} 个文件，删除可回滚`;
      appendLog(`▶ 开始监控: ${resolvedPath}（${prefetchMsg}）`, "success");
      if (ws.prefetch_truncated) {
        appendLog(
          "⚠ 桌面文件超过预缓存上限，较旧/子目录中的文件删除后可能无法回滚",
          "warn",
        );
      }
      await refresh();
    } catch (e) {
      appendLog(`监控启动失败: ${e}`, "error");
      throw e;
    } finally {
      setLoading(false);
    }
  }, [watchPath, appendLog, refresh]);

  const stopWatching = useCallback(async () => {
    setLoading(true);
    try {
      await invoke("stop_watching");
      appendLog("■ 监控已停止", "warn");
      await refresh();
    } catch (e) {
      appendLog(`停止监控失败: ${e}`, "error");
      throw e;
    } finally {
      setLoading(false);
    }
  }, [appendLog, refresh]);

  const createSnapshot = useCallback(async () => {
    const desc = snapshotDesc.trim() || `快照 ${new Date().toLocaleString()}`;

    let latestWatching = watching;
    try {
      latestWatching = await invoke<WatchingStatus>("get_watching_status");
      setWatching(latestWatching);
    } catch {
      // 刷新失败时沿用本地状态
    }

    const activeWatchPath = latestWatching?.paths?.[0] ?? watchPath;

    if (!latestWatching?.is_watching) {
      appendLog("请先点击 WATCH 启动监控，并在监控目录中修改文件后再创建快照", "warn");
      return;
    }

    if ((latestWatching.pending_changes ?? 0) === 0) {
      appendLog(
        `监控目录 ${activeWatchPath} 中暂无待提交变更，请先在目录内新建或修改文件`,
        "warn",
      );
      return;
    }

    setLoading(true);
    try {
      const result = await invoke<CreateSnapshotResult>("create_snapshot", {
        description: desc,
      });
      appendLog(
        `📸 快照已创建 #${result.id}（${result.file_count} 个文件）— ${desc}`,
        "success",
      );
      setSnapshotDesc("");
      await refresh();
      return result.id;
    } catch (e) {
      appendLog(`创建快照失败: ${e}`, "error");
      throw e;
    } finally {
      setLoading(false);
    }
  }, [snapshotDesc, watchPath, watching, appendLog, refresh]);

  const triggerRollback = useCallback(
    async (snapshotId: number) => {
      setLoading(true);
      try {
        const result = await invoke<RollbackResult>("trigger_rollback", { snapshotId });
        if (result.restored > 0) {
          appendLog(
            `⏪ 快照 #${snapshotId} 已恢复 ${result.restored} 个文件`,
            "success",
          );
          for (const line of result.details) {
            appendLog(`  · ${line}`, "info");
          }
        } else if (result.removed > 0) {
          appendLog(
            `⏪ 快照 #${snapshotId} 已撤销 ${result.removed} 个新建文件`,
            "warn",
          );
          for (const line of result.details) {
            appendLog(`  · ${line}`, "info");
          }
        } else {
          appendLog(`⏪ 已回滚快照 #${snapshotId}`, "warn");
        }
        if (result.skipped > 0) {
          appendLog(`⚠ ${result.skipped} 项被跳过`, "warn");
        }
        await refresh();
      } catch (e) {
        appendLog(`回滚失败: ${e}`, "error");
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [appendLog, refresh],
  );

  const deleteSnapshot = useCallback(
    async (snapshotId: number) => {
      setLoading(true);
      try {
        await invoke("delete_snapshot", { snapshotId });
        appendLog(`🗑 已删除快照 #${snapshotId}`, "warn");
        await refresh();
      } catch (e) {
        appendLog(`删除快照失败: ${e}`, "error");
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [appendLog, refresh],
  );

  return {
    watchPath,
    setWatchPath,
    snapshotDesc,
    setSnapshotDesc,
    watching,
    snapshots,
    logs,
    loading,
    logEndRef,
    appendLog,
    refresh,
    startWatching,
    stopWatching,
    createSnapshot,
    triggerRollback,
    deleteSnapshot,
  };
}
