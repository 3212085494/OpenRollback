import { useState } from "react";
import { ConfirmationModal } from "./components/ConfirmationModal";
import { DeleteSnapshotModal } from "./components/DeleteSnapshotModal";
import { OperationLog } from "./components/OperationLog";
import { SnapshotFilePanel } from "./components/SnapshotFilePanel";
import { TimeMachine } from "./components/TimeMachine";
import { TopBar } from "./components/TopBar";
import { useOpenRollback } from "./hooks/useOpenRollback";
import type { Snapshot } from "./types";

function App() {
  const {
    watchPath,
    setWatchPath,
    snapshotDesc,
    setSnapshotDesc,
    watching,
    snapshots,
    logs,
    loading,
    logEndRef,
    refresh,
    startWatching,
    stopWatching,
    createSnapshot,
    triggerRollback,
    deleteSnapshot,
  } = useOpenRollback();

  const [rollbackTarget, setRollbackTarget] = useState<Snapshot | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Snapshot | null>(null);
  const [selectedSnapshot, setSelectedSnapshot] = useState<Snapshot | null>(null);

  const handleConfirmRollback = async () => {
    if (!rollbackTarget) return;
    try {
      await triggerRollback(rollbackTarget.id);
      setRollbackTarget(null);
      setSelectedSnapshot(null);
    } catch {
      // 错误已在日志中记录
    }
  };

  const handleConfirmDelete = async () => {
    if (!deleteTarget) return;
    try {
      await deleteSnapshot(deleteTarget.id);
      if (selectedSnapshot?.id === deleteTarget.id) {
        setSelectedSnapshot(null);
      }
      setDeleteTarget(null);
    } catch {
      // 错误已在日志中记录
    }
  };

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
      <div
        className="pointer-events-none fixed inset-0 opacity-[0.03]"
        style={{
          backgroundImage:
            "repeating-linear-gradient(0deg, transparent, transparent 2px, #34d399 2px, #34d399 3px)",
        }}
      />

      <TopBar
        watching={watching}
        watchPath={watchPath}
        onWatchPathChange={setWatchPath}
        snapshotDesc={snapshotDesc}
        onSnapshotDescChange={setSnapshotDesc}
        loading={loading}
        onStartWatching={startWatching}
        onStopWatching={stopWatching}
        onCreateSnapshot={createSnapshot}
      />

      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-1 lg:grid-cols-[minmax(240px,300px)_minmax(0,1fr)]">
        <OperationLog logs={logs} logEndRef={logEndRef} />

        <div className="flex min-h-0 min-w-0 flex-col overflow-hidden lg:flex-row">
          <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
            <TimeMachine
              snapshots={snapshots}
              selectedId={selectedSnapshot?.id ?? null}
              loading={loading}
              onSelect={setSelectedSnapshot}
              onRollback={setRollbackTarget}
              onDelete={setDeleteTarget}
              onRefresh={refresh}
            />
          </div>
          <SnapshotFilePanel
            snapshot={selectedSnapshot}
            onRollback={setRollbackTarget}
            onDelete={setDeleteTarget}
            loading={loading}
          />
        </div>
      </div>

      <ConfirmationModal
        snapshot={rollbackTarget}
        loading={loading}
        onConfirm={handleConfirmRollback}
        onCancel={() => setRollbackTarget(null)}
      />

      <DeleteSnapshotModal
        snapshot={deleteTarget}
        loading={loading}
        onConfirm={handleConfirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}

export default App;
