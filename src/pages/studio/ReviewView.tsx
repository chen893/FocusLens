import { ExportProfile, ExportStatus } from "../../types/project";
import { Button } from "../../components/ui/Button";
import { Icons } from "../../components/ui/Icons";
import { StatusChip } from "../../components/ui/StatusChip";
import { TimelineEditor } from "../../components/TimelineEditor";
import { CameraMotionPanel } from "../../components/CameraMotionPanel";
import { ExportProgressCard } from "../../components/ExportProgressCard";

type ReviewViewProps = {
  projectId: string;
  projectTitle: string;
  setProjectTitle: (title: string) => void;
  titleDirty: boolean;
  setTitleDirty: (dirty: boolean) => void;
  titleHint: string | null;
  onSaveTitle: () => Promise<void>;
  previewSrc: string | null;
  manifest: any;
  updateTimeline: any;
  updateCameraMotion: any;
  exportProfile: ExportProfile;
  setExportProfilePatch: (patch: Partial<ExportProfile>) => void;
  exportStatus: ExportStatus;
  exportStatusText: Record<ExportStatus, string>;
  exportStatusMap: Record<ExportStatus, string>;
  taskId: string | null;
  progress: number;
  exportDetail: string;
  onRetryExport: () => Promise<void>;
  onStartExport: () => Promise<void>;
  onCheckQualityGate: () => Promise<void>;
  checkingGate: boolean;
  qualityGate: { passed: boolean; reasons: string[] } | null;
  actionError: string | null;
  submittingExport: boolean;
  onBackToDashboard: () => void;
  onStartReRecord: () => void;
};

export function ReviewView({
  projectId,
  projectTitle,
  setProjectTitle,
  titleDirty,
  setTitleDirty,
  titleHint,
  onSaveTitle,
  previewSrc,
  manifest,
  updateTimeline,
  updateCameraMotion,
  exportProfile,
  setExportProfilePatch,
  exportStatus,
  exportStatusText,
  exportStatusMap,
  taskId,
  progress,
  exportDetail,
  onRetryExport,
  onStartExport,
  onCheckQualityGate,
  checkingGate,
  qualityGate,
  actionError,
  submittingExport,
  onBackToDashboard,
  onStartReRecord
}: ReviewViewProps) {
  const exportTaskActive = Boolean(
    taskId && (exportStatus === "queued" || exportStatus === "running" || exportStatus === "fallback")
  );

  const resolveOutputLabel = (resolution: ExportProfile["resolution"], fps: ExportProfile["fps"]) => {
    if (resolution === "1080p") {
      return fps === 60 ? "1080p / 60fps / 12Mbps" : "1080p / 30fps / 8Mbps";
    }
    return fps === 60 ? "720p / 60fps / 8Mbps" : "720p / 30fps / 4Mbps";
  };

  return (
    <div className="studio-shell">
      <header className="studio-topbar">
        <Button variant="outline" icon={<Icons.Back />} onClick={onBackToDashboard}>
          返回列表
        </Button>
        <p className="studio-mode-tag">编辑与导出</p>
        <div className="row gap">
          <Button variant="outline" icon={<Icons.Refresh />} onClick={onStartReRecord}>
            重新录制
          </Button>
          <Button
            variant="primary"
            icon={<Icons.Export />}
            onClick={onStartExport}
            loading={submittingExport}
            disabled={exportTaskActive}
          >
            {exportTaskActive ? "导出进行中" : "完成并导出"}
          </Button>
        </div>
      </header>

      <div className="studio-title-row studio-title-row-glass glass">
        <div className="studio-title-main">
          <label className="studio-title-label">
            <span className="eyebrow eyebrow-sm">项目名称</span>
            <input
              className="studio-title-input"
              placeholder="未命名项目"
              value={projectTitle}
              onChange={(event) => {
                setTitleDirty(true);
                setProjectTitle(event.target.value);
              }}
              onBlur={() => titleDirty && onSaveTitle()}
            />
          </label>
        </div>
        <div className="studio-title-side">
          <p className="mono studio-title-id">ID: {projectId}</p>
          {titleHint && <p className="note studio-title-hint">{titleHint}</p>}
        </div>
      </div>

      <div className="review-layout">
        <div className="review-left">
          <div className="video-shell video-shell-glass glass">
            {previewSrc ? (
              <video controls src={previewSrc} className="review-video" />
            ) : (
              <div className="video-fallback">
                <Icons.Folder size={64} className="review-fallback-icon" />
                <p>暂无录制文件可供预览</p>
              </div>
            )}
          </div>
          {manifest && (
            <div className="stack review-editor-stack">
              <TimelineEditor timeline={manifest.timeline} onChange={updateTimeline} />
              <CameraMotionPanel profile={manifest.cameraMotion} onChange={updateCameraMotion} />
            </div>
          )}
        </div>

        <aside className="review-right">
          <div className="review-sidebar">
            <div className="panel stack glass review-config-panel">
              <div className="panel-head">
                <strong className="review-panel-title">导出配置</strong>
                <StatusChip
                  status={taskId ? exportStatusMap[exportStatus] : "idle"}
                  label={taskId ? exportStatusText[exportStatus] : "待导出"}
                />
              </div>

              <div className="stack">
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
                    <option value="1080p / 60fps / 12Mbps">极清 (1080p / 60fps)</option>
                    <option value="1080p / 30fps / 8Mbps">高清 (1080p / 30fps)</option>
                    <option value="720p / 60fps / 8Mbps">流畅 (720p / 60fps)</option>
                    <option value="720p / 30fps / 4Mbps">体积优先 (720p / 30fps)</option>
                  </select>
                </label>

                <div className="row gap">
                  <Button
                    variant="primary"
                    className="btn-fill"
                    onClick={onStartExport}
                    disabled={submittingExport || exportTaskActive}
                    icon={<Icons.Export />}
                  >
                    {exportTaskActive ? "导出中" : "开始渲染"}
                  </Button>
                  <Button
                    variant="outline"
                    title="检查视频质量门槛"
                    onClick={onCheckQualityGate}
                    loading={checkingGate}
                    icon={<Icons.Refresh size={16} />}
                  />
                </div>
              </div>
              {actionError && <p className="error text-xs">{actionError}</p>}
            </div>

            <ExportProgressCard
              status={exportStatus}
              progress={progress}
              detail={exportDetail}
              onRetry={onRetryExport}
            />

            {qualityGate && (
              <div className="panel stack glass review-quality-panel">
                <div className="row gap row-between">
                  <strong className="review-subtitle">质量门槛检查</strong>
                  <StatusChip
                    status={qualityGate.passed ? 'success' : 'failed'}
                    label={qualityGate.passed ? '达标' : '不达标'}
                  />
                </div>
                {!qualityGate.passed && (
                  <div className="stack quality-reasons">
                    {qualityGate.reasons.map((reason) => (
                      <p key={reason} className="warn quality-reason">
                        • {reason}
                      </p>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
