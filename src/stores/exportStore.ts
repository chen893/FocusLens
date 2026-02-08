import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ExportProfile, ExportStatus } from "../types/project";
import { normalizeInvokeError } from "../utils/tauriError";

type ExportTaskStatusSnapshot = {
  taskId: string;
  projectId: string;
  status: ExportStatus;
  retries: number;
  lastError?: {
    code: string;
    message: string;
    suggestion?: string;
  };
};

function detailByStatus(status: ExportStatus): string {
  if (status === "queued") return "导出任务排队中";
  if (status === "running") return "导出处理中";
  if (status === "fallback") return "硬件编码不可用，已回退软件编码";
  if (status === "success") return "导出完成";
  return "导出失败";
}

function progressByStatus(status: ExportStatus): number {
  if (status === "queued") return 0;
  if (status === "running") return 45;
  if (status === "fallback") return 62;
  return 100;
}

let exportPollSerial = 0;

type ExportStore = {
  taskId: string | null;
  status: ExportStatus;
  progress: number;
  detail: string;
  error?: string;
  startExport: (projectId: string, profile: ExportProfile) => Promise<void>;
  retryExport: () => Promise<void>;
  setProgress: (
    taskId: string,
    status: ExportStatus,
    progress: number,
    detail: string
  ) => void;
};

async function pollExportStatus(
  taskId: string,
  set: (partial: Partial<ExportStore>) => void,
  get: () => ExportStore
) {
  const pollSerial = ++exportPollSerial;
  for (let attempt = 0; attempt < 180; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 500));
    if (get().taskId !== taskId || pollSerial !== exportPollSerial) {
      return;
    }
    try {
      const snapshot = await invoke<ExportTaskStatusSnapshot>("get_export_task_status", {
        exportTaskId: taskId
      });
      if (get().taskId !== taskId || pollSerial !== exportPollSerial) {
        return;
      }
      const current = get();
      const statusProgress = progressByStatus(snapshot.status);
      const progress =
        snapshot.status === "running" || snapshot.status === "fallback"
          ? Math.max(current.progress, statusProgress)
          : statusProgress;
      const detail =
        snapshot.status === "failed"
          ? snapshot.lastError?.message ?? detailByStatus(snapshot.status)
          : current.status === snapshot.status && current.detail.trim().length > 0
            ? current.detail
            : detailByStatus(snapshot.status);
      set({
        status: snapshot.status,
        progress,
        detail,
        error:
          snapshot.status === "failed"
            ? snapshot.lastError?.message ?? "导出失败"
            : undefined
      });
      if (snapshot.status === "success" || snapshot.status === "failed") {
        return;
      }
    } catch {
      // 忽略短暂状态查询失败，继续轮询。
    }
  }
}

export const useExportStore = create<ExportStore>((set, get) => ({
  taskId: null,
  status: "queued",
  progress: 0,
  detail: "等待导出",
  error: undefined,
  startExport: async (projectId, profile) => {
    try {
      const taskId = await invoke<string>("start_export", { projectId, profile });
      set({ taskId, status: "queued", progress: 0, detail: "导出队列中", error: undefined });
      void pollExportStatus(taskId, set, get);
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "START_EXPORT_FAIL",
        "导出任务创建失败"
      );
      set({
        taskId: null,
        status: "failed",
        progress: 100,
        detail: "导出任务创建失败",
        error: parsed.message
      });
    }
  },
  retryExport: async () => {
    const { taskId } = get();
    if (!taskId) {
      return;
    }
    try {
      const newTaskId = await invoke<string>("retry_export", { exportTaskId: taskId });
      set({
        taskId: newTaskId,
        status: "queued",
        progress: 0,
        detail: "已创建重试任务",
        error: undefined
      });
      void pollExportStatus(newTaskId, set, get);
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "RETRY_EXPORT_FAIL",
        "创建重试导出任务失败"
      );
      set({
        taskId,
        status: "failed",
        progress: 100,
        detail: "创建重试导出任务失败",
        error: parsed.message
      });
    }
  },
  setProgress: (taskId, status, progress, detail) => {
    const currentTaskId = get().taskId;
    if (currentTaskId && taskId !== currentTaskId) {
      return;
    }
    set({
      taskId: currentTaskId ?? taskId,
      status,
      progress: Math.max(0, Math.min(100, Math.round(progress))),
      detail,
      error: status === "failed" ? detail : undefined
    });
  }
}));
