export interface CreateSnapshotResult {
  id: number;
  file_count: number;
}

export interface RollbackResult {
  restored: number;
  removed: number;
  skipped: number;
  details: string[];
}

export interface Snapshot {
  id: number;
  timestamp: string;
  description: string;
  status: string;
  file_count: number;
}

export interface WatchingStatus {
  is_watching: boolean;
  paths: string[];
  pending_changes: number;
  cached_files: number;
  prefetch_total: number;
  prefetch_truncated: boolean;
}

export interface FileChange {
  id: number;
  path: string;
  action: string;
  original_hash: string | null;
  backup_path: string | null;
}

export interface QuickPath {
  label: string;
  path: string;
}

export type LogLevel = "info" | "success" | "warn" | "error";

export interface LogEntry {
  id: string;
  time: string;
  level: LogLevel;
  message: string;
}
