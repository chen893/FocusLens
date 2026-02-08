import type { RecordingRuntimeStatus } from "../types/project";
import { Button } from "./ui/Button";
import { Icons } from "./ui/Icons";

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
    <div className="panel control-bar glass panel-glass">
      <p className="eyebrow eyebrow-sm control-bar-title">录制控制</p>
      <div className="row gap">
        {(status === "idle" || status === "error" || status === "stopped") && (
          <Button
            variant="primary"
            disabled={controlsDisabled}
            onClick={() => void onStart()}
            loading={busyAction === "start"}
            icon={<Icons.Record />}
          >
            开始录制
          </Button>
        )}
        {status === "recording" && (
          <Button
            variant="outline"
            disabled={controlsDisabled}
            onClick={() => void onPause()}
            loading={busyAction === "pause"}
            icon={<Icons.Pause />}
          >
            暂停
          </Button>
        )}
        {status === "paused" && (
          <Button
            variant="primary"
            disabled={controlsDisabled}
            onClick={() => void onResume()}
            loading={busyAction === "resume"}
            icon={<Icons.Resume />}
          >
            继续
          </Button>
        )}
        {(status === "recording" || status === "paused") && (
          <Button
            variant="danger"
            disabled={controlsDisabled}
            onClick={() => void onStop()}
            loading={busyAction === "stop"}
            icon={<Icons.Stop />}
          >
            停止
          </Button>
        )}
      </div>
    </div>
  );
}
