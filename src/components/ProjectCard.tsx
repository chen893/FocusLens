import { useState, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ProjectListItem, ProjectStatus } from "../types/project";
import { Button } from "./ui/Button";
import { Icons } from "./ui/Icons";
import { StatusChip } from "./ui/StatusChip";
import { formatDuration, formatDateTime } from "../utils/format";

type ProjectCardProps = {
  project: ProjectListItem;
  onOpen: (projectId: string) => void;
  onDelete: (projectId: string) => Promise<void>;
  onRename: (projectId: string, title: string) => Promise<void>;
};

const statusMap: Record<ProjectStatus, string> = {
  recording: "recording",
  ready_to_edit: "queued",
  exporting: "running",
  export_failed: "failed",
  export_succeeded: "success"
};

const statusText: Record<ProjectStatus, string> = {
  recording: "录制中",
  ready_to_edit: "待编辑",
  exporting: "正在导出",
  export_failed: "导出失败",
  export_succeeded: "已完成"
};

export function ProjectCard({
  project,
  onOpen,
  onDelete,
  onRename
}: ProjectCardProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [renameSuccess, setRenameSuccess] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);

  const title = project.title?.trim() || `Project_${project.projectId.slice(0, 8)}`;
  const isFeedbackPositive = Boolean(error && (error.includes("复制") || error.includes("成功")));

  const handleRename = async () => {
    if (!titleDraft.trim()) {
      setError("标题不能为空");
      return;
    }
    setPending(true);
    setError(null);
    try {
      await onRename(project.projectId, titleDraft.trim());
      setIsEditing(false);
      setRenameSuccess(true);
      setTimeout(() => setRenameSuccess(false), 2000);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  };

  const handleDelete = async () => {
    if (!window.confirm("确认删除该项目及其录制文件吗？此操作不可恢复。")) return;
    setPending(true);
    try {
      await onDelete(project.projectId);
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  };

  const copyExportPath = async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
      setError("导出路径已复制到剪贴板");
      setTimeout(() => setError(null), 3000);
    } catch {
      setError("复制失败");
    }
  };

  const downloadExport = (path: string) => {
    try {
      const href = convertFileSrc(path);
      const anchor = document.createElement("a");
      anchor.href = href;
      anchor.download = `${title}.mp4`;
      anchor.click();
    } catch {
      setError("无法直接下载");
    }
  };

  let previewSrc: string | null = null;
  if (project.rawPath) {
    try {
      previewSrc = convertFileSrc(project.rawPath);
    } catch {
      previewSrc = null;
    }
  }

  return (
    <article
      className={`project-card ${renameSuccess ? 'rename-success' : ''}`}
      onMouseEnter={() => videoRef.current?.play().catch(() => {})}
      onMouseLeave={() => {
        if (videoRef.current) {
          videoRef.current.pause();
          videoRef.current.currentTime = 0;
        }
      }}
    >
      <div className="project-card-head">
        <StatusChip
          status={statusMap[project.status]}
          label={statusText[project.status]}
        />
        <small className="mono project-id-short">{project.projectId.slice(0, 8)}</small>
      </div>

      <div className="project-thumb">
        {previewSrc ? (
          <video ref={videoRef} src={previewSrc} muted preload="metadata" loop />
        ) : (
          <div className="project-thumb-fallback">
            <Icons.Folder size={32} className="project-thumb-icon" />
            <span>预览不可用</span>
          </div>
        )}
      </div>

      {!isEditing ? (
        <div className="project-title-row">
          <h2 className="project-title">
            {title}
          </h2>
          <Button
            variant="ghost"
            icon={<Icons.Edit size={14} />}
            onClick={() => {
              setIsEditing(true);
              setTitleDraft(title);
            }}
          />
        </div>
      ) : (
        <div className="project-rename-row">
          <label className="project-rename-label">
            <span className="project-rename-caption">重命名项目</span>
            <input
              value={titleDraft}
              onChange={(e) => setTitleDraft(e.target.value)}
              disabled={pending}
              autoFocus
              onKeyDown={(e) => e.key === 'Enter' && handleRename()}
            />
          </label>
          <Button variant="primary" onClick={() => void handleRename()} loading={pending}>
            保存
          </Button>
          <Button variant="ghost" onClick={() => setIsEditing(false)}>取消</Button>
        </div>
      )}

      {error && <p className={`note project-feedback ${isFeedbackPositive ? "project-feedback-success" : "project-feedback-error"}`}>{error}</p>}

      <div className="project-meta">
        <p>
          <span>时长</span>
          <strong>{formatDuration(project.durationMs)}</strong>
        </p>
        <p>
          <span>更新于</span>
          <strong>{formatDateTime(project.updatedAt)}</strong>
        </p>
      </div>

      <div className="project-actions project-actions-wrap">
        <Button
          variant="primary"
          className="btn-fill"
          onClick={() => onOpen(project.projectId)}
        >
          进入工作室
        </Button>
        <div className="project-action-row">
          {project.exportPath && (
            <>
              <Button
                variant="outline"
                icon={<Icons.Export size={16} />}
                onClick={() => downloadExport(project.exportPath!)}
                title="下载视频"
              />
              <Button
                variant="outline"
                icon={<Icons.Copy size={16} />}
                onClick={() => void copyExportPath(project.exportPath!)}
                title="复制路径"
              />
            </>
          )}
          <Button
            variant="danger"
            icon={<Icons.Delete size={16} />}
            onClick={() => void handleDelete()}
            disabled={pending}
            className="push-end"
            title="删除项目"
          />
        </div>
      </div>
    </article>
  );
}
