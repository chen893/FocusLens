import { useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { ProjectListItem, ProjectStatus } from "../types/project";

type DashboardPageProps = {
  projects: ProjectListItem[];
  loading: boolean;
  error: string | null;
  onRefresh: () => Promise<void>;
  onNewRecording: () => void;
  onOpenProject: (projectId: string) => void;
  onDeleteProject: (projectId: string) => Promise<void>;
  onRenameProject: (projectId: string, title: string) => Promise<void>;
};

const statusText: Record<ProjectStatus, string> = {
  recording: "录制中断",
  ready_to_edit: "草稿",
  exporting: "导出中",
  export_failed: "导出失败",
  export_succeeded: "已导出"
};

function formatDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "--";
  }
  return date.toLocaleString("zh-CN", {
    hour12: false,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  });
}

function statusClass(status: ProjectStatus) {
  if (status === "export_succeeded") return "status-success";
  if (status === "export_failed") return "status-failed";
  if (status === "exporting") return "status-running";
  if (status === "recording") return "status-fallback";
  return "status-queued";
}

export function DashboardPage({
  projects,
  loading,
  error,
  onRefresh,
  onNewRecording,
  onOpenProject,
  onDeleteProject,
  onRenameProject
}: DashboardPageProps) {
  const [editingProjectId, setEditingProjectId] = useState<string | null>(null);
  const [titleDraft, setTitleDraft] = useState("");
  const [rowPendingId, setRowPendingId] = useState<string | null>(null);
  const [rowError, setRowError] = useState<string | null>(null);
  const latestProjectTime = useMemo(() => {
    if (!projects.length) {
      return "暂无项目";
    }
    return formatDateTime(projects[0].updatedAt);
  }, [projects]);

  const handleRename = async (projectId: string) => {
    if (!titleDraft.trim()) {
      setRowError("标题不能为空");
      return;
    }
    setRowPendingId(projectId);
    setRowError(null);
    try {
      await onRenameProject(projectId, titleDraft.trim());
      setEditingProjectId(null);
      setTitleDraft("");
    } catch (error) {
      setRowError(String(error));
    } finally {
      setRowPendingId(null);
    }
  };

  const handleDelete = async (projectId: string) => {
    const confirmed = window.confirm("确认删除该项目及其录制文件吗？此操作不可恢复。");
    if (!confirmed) {
      return;
    }
    setRowPendingId(projectId);
    setRowError(null);
    try {
      await onDeleteProject(projectId);
      if (editingProjectId === projectId) {
        setEditingProjectId(null);
      }
    } catch (error) {
      setRowError(String(error));
    } finally {
      setRowPendingId(null);
    }
  };

  const copyExportPath = async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
      setRowError("导出路径已复制到剪贴板");
    } catch {
      setRowError("复制失败，请手动复制路径");
    }
  };

  const downloadExport = (path: string, title: string) => {
    try {
      const href = convertFileSrc(path);
      const anchor = document.createElement("a");
      anchor.href = href;
      anchor.download = `${title}.mp4`;
      anchor.rel = "noopener";
      anchor.click();
    } catch {
      setRowError("无法直接下载，请先复制导出路径");
    }
  };

  return (
    <section className="dashboard-shell">
      <header className="dashboard-hero">
        <div>
          <p className="eyebrow">FocusLens Dashboard</p>
          <h1>录屏项目管理器</h1>
          <p className="muted">最近更新：{latestProjectTime}</p>
        </div>
        <div className="dashboard-actions">
          <button className="btn-outline" onClick={() => void onRefresh()} disabled={loading}>
            {loading ? "刷新中..." : "刷新列表"}
          </button>
          <button className="btn-primary" onClick={onNewRecording}>
            新建录制
          </button>
        </div>
      </header>

      {error && <p className="error">{error}</p>}
      {rowError && <p className="note">{rowError}</p>}

      <div className="project-grid">
        {projects.map((project) => {
          const title = project.title?.trim() || `Project_${project.projectId.slice(0, 8)}`;
          const isEditing = editingProjectId === project.projectId;
          const pending = rowPendingId === project.projectId;
          let previewSrc: string | null = null;
          if (project.rawPath) {
            try {
              previewSrc = convertFileSrc(project.rawPath);
            } catch {
              previewSrc = null;
            }
          }
          return (
            <article className="project-card" key={project.projectId}>
              <div className="project-card-head">
                <span className={`status-chip ${statusClass(project.status)}`}>
                  {statusText[project.status]}
                </span>
                <small className="mono">{project.projectId.slice(0, 8)}</small>
              </div>

              <div className="project-thumb">
                {previewSrc ? (
                  <video src={previewSrc} muted preload="metadata" />
                ) : (
                  <div className="project-thumb-fallback">无缩略图</div>
                )}
              </div>

              {!isEditing ? (
                <h2>{title}</h2>
              ) : (
                <label>
                  项目名称
                  <input
                    value={titleDraft}
                    onChange={(event) => setTitleDraft(event.target.value)}
                    disabled={pending}
                  />
                </label>
              )}

              <div className="project-meta">
                <p>
                  <span>时长</span>
                  <strong>{formatDuration(project.durationMs)}</strong>
                </p>
                <p>
                  <span>更新</span>
                  <strong>{formatDateTime(project.updatedAt)}</strong>
                </p>
              </div>

              <div className="project-actions">
                <button className="btn-primary" onClick={() => onOpenProject(project.projectId)}>
                  重新编辑/导出
                </button>
                {!isEditing ? (
                  <button
                    className="btn-outline"
                    onClick={() => {
                      setEditingProjectId(project.projectId);
                      setTitleDraft(title);
                    }}
                  >
                    重命名
                  </button>
                ) : (
                  <button
                    className="btn-outline"
                    onClick={() => void handleRename(project.projectId)}
                    disabled={pending}
                  >
                    {pending ? "保存中..." : "保存名称"}
                  </button>
                )}
                {project.exportPath && (
                  <button
                    className="btn-outline"
                    onClick={() => downloadExport(project.exportPath ?? "", title)}
                  >
                    下载导出
                  </button>
                )}
                {project.exportPath && (
                  <button
                    className="btn-outline"
                    onClick={() => void copyExportPath(project.exportPath ?? "")}
                  >
                    复制导出路径
                  </button>
                )}
                <button
                  className="danger"
                  onClick={() => void handleDelete(project.projectId)}
                  disabled={pending}
                >
                  删除
                </button>
              </div>
            </article>
          );
        })}
      </div>

      {!loading && projects.length === 0 && (
        <div className="empty-state">
          <p>暂无项目，点击右上角“新建录制”开始。</p>
        </div>
      )}
    </section>
  );
}
