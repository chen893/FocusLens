import type { ExportStatus } from "../types/project";

type ExportProgressCardProps = {
  status: ExportStatus;
  progress: number;
  detail: string;
  onRetry: () => Promise<void>;
};

export function ExportProgressCard({
  status,
  progress,
  detail,
  onRetry
}: ExportProgressCardProps) {
  const statusText = {
    queued: "排队中",
    running: "导出中",
    fallback: "软编回退",
    success: "已完成",
    failed: "失败"
  } as const;

  const percent = Math.max(0, Math.min(100, Math.round(progress)));

  return (
    <div className="panel stack">
      <div className="panel-head">
        <strong>导出任务</strong>
        <span className={`status-chip status-${status}`}>{statusText[status]}</span>
      </div>
      <progress max={100} value={percent} />
      <div className="progress-meta">
        <span>{statusText[status]}</span>
        <span className="mono">{percent}%</span>
      </div>
      <p>{detail}</p>
      {status === "failed" && (
        <button onClick={() => void onRetry()} className="danger">
          重试导出
        </button>
      )}
    </div>
  );
}
