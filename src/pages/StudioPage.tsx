import { useEffect, useMemo, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useExportStore } from "../stores/exportStore";
import { useProjectStore } from "../stores/projectStore";
import { useRecordingStore } from "../stores/recordingStore";
import { useSettingsStore } from "../stores/settingsStore";
import { RecordingView } from "./studio/RecordingView";
import { ReviewView } from "./studio/ReviewView";
import type {
  ExportProfile,
  ExportStatus,
  HotkeySettings,
  RecordingProfile,
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

const exportStatusMap: Record<ExportStatus, string> = {
  queued: "queued",
  running: "running",
  fallback: "running",
  success: "success",
  failed: "failed"
};

const exportStatusText: Record<ExportStatus, string> = {
  queued: "等待队列",
  running: "正在导出",
  fallback: "软编兼容模式",
  success: "导出成功",
  failed: "导出失败"
};

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

  const { taskId, status: exportStatus, progress, detail: exportDetail, error: exportError, startExport, retryExport } = useExportStore();

  const [recordingProfile, setRecordingProfile] = useState<RecordingProfile>(initialProfile);
  const [hotkeys, setHotkeys] = useState<HotkeySettings>(savedHotkeys);
  const [busyAction, setBusyAction] = useState<BusyAction>(null);
  const [loadingProject, setLoadingProject] = useState(false);
  const [projectTitle, setProjectTitle] = useState("");
  const [savingTitle, setSavingTitle] = useState(false);
  const [titleHint, setTitleHint] = useState<string | null>(null);
  const [exportProfile, setExportProfile] = useState<ExportProfile>(defaultExport);
  const [submittingExport, setSubmittingExport] = useState(false);
  const [checkingGate, setCheckingGate] = useState(false);
  const [qualityGate, setQualityGate] = useState<{ passed: boolean; reasons: string[] } | null>(null);
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
    if (mode !== "review" || !projectId) return;
    setLoadingProject(true);
    void loadProject(projectId).finally(() => setLoadingProject(false));
  }, [mode, projectId, loadProject]);

  useEffect(() => {
    if (mode !== "review") return;
    setQualityGate(null);
    setActionError(null);
    setTitleHint(null);
    setTitleDirty(false);
    setExportDirty(false);
    setHydratedProjectId(null);
  }, [mode, projectId]);

  useEffect(() => {
    if (!manifest || mode !== "review" || !projectId || currentProjectId !== projectId) return;
    if (hydratedProjectId !== projectId) {
      setProjectTitle(manifest.title?.trim() || `Project_${projectId.slice(0, 8)}`);
      setExportProfile(manifest.export ?? defaultExport);
      setTitleDirty(false);
      setExportDirty(false);
      setHydratedProjectId(projectId);
      return;
    }
    if (!titleDirty) setProjectTitle(manifest.title?.trim() || `Project_${projectId.slice(0, 8)}`);
    if (!exportDirty) setExportProfile(manifest.export ?? defaultExport);
  }, [manifest, mode, projectId, currentProjectId, hydratedProjectId, titleDirty, exportDirty]);

  useEffect(() => {
    if (recordingError?.message) setToast(recordingError.message);
  }, [recordingError?.message]);

  useEffect(() => {
    if (exportError) setToast(exportError);
  }, [exportError]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 3600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const runAction = async (action: Exclude<BusyAction, null>, task: () => Promise<void>) => {
    if (busyAction) return;
    setBusyAction(action);
    try {
      await task();
    } catch (e) {
      console.error(e);
    } finally {
      setBusyAction(null);
    }
  };

  const previewSrc = useMemo(() => {
    const rawPath = manifest?.artifacts.rawRecordingPath;
    if (!rawPath) return null;
    try {
      return convertFileSrc(rawPath);
    } catch {
      return null;
    }
  }, [manifest?.artifacts.rawRecordingPath]);

  const saveProjectTitle = async () => {
    if (!projectId) return;
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

  const handleStartExport = async () => {
    if (!projectId || submittingExport) return;
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

  const handleCheckQualityGate = async () => {
    if (!projectId || checkingGate) return;
    setCheckingGate(true);
    setActionError(null);
    try {
      const result = await invoke<{ passed: boolean; reasons: string[] }>("validate_quality_gate", { projectId });
      setQualityGate(result);
    } catch (gateError) {
      setActionError(`质量门槛检查失败：${String(gateError)}`);
    } finally {
      setCheckingGate(false);
    }
  };

  return (
    <>
      {mode === "recording" ? (
        <RecordingView
          status={recordingStatus}
          durationMs={durationMs}
          detail={recordingDetail}
          degradeMessage={degradeMessage ?? null}
          recordingProfile={recordingProfile}
          setRecordingProfile={setRecordingProfile}
          hotkeys={hotkeys}
          setHotkeys={setHotkeys}
          audioDevices={audioDevices}
          capability={capability}
          busyAction={busyAction}
          onRunAction={runAction}
          onStartRecording={startRecording}
          onPauseRecording={pauseRecording}
          onResumeRecording={resumeRecording}
          onStopRecording={stopRecording}
          onSaveHotkeys={saveHotkeys}
          onBackToDashboard={onBackToDashboard}
        />
      ) : (
        <ReviewView
          projectId={projectId || ""}
          projectTitle={projectTitle}
          setProjectTitle={setProjectTitle}
          titleDirty={titleDirty}
          setTitleDirty={setTitleDirty}
          titleHint={titleHint}
          onSaveTitle={saveProjectTitle}
          previewSrc={previewSrc}
          manifest={manifest}
          updateTimeline={updateTimeline}
          updateCameraMotion={updateCameraMotion}
          exportProfile={exportProfile}
          setExportProfilePatch={(patch) => {
            setExportDirty(true);
            setExportProfile(prev => ({ ...prev, ...patch }));
          }}
          exportStatus={exportStatus}
          exportStatusText={exportStatusText}
          exportStatusMap={exportStatusMap}
          taskId={taskId}
          progress={progress}
          exportDetail={exportDetail}
          onRetryExport={retryExport}
          onStartExport={handleStartExport}
          onCheckQualityGate={handleCheckQualityGate}
          checkingGate={checkingGate}
          qualityGate={qualityGate}
          actionError={actionError}
          submittingExport={submittingExport}
          onBackToDashboard={onBackToDashboard}
          onStartReRecord={onStartReRecord}
        />
      )}
      {toast && <div className="toast glass">{toast}</div>}
    </>
  );
}
