export type CaptureMode = "fullscreen" | "window";
export type Resolution = "1080p" | "720p";
export type AspectRatio = "16:9" | "9:16" | "1:1";
export type CameraIntensity = "low" | "medium" | "high";

export type RecordingProfile = {
  captureMode: CaptureMode;
  windowTarget?: string;
  frameRate: 30 | 60;
  resolution: Resolution;
  microphoneDeviceId?: string;
  systemAudioEnabled: boolean;
  hotkeys: {
    startStop: string;
    pauseResume: string;
  };
};

export type CameraMotionProfile = {
  enabled: boolean;
  intensity: CameraIntensity;
  smoothing: number;
  maxZoom: number;
  idleThresholdMs: number;
};

export type ExportProfile = {
  format: "mp4";
  resolution: Resolution;
  bitrateMbps: number;
  fps: 30 | 60;
  videoCodec: "h264";
  audioCodec: "aac";
};

export type TimelineConfig = {
  trimStartMs: number;
  trimEndMs: number;
  aspectRatio: AspectRatio;
  cursorHighlightEnabled: boolean;
};

export type ProjectManifest = {
  schemaVersion: number;
  appVersion: string;
  title?: string | null;
  createdAt: string;
  updatedAt: string;
  recording: RecordingProfile;
  cameraMotion: CameraMotionProfile;
  export: ExportProfile;
  timeline: TimelineConfig;
  artifacts: {
    rawRecordingPath?: string;
    cursorTrackPath?: string;
    lastExportPath?: string;
    exportLogPath?: string;
  };
  quality: {
    avOffsetMs: number;
    avgDropRate: number;
    peakDropRate: number;
  };
  status: ProjectStatus;
  lastError?: AppError | null;
};

export type ProjectStatus =
  | "recording"
  | "ready_to_edit"
  | "exporting"
  | "export_failed"
  | "export_succeeded";

export type ProjectListItem = {
  projectId: string;
  title?: string | null;
  createdAt: string;
  updatedAt: string;
  status: ProjectStatus;
  durationMs: number;
  hasExport: boolean;
  exportPath?: string | null;
  rawPath?: string | null;
};

export type RecoverableProject = {
  projectId: string;
  reason: string;
  path: string;
};

export type RecordingRuntimeStatus =
  | "idle"
  | "recording"
  | "paused"
  | "stopped"
  | "error";

export type ExportStatus =
  | "queued"
  | "running"
  | "fallback"
  | "success"
  | "failed";

export type AppError = {
  code: string;
  message: string;
  suggestion?: string;
};

export type PlatformCapability = {
  platform: string;
  supportsScreenCapture: boolean;
  supportsWindowCapture: boolean;
  supportsMicrophone: boolean;
  supportsSystemAudio: boolean;
  systemAudioDegradeMessage?: string;
};

export type RecordingDevice = {
  id: string;
  label: string;
  kind: string;
};

export type HotkeySettings = {
  startStop: string;
  pauseResume: string;
};

export type RecordingStatusEvent = {
  sessionId: string;
  status: RecordingRuntimeStatus;
  durationMs: number;
  sourceLabel: string;
  detail: string;
  degradeMessage?: string;
};
