import { useEffect, useMemo, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { CameraMotionPanel } from "../components/CameraMotionPanel";
import { ExportProgressCard } from "../components/ExportProgressCard";
import { TimelineEditor } from "../components/TimelineEditor";
import { useExportStore } from "../stores/exportStore";
import { useProjectStore } from "../stores/projectStore";
import { useRecordingStore } from "../stores/recordingStore";
import { useSettingsStore } from "../stores/settingsStore";
import type {
  ExportProfile,
  ExportStatus,
  HotkeySettings,
  RecordingProfile,
  RecordingRuntimeStatus
} from "../types/project";

type StudioMode = "recording" | "review";

type StudioPageProps = {
  mode: StudioMode;
  projectId: string | null;
  onBackToDashboard: () => void;
  onStartReRecord: () => void;
};

type BusyAction = "start" | "pause" | "resume" | "stop" | "saveHotkeys" | null;

const initialProfile: RecordingProfile = {
  captureMode: "fullscreen",
  frameRate: 30,
  resolution: "1080p",
  microphoneDeviceId: undefined,
  systemAudioEnabled: false,
  hotkeys: {
    startStop: "Ctrl+Shift+R",
    pauseResume: "Ctrl+Shift+P"
  }
};

const defaultExport: ExportProfile = {
  format: "mp4",
  resolution: "1080p",
  bitrateMbps: 8,
  fps: 30,
  videoCodec: "h264",
  audioCodec: "aac"
};

const recordingStatusText: Record<RecordingRuntimeStatus, string> = {
  idle: "待开始",
  recording: "录制中",
  paused: "已暂停",
  stopped: "已完成",
  error: "异常"
};

const exportStatusText: Record<ExportStatus, string> = {
  queued: "排队中",
  running: "导出中",
  fallback: "软编回退",
  success: "已完成",
  failed: "失败"
};

function formatDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function prettyStatus(status: RecordingRuntimeStatus) {
  if (status === "recording") return "status-running";
  if (status === "paused") return "status-fallback";
  if (status === "stopped") return "status-success";
  if (status === "error") return "status-failed";
  return "status-queued";
}

function exportTone(status: ExportStatus) {
  if (status === "success") return "status-success";
  if (status === "failed") return "status-failed";
  if (status === "fallback") return "status-fallback";
  if (status === "running") return "status-running";
  return "status-queued";
}

function resolveOutputLabel(resolution: ExportProfile["resolution"], fps: ExportProfile["fps"]) {
  if (resolution === "1080p") {
    return fps === 60 ? "1080p / 60fps / 12Mbps" : "1080p / 30fps / 8Mbps";
  }
  return fps === 60 ? "720p / 60fps / 8Mbps" : "720p / 30fps / 4Mbps";
}

export function StudioPage({
  mode,
  projectId,
  onBackToDashboard,
  onStartReRecord
}: StudioPageProps) {
  const {
    status: recordingStatus,
    durationMs,
    detail: recordingDetail,
    degradeMessage,
    error: recordingError,
    startRecording,
    pauseRecording,
    resumeRecording,
    stopRecording
  } = useRecordingStore();
  const {
    capability,
    audioDevices,
    hotkeys: savedHotkeys,
    saveHotkeys
  } = useSettingsStore();
  const {
    currentProjectId,
    manifest,
    loadProject,
    updateTimeline,
    updateCameraMotion,
    flushUpdates
  } = useProjectStore();
  const { taskId, status, progress, detail, error, startExport, retryExport } = useExportStore();

  const [recordingProfile, setRecordingProfile] = useState<RecordingProfile>(initialProfile);
  const [hotkeys, setHotkeys] = useState<HotkeySettings>({
    startStop: "Ctrl+Shift+R",
    pauseResume: "Ctrl+Shift+P"
  });
  const [busyAction, setBusyAction] = useState<BusyAction>(null);
  const [recordingActionError, setRecordingActionError] = useState<string | null>(null);
  const [loadingProject, setLoadingProject] = useState(false);
  const [loadProjectError, setLoadProjectError] = useState<string | null>(null);
  const [projectTitle, setProjectTitle] = useState("");
  const [savingTitle, setSavingTitle] = useState(false);
  const [titleHint, setTitleHint] = useState<string | null>(null);
  const [exportProfile, setExportProfile] = useState<ExportProfile>(defaultExport);
  const [submittingExport, setSubmittingExport] = useState(false);
  const [checkingGate, setCheckingGate] = useState(false);
  const [qualityGate, setQualityGate] = useState<{ passed: boolean; reasons: string[] } | null>(
    null
  );
  const [actionError, setActionError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [titleDirty, setTitleDirty] = useState(false);
  const [exportDirty, setExportDirty] = useState(false);
  const [hydratedProjectId, setHydratedProjectId] = useState<string | null>(null);

  useEffect(() => {
    setHotkeys(savedHotkeys);
    setRecordingProfile((prev) => ({ ...prev, hotkeys: savedHotkeys }));
  }, [savedHotkeys]);

  useEffect(() => {
    if (mode !== "review" || !projectId) {
      return;
    }
    setLoadingProject(true);
    setLoadProjectError(null);
    void loadProject(projectId)
      .catch((loadError) => {
        setLoadProjectError(`加载项目失败：${String(loadError)}`);
      })
      .finally(() => {
        setLoadingProject(false);
      });
  }, [mode, projectId, loadProject]);

  useEffect(() => {
    if (mode !== "review") {
      return;
    }
    setQualityGate(null);
    setActionError(null);
    setTitleHint(null);
    setTitleDirty(false);
    setExportDirty(false);
    setHydratedProjectId(null);
  }, [mode, projectId]);

  useEffect(() => {
    if (!manifest || mode !== "review" || !projectId || currentProjectId !== projectId) {
      return;
    }
    if (hydratedProjectId !== projectId) {
      setProjectTitle(manifest.title?.trim() || `Project_${projectId.slice(0, 8)}`);
      setExportProfile(manifest.export ?? defaultExport);
      setTitleDirty(false);
      setExportDirty(false);
      setHydratedProjectId(projectId);
      return;
    }
    if (!titleDirty) {
      setProjectTitle(manifest.title?.trim() || `Project_${projectId.slice(0, 8)}`);
    }
    if (!exportDirty) {
      setExportProfile(manifest.export ?? defaultExport);
    }
  }, [
    manifest,
    mode,
    projectId,
    currentProjectId,
    hydratedProjectId,
    titleDirty,
    exportDirty
  ]);

  useEffect(() => {
    if (recordingError?.message) {
      setToast(recordingError.message);
    }
  }, [recordingError?.message]);

  useEffect(() => {
    if (error) {
      setToast(error);
    }
  }, [error]);

  useEffect(() => {
    if (!toast) {
      return;
    }
    const timer = window.setTimeout(() => setToast(null), 3600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const runAction = async (
    action: Exclude<BusyAction, null>,
    task: () => Promise<void>
  ) => {
    if (busyAction) {
      return;
    }
    setBusyAction(action);
    setRecordingActionError(null);
    try {
      await task();
    } catch (runError) {
      setRecordingActionError(String(runError));
    } finally {
      setBusyAction(null);
    }
  };

  const previewSrc = useMemo(() => {
    const rawPath = manifest?.artifacts.rawRecordingPath;
    if (!rawPath) {
      return null;
    }
    try {
      return convertFileSrc(rawPath);
    } catch {
      return null;
    }
  }, [manifest?.artifacts.rawRecordingPath]);

  const saveProjectTitle = async () => {
    if (!projectId) {
      return;
    }
    const trimmed = projectTitle.trim();
    setSavingTitle(true);
    setTitleHint(null);
    try {
      await invoke("update_project_title", { projectId, title: trimmed });
      setTitleDirty(false);
      setTitleHint(trimmed ? "项目名称已保存" : "已清空标题，将使用默认项目名");
      await loadProject(projectId);
    } catch (saveError) {
      setTitleHint(`保存失败：${String(saveError)}`);
    } finally {
      setSavingTitle(false);
    }
  };

  const exportTaskActive = Boolean(
    taskId && (status === "queued" || status === "running" || status === "fallback")
  );

  const setExportProfilePatch = (patch: Partial<ExportProfile>) => {
    setExportDirty(true);
    setExportProfile((prev) => ({ ...prev, ...patch }));
  };

  const submitExport = async () => {
    if (!projectId || submittingExport || exportTaskActive) {
      return;
    }
    setSubmittingExport(true);
    setActionError(null);
    try {
      await loadProject(projectId);
      await flushUpdates();
      await startExport(projectId, exportProfile);
      setExportDirty(false);
    } catch (submitError) {
      setActionError(`导出准备失败：${String(submitError)}`);
    } finally {
      setSubmittingExport(false);
    }
  };

  const checkQualityGate = async () => {
    if (!projectId || checkingGate) {
      return;
    }
    setCheckingGate(true);
    setActionError(null);
    try {
      const result = await invoke<{ passed: boolean; reasons: string[] }>("validate_quality_gate", {
        projectId
      });
      setQualityGate(result);
    } catch (gateError) {
      setActionError(`质量门槛检查失败：${String(gateError)}`);
    } finally {
      setCheckingGate(false);
    }
  };

  if (mode === "recording") {
    return (
      <section className="studio-shell">
        <header className="studio-topbar">
          <button className="btn-outline" onClick={onBackToDashboard}>
            返回列表
          </button>
          <p className="studio-mode-tag">Recording Mode</p>
          <span className={`status-chip ${prettyStatus(recordingStatus)}`}>
            {recordingStatusText[recordingStatus]}
          </span>
        </header>

        <div className="recording-layout">
          <div className="preview-canvas">
            <div className="preview-overlay">
              <p className="preview-title">录制预览区</p>
              <p className="muted">时长 {formatDuration(durationMs)}</p>
              <p className="muted">{recordingDetail}</p>
              {degradeMessage && <p className="warn">{degradeMessage}</p>}
            </div>
          </div>

          <aside className="recording-panel">
            <h2>录制控制台</h2>
            <div className="stack">
              <label>
                录制区域
                <select
                  value={recordingProfile.captureMode}
                  onChange={(event) =>
                    setRecordingProfile((prev) => ({
                      ...prev,
                      captureMode: event.target.value as RecordingProfile["captureMode"]
                    }))
                  }
                >
                  <option value="fullscreen">全屏</option>
                  <option value="window">窗口</option>
                </select>
              </label>
              <label>
                分辨率
                <select
                  value={recordingProfile.resolution}
                  onChange={(event) =>
                    setRecordingProfile((prev) => ({
                      ...prev,
                      resolution: event.target.value as RecordingProfile["resolution"]
                    }))
                  }
                >
                  <option value="1080p">1080p</option>
                  <option value="720p">720p</option>
                </select>
              </label>
              <label>
                帧率
                <select
                  value={recordingProfile.frameRate}
                  onChange={(event) =>
                    setRecordingProfile((prev) => ({
                      ...prev,
                      frameRate: Number(event.target.value) as 30 | 60
                    }))
                  }
                >
                  <option value={30}>30 FPS</option>
                  <option value={60}>60 FPS</option>
                </select>
              </label>
              <label>
                麦克风
                <select
                  value={recordingProfile.microphoneDeviceId ?? "__none__"}
                  onChange={(event) =>
                    setRecordingProfile((prev) => ({
                      ...prev,
                      microphoneDeviceId:
                        event.target.value === "__none__" ? undefined : event.target.value
                    }))
                  }
                >
                  <option value="__none__">不使用麦克风</option>
                  {audioDevices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="row toggle-row">
                <input
                  type="checkbox"
                  checked={recordingProfile.systemAudioEnabled}
                  disabled={!capability?.supportsSystemAudio}
                  onChange={(event) =>
                    setRecordingProfile((prev) => ({
                      ...prev,
                      systemAudioEnabled: event.target.checked
                    }))
                  }
                />
                系统音频
              </label>
            </div>

            <div className="recording-hotkeys">
              <label>
                开始/停止快捷键
                <input
                  value={hotkeys.startStop}
                  onChange={(event) =>
                    setHotkeys((prev) => ({ ...prev, startStop: event.target.value }))
                  }
                />
              </label>
              <label>
                暂停/继续快捷键
                <input
                  value={hotkeys.pauseResume}
                  onChange={(event) =>
                    setHotkeys((prev) => ({ ...prev, pauseResume: event.target.value }))
                  }
                />
              </label>
              <button
                className="btn-outline"
                onClick={() =>
                  void runAction("saveHotkeys", async () => {
                    await saveHotkeys(hotkeys);
                    setRecordingProfile((prev) => ({ ...prev, hotkeys }));
                  })
                }
              >
                {busyAction === "saveHotkeys" ? "保存中..." : "保存快捷键"}
              </button>
            </div>

            <div className="recording-main-actions">
              {(recordingStatus === "idle" ||
                recordingStatus === "error" ||
                recordingStatus === "stopped") && (
                <button
                  className="record-button"
                  onClick={() =>
                    void runAction("start", () =>
                      startRecording({
                        ...recordingProfile,
                        hotkeys
                      })
                    )
                  }
                  disabled={busyAction !== null}
                >
                  {busyAction === "start" ? "启动中..." : "开始录制"}
                </button>
              )}
              {recordingStatus === "recording" && (
                <>
                  <button
                    className="btn-outline"
                    onClick={() => void runAction("pause", pauseRecording)}
                    disabled={busyAction !== null}
                  >
                    {busyAction === "pause" ? "暂停中..." : "暂停"}
                  </button>
                  <button
                    className="danger"
                    onClick={() => void runAction("stop", stopRecording)}
                    disabled={busyAction !== null}
                  >
                    {busyAction === "stop" ? "停止中..." : "停止录制"}
                  </button>
                </>
              )}
              {recordingStatus === "paused" && (
                <>
                  <button
                    className="btn-outline"
                    onClick={() => void runAction("resume", resumeRecording)}
                    disabled={busyAction !== null}
                  >
                    {busyAction === "resume" ? "继续中..." : "继续"}
                  </button>
                  <button
                    className="danger"
                    onClick={() => void runAction("stop", stopRecording)}
                    disabled={busyAction !== null}
                  >
                    {busyAction === "stop" ? "停止中..." : "停止录制"}
                  </button>
                </>
              )}
            </div>

            {recordingActionError && <p className="error">{recordingActionError}</p>}
          </aside>
        </div>

        {toast && <div className="toast">{toast}</div>}
      </section>
    );
  }

  return (
    <section className="studio-shell">
      <header className="studio-topbar">
        <button className="btn-outline" onClick={onBackToDashboard}>
          返回列表
        </button>
        <p className="studio-mode-tag">Review & Export Mode</p>
        <div className="row gap">
          <button className="btn-outline" onClick={onStartReRecord}>
            重新录制
          </button>
          <button
            className="btn-primary"
            onClick={() => void submitExport()}
            disabled={!projectId || submittingExport || exportTaskActive}
          >
            {submittingExport ? "准备中..." : exportTaskActive ? "导出进行中..." : "立即导出"}
          </button>
        </div>
      </header>

      {!projectId && (
        <div className="panel">
          <p>当前没有项目，请从列表选择一个项目。</p>
        </div>
      )}

      {projectId && (
        <>
          <div className="studio-title-row">
            <label>
              项目名称
              <input
                value={projectTitle}
                onChange={(event) => {
                  setTitleDirty(true);
                  setProjectTitle(event.target.value);
                }}
              />
            </label>
            <button className="btn-outline" onClick={() => void saveProjectTitle()} disabled={savingTitle}>
              {savingTitle ? "保存中..." : "保存名称"}
            </button>
            <code>{projectId}</code>
          </div>
          {titleHint && <p className="note">{titleHint}</p>}

          {loadingProject && <p className="note">加载项目中...</p>}
          {loadProjectError && <p className="error">{loadProjectError}</p>}

          <div className="review-layout">
            <div className="review-left">
              <div className="video-shell">
                {previewSrc ? (
                  <video controls src={previewSrc} />
                ) : (
                  <div className="video-fallback">
                    <p>未找到可预览的视频文件</p>
                    <p className="muted">请先完成录制后再进入预览导出模式。</p>
                  </div>
                )}
              </div>
              {manifest && (
                <>
                  <TimelineEditor timeline={manifest.timeline} onChange={updateTimeline} />
                  <CameraMotionPanel profile={manifest.cameraMotion} onChange={updateCameraMotion} />
                </>
              )}
            </div>

            <aside className="review-right">
              <div className="panel stack">
                <div className="panel-head">
                  <strong>导出设置</strong>
                  <span className={`status-chip ${taskId ? exportTone(status) : "status-queued"}`}>
                    {taskId ? exportStatusText[status] : "待开始"}
                  </span>
                </div>
                <label>
                  质量预设
                  <select
                    value={resolveOutputLabel(exportProfile.resolution, exportProfile.fps)}
                    onChange={(event) => {
                      const value = event.target.value;
                      if (value.includes("1080p / 60")) {
                        setExportProfilePatch({
                          resolution: "1080p",
                          fps: 60,
                          bitrateMbps: 12
                        });
                        return;
                      }
                      if (value.includes("1080p / 30")) {
                        setExportProfilePatch({
                          resolution: "1080p",
                          fps: 30,
                          bitrateMbps: 8
                        });
                        return;
                      }
                      if (value.includes("720p / 60")) {
                        setExportProfilePatch({
                          resolution: "720p",
                          fps: 60,
                          bitrateMbps: 8
                        });
                        return;
                      }
                      setExportProfilePatch({
                        resolution: "720p",
                        fps: 30,
                        bitrateMbps: 4
                      });
                    }}
                  >
                    <option value="1080p / 60fps / 12Mbps">高画质（1080p / 60fps）</option>
                    <option value="1080p / 30fps / 8Mbps">平衡（1080p / 30fps）</option>
                    <option value="720p / 60fps / 8Mbps">流畅（720p / 60fps）</option>
                    <option value="720p / 30fps / 4Mbps">小体积（720p / 30fps）</option>
                  </select>
                </label>
                <label>
                  分辨率
                  <select
                    value={exportProfile.resolution}
                    onChange={(event) =>
                      setExportProfilePatch({
                        resolution: event.target.value as ExportProfile["resolution"]
                      })
                    }
                  >
                    <option value="1080p">1080p</option>
                    <option value="720p">720p</option>
                  </select>
                </label>
                <label>
                  帧率
                  <select
                    value={exportProfile.fps}
                    onChange={(event) =>
                      setExportProfilePatch({
                        fps: Number(event.target.value) as ExportProfile["fps"]
                      })
                    }
                  >
                    <option value={30}>30 FPS</option>
                    <option value={60}>60 FPS</option>
                  </select>
                </label>
                <label>
                  码率（Mbps）
                  <input
                    type="number"
                    min={2}
                    max={24}
                    step={1}
                    value={exportProfile.bitrateMbps}
                    onChange={(event) =>
                      setExportProfilePatch({
                        bitrateMbps: Math.min(
                          24,
                          Math.max(2, Number(event.target.value) || exportProfile.bitrateMbps)
                        )
                      })
                    }
                  />
                </label>

                <div className="row gap wrap">
                  <button
                    className="btn-primary"
                    onClick={() => void submitExport()}
                    disabled={submittingExport || exportTaskActive}
                  >
                    {submittingExport ? "准备中..." : exportTaskActive ? "导出中..." : "导出视频"}
                  </button>
                  <button
                    className="btn-outline"
                    onClick={() => void checkQualityGate()}
                    disabled={checkingGate}
                  >
                    {checkingGate ? "检查中..." : "检查质量门槛"}
                  </button>
                </div>
                {actionError && <p className="error">{actionError}</p>}
              </div>

              <ExportProgressCard
                status={status}
                progress={progress}
                detail={detail}
                onRetry={retryExport}
              />

              {qualityGate && (
                <div className="panel stack">
                  <strong>质量门槛</strong>
                  <p>{qualityGate.passed ? "通过" : "未通过"}</p>
                  {!qualityGate.passed &&
                    qualityGate.reasons.map((reason) => (
                      <p key={reason} className="warn">
                        {reason}
                      </p>
                    ))}
                </div>
              )}
            </aside>
          </div>
        </>
      )}

      {toast && <div className="toast">{toast}</div>}
    </section>
  );
}
