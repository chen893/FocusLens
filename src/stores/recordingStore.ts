import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  RecordingProfile,
  RecordingRuntimeStatus,
  RecordingStatusEvent
} from "../types/project";
import { normalizeInvokeError } from "../utils/tauriError";

type RecordingStore = {
  sessionId: string | null;
  projectId: string | null;
  status: RecordingRuntimeStatus;
  durationMs: number;
  sourceLabel: string;
  detail: string;
  degradeMessage?: string;
  error?: AppError;
  setStatus: (status: RecordingRuntimeStatus) => void;
  syncFromEvent: (payload: RecordingStatusEvent) => void;
  startRecording: (profile: RecordingProfile) => Promise<void>;
  pauseRecording: () => Promise<void>;
  resumeRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
};

export const useRecordingStore = create<RecordingStore>((set, get) => ({
  sessionId: null,
  projectId: null,
  status: "idle",
  durationMs: 0,
  sourceLabel: "未开始",
  detail: "等待开始录制",
  setStatus: (status) => set({ status }),
  syncFromEvent: (payload) =>
    set({
      status: payload.status,
      durationMs: payload.durationMs,
      sourceLabel: payload.sourceLabel,
      detail: payload.detail,
      degradeMessage: payload.degradeMessage,
      sessionId:
        payload.status === "stopped" || payload.status === "error"
          ? null
          : payload.sessionId
    }),
  startRecording: async (profile) => {
    try {
      const sessionId = await invoke<string>("start_recording", { profile });
      set({
        sessionId,
        projectId: null,
        status: "recording",
        durationMs: 0,
        sourceLabel: profile.captureMode === "fullscreen" ? "全屏" : "窗口",
        detail: "录制已开始",
        degradeMessage: undefined,
        error: undefined
      });
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "START_RECORDING_FAIL",
        "开始录制失败，请检查录制权限和音频设置"
      );
      set({
        sessionId: null,
        status: "error",
        detail: "开始录制失败",
        error: parsed
      });
    }
  },
  pauseRecording: async () => {
    const { sessionId } = get();
    if (!sessionId) {
      set({
        status: "error",
        detail: "当前没有可暂停的录制会话，请重新开始录制",
        error: {
          code: "SESSION_NOT_FOUND",
          message: "暂停失败：未找到进行中的录制会话"
        }
      });
      return;
    }
    try {
      await invoke("pause_recording", { sessionId });
      set({ status: "paused", detail: "录制已暂停", error: undefined });
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "PAUSE_RECORDING_FAIL",
        "暂停录制失败"
      );
      set({
        status: "error",
        detail: "暂停失败",
        sessionId: null,
        error: parsed
      });
    }
  },
  resumeRecording: async () => {
    const { sessionId } = get();
    if (!sessionId) {
      set({
        status: "error",
        detail: "当前没有可继续的录制会话，请重新开始录制",
        error: {
          code: "SESSION_NOT_FOUND",
          message: "继续失败：未找到已暂停的录制会话"
        }
      });
      return;
    }
    try {
      await invoke("resume_recording", { sessionId });
      set({ status: "recording", detail: "录制已继续", error: undefined });
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "RESUME_RECORDING_FAIL",
        "继续录制失败"
      );
      set({
        status: "error",
        detail: "继续失败",
        sessionId: null,
        error: parsed
      });
    }
  },
  stopRecording: async () => {
    const { sessionId } = get();
    if (!sessionId) {
      set({
        status: "error",
        detail: "当前没有可停止的录制会话，请重新开始录制",
        error: {
          code: "SESSION_NOT_FOUND",
          message: "停止失败：未找到进行中的录制会话"
        }
      });
      return;
    }
    try {
      set({ detail: "正在停止录制，请稍候…", error: undefined });
      const projectId = await invoke<string>("stop_recording", { sessionId });
      set({
        status: "stopped",
        projectId,
        sessionId: null,
        detail: "录制已完成",
        error: undefined
      });
    } catch (error) {
      const parsed = normalizeInvokeError(
        error,
        "STOP_RECORDING_FAIL",
        "停止录制失败"
      );
      set({
        status: "error",
        sessionId: null,
        detail: "停止失败",
        error: parsed
      });
    }
  }
}));
