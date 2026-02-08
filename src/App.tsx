import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { DashboardPage } from "./pages/DashboardPage";
import { StudioPage } from "./pages/StudioPage";
import { useExportStore } from "./stores/exportStore";
import { useRecordingStore } from "./stores/recordingStore";
import { useSettingsStore } from "./stores/settingsStore";
import type { ExportStatus, ProjectListItem, RecordingStatusEvent } from "./types/project";

type AppView = "dashboard" | "studio";
type StudioMode = "recording" | "review";

type ExportProgressEvent = {
  taskId: string;
  status: ExportStatus;
  progress: number;
  detail: string;
};

function App() {
  const [view, setView] = useState<AppView>("dashboard");
  const [studioMode, setStudioMode] = useState<StudioMode>("recording");
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [loadingProjects, setLoadingProjects] = useState(false);
  const [projectListError, setProjectListError] = useState<string | null>(null);

  const setProgress = useExportStore((state) => state.setProgress);
  const exportStatus = useExportStore((state) => state.status);
  const syncRecording = useRecordingStore((state) => state.syncFromEvent);
  const recordingStatus = useRecordingStore((state) => state.status);
  const recordingProjectId = useRecordingStore((state) => state.projectId);
  const loadSettings = useSettingsStore((state) => state.loadSettings);

  const refreshProjects = useCallback(async () => {
    setLoadingProjects(true);
    setProjectListError(null);
    try {
      const rows = await invoke<ProjectListItem[]>("list_projects");
      setProjects(rows);
    } catch (error) {
      setProjectListError(`读取项目列表失败：${String(error)}`);
    } finally {
      setLoadingProjects(false);
    }
  }, []);

  useEffect(() => {
    void loadSettings().catch(() => undefined);
    void refreshProjects();
  }, [loadSettings, refreshProjects]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;
    void listen<ExportProgressEvent>("export/progress", (event) => {
      setProgress(
        event.payload.taskId,
        event.payload.status,
        event.payload.progress,
        event.payload.detail
      );
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
    });
    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [setProgress]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;
    void listen<RecordingStatusEvent>("recording/status", (event) => {
      syncRecording(event.payload);
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
    });
    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [syncRecording]);

  useEffect(() => {
    if (recordingStatus === "stopped" && recordingProjectId) {
      setActiveProjectId(recordingProjectId);
      setView("studio");
      setStudioMode("review");
      void refreshProjects();
    }
  }, [recordingStatus, recordingProjectId, refreshProjects]);

  useEffect(() => {
    if (exportStatus === "success") {
      void refreshProjects();
    }
  }, [exportStatus, refreshProjects]);

  const openDashboard = () => {
    setView("dashboard");
    void refreshProjects();
  };

  const startNewRecording = () => {
    setActiveProjectId(null);
    setStudioMode("recording");
    setView("studio");
  };

  const openProjectInStudio = (projectId: string) => {
    setActiveProjectId(projectId);
    setStudioMode("review");
    setView("studio");
  };

  const deleteProject = async (projectId: string) => {
    await invoke("delete_project", { projectId });
    if (activeProjectId === projectId) {
      setActiveProjectId(null);
      setStudioMode("recording");
    }
    await refreshProjects();
  };

  const renameProject = async (projectId: string, title: string) => {
    await invoke("update_project_title", { projectId, title });
    await refreshProjects();
  };

  return (
    <main className="app-shell">
      {view === "dashboard" ? (
        <DashboardPage
          projects={projects}
          loading={loadingProjects}
          error={projectListError}
          onRefresh={refreshProjects}
          onNewRecording={startNewRecording}
          onOpenProject={openProjectInStudio}
          onDeleteProject={deleteProject}
          onRenameProject={renameProject}
        />
      ) : (
        <StudioPage
          mode={studioMode}
          projectId={activeProjectId}
          onBackToDashboard={openDashboard}
          onStartReRecord={() => {
            setActiveProjectId(null);
            setStudioMode("recording");
          }}
        />
      )}
    </main>
  );
}

export default App;

