import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  HotkeySettings,
  PlatformCapability,
  RecordingDevice
} from "../types/project";

type SettingsStore = {
  capability: PlatformCapability | null;
  audioDevices: RecordingDevice[];
  hotkeys: HotkeySettings;
  loadSettings: () => Promise<void>;
  saveHotkeys: (hotkeys: HotkeySettings) => Promise<void>;
};

export const useSettingsStore = create<SettingsStore>((set) => ({
  capability: null,
  audioDevices: [],
  hotkeys: {
    startStop: "Ctrl+Shift+R",
    pauseResume: "Ctrl+Shift+P"
  },
  loadSettings: async () => {
    const [capabilityResult, audioDevicesResult, hotkeysResult] = await Promise.allSettled([
      invoke<PlatformCapability>("get_platform_capability"),
      invoke<RecordingDevice[]>("list_audio_input_devices"),
      invoke<HotkeySettings>("load_hotkeys")
    ]);

    set({
      capability: capabilityResult.status === "fulfilled" ? capabilityResult.value : null,
      audioDevices:
        audioDevicesResult.status === "fulfilled"
          ? audioDevicesResult.value
          : [],
      hotkeys:
        hotkeysResult.status === "fulfilled"
          ? hotkeysResult.value
          : {
              startStop: "Ctrl+Shift+R",
              pauseResume: "Ctrl+Shift+P"
            }
    });
  },
  saveHotkeys: async (hotkeys) => {
    await invoke("save_hotkeys", { hotkeys });
    set({ hotkeys });
  }
}));
