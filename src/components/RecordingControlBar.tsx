import type { RecordingRuntimeStatus } from "../types/project";

type RecordingControlBarProps = {
  status: RecordingRuntimeStatus;
  busyAction: "start" | "pause" | "resume" | "stop" | null;
  onStart: () => Promise<void>;
  onPause: () => Promise<void>;
  onResume: () => Promise<void>;
  onStop: () => Promise<void>;
};

export function RecordingControlBar({
  status,
  busyAction,
  onStart,
  onPause,
  onResume,
  onStop
}: RecordingControlBarProps) {
  const controlsDisabled = busyAction !== null;

  return (
    <div className="panel control-bar">
      <strong>录制控制</strong>
      <div className="row gap">
        {(status === "idle" || status === "error" || status === "stopped") && (
          <button
            className="btn-primary"
            disabled={controlsDisabled}
            onClick={() => void onStart()}
          >
            {busyAction === "start" ? "启动中..." : "开始录制"}
          </button>
        )}
        {status === "recording" && (
          <button disabled={controlsDisabled} onClick={() => void onPause()}>
            {busyAction === "pause" ? "暂停中..." : "暂停"}
          </button>
        )}
        {status === "paused" && (
          <button disabled={controlsDisabled} onClick={() => void onResume()}>
            {busyAction === "resume" ? "继续中..." : "继续"}
          </button>
        )}
        {(status === "recording" || status === "paused") && (
          <button
            className="danger"
            disabled={controlsDisabled}
            onClick={() => void onStop()}
          >
            {busyAction === "stop" ? "停止中..." : "停止"}
          </button>
        )}
      </div>
    </div>
  );
}
