import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  CameraMotionProfile,
  ProjectManifest,
  TimelineConfig
} from "../types/project";

let projectWriteQueue: Promise<void> = Promise.resolve();
let projectLoadSerial = 0;

type ProjectStore = {
  currentProjectId: string | null;
  manifest: ProjectManifest | null;
  loadProject: (projectId: string) => Promise<void>;
  updateTimeline: (patch: Partial<TimelineConfig>) => Promise<void>;
  updateCameraMotion: (patch: Partial<CameraMotionProfile>) => Promise<void>;
  flushUpdates: () => Promise<void>;
};

export const useProjectStore = create<ProjectStore>((set, get) => ({
  currentProjectId: null,
  manifest: null,
  loadProject: async (projectId) => {
    const loadSerial = ++projectLoadSerial;
    set({ currentProjectId: projectId, manifest: null });
    const manifest = await invoke<ProjectManifest>("load_project", { projectId });
    if (loadSerial !== projectLoadSerial || get().currentProjectId !== projectId) {
      return;
    }
    set({ currentProjectId: projectId, manifest });
  },
  updateTimeline: async (patch) => {
    const { currentProjectId } = get();
    if (!currentProjectId) {
      return;
    }
    const targetProjectId = currentProjectId;
    projectWriteQueue = projectWriteQueue
      .catch(() => undefined)
      .then(async () => {
        await invoke("update_timeline", { projectId: targetProjectId, patch });
        if (get().currentProjectId !== targetProjectId) {
          return;
        }
        const latest = get().manifest;
        if (!latest) {
          return;
        }
        set({
          manifest: {
            ...latest,
            timeline: { ...latest.timeline, ...patch }
          }
        });
      });
    await projectWriteQueue;
  },
  updateCameraMotion: async (patch) => {
    const { currentProjectId } = get();
    if (!currentProjectId) {
      return;
    }
    const targetProjectId = currentProjectId;
    projectWriteQueue = projectWriteQueue
      .catch(() => undefined)
      .then(async () => {
        await invoke("update_camera_motion", { projectId: targetProjectId, patch });
        if (get().currentProjectId !== targetProjectId) {
          return;
        }
        const latest = get().manifest;
        if (!latest) {
          return;
        }
        set({
          manifest: {
            ...latest,
            cameraMotion: { ...latest.cameraMotion, ...patch }
          }
        });
      });
    await projectWriteQueue;
  },
  flushUpdates: async () => {
    await projectWriteQueue;
  }
}));
