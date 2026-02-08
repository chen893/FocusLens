import { RecordingProfile, RecordingRuntimeStatus, HotkeySettings } from "../../types/project";
import { Button } from "../../components/ui/Button";
import { Icons } from "../../components/ui/Icons";
import { StatusChip } from "../../components/ui/StatusChip";
import { formatDuration } from "../../utils/format";

type RecordingViewProps = {
  status: RecordingRuntimeStatus;
  durationMs: number;
  detail: string;
  degradeMessage: string | null;
  recordingProfile: RecordingProfile;
  setRecordingProfile: React.Dispatch<React.SetStateAction<RecordingProfile>>;
  hotkeys: HotkeySettings;
  setHotkeys: React.Dispatch<React.SetStateAction<HotkeySettings>>;
  audioDevices: Array<{ id: string; label: string }>;
  capability: { supportsSystemAudio: boolean } | null;
  busyAction: string | null;
  onRunAction: (action: any, task: () => Promise<void>) => Promise<void>;
  onStartRecording: (profile: RecordingProfile) => Promise<void>;
  onPauseRecording: () => Promise<void>;
  onResumeRecording: () => Promise<void>;
  onStopRecording: () => Promise<void>;
  onSaveHotkeys: (hotkeys: HotkeySettings) => Promise<void>;
  onBackToDashboard: () => void;
};

const recordingStatusMap: Record<RecordingRuntimeStatus, string> = {
  idle: "idle",
  recording: "recording",
  paused: "pending",
  stopped: "success",
  error: "failed"
};

const recordingStatusText: Record<RecordingRuntimeStatus, string> = {
  idle: "准备就绪",
  recording: "正在录制",
  paused: "已暂停",
  stopped: "录制结束",
  error: "录制异常"
};

export function RecordingView({
  status,
  durationMs,
  detail,
  degradeMessage,
  recordingProfile,
  setRecordingProfile,
  hotkeys,
  setHotkeys,
  audioDevices,
  capability,
  busyAction,
  onRunAction,
  onStartRecording,
  onPauseRecording,
  onResumeRecording,
  onStopRecording,
  onSaveHotkeys,
  onBackToDashboard
}: RecordingViewProps) {
  return (
    <div className="studio-shell">
      <header className="studio-topbar">
        <Button variant="outline" icon={<Icons.Back />} onClick={onBackToDashboard}>
          返回列表
        </Button>
        <p className="studio-mode-tag">录制模式</p>
        <StatusChip
          status={recordingStatusMap[status]}
          label={recordingStatusText[status]}
        />
      </header>

      <div className="recording-layout">
        <div className="preview-canvas">
          <div className="preview-overlay preview-overlay-card glass">
            <div className="preview-overlay-head">
              <p className="preview-title preview-title-compact">实时预览</p>
              <div className="preview-runtime">
                <span className="mono preview-time">{formatDuration(durationMs)}</span>
                {status === 'recording' && <div className="pulse-red preview-recording-dot" />}
              </div>
            </div>
            <p className="muted preview-detail">{detail}</p>
            {degradeMessage && <p className="warn preview-warn">{degradeMessage}</p>}
          </div>
        </div>

        <aside className="recording-panel">
          <h2 className="recording-panel-title">录制控制台</h2>

          <div className="panel stack capture-panel">
            <p className="eyebrow eyebrow-sm">采集配置</p>
            <div className="form-grid-two">
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
            </div>
            <div className="form-grid-two">
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
                  <option value="__none__">无麦克风</option>
                  {audioDevices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <label
              className={`toggle-field toggle-field-spaced ${!capability?.supportsSystemAudio ? "toggle-field-disabled" : ""}`}
            >
              <span className="toggle-field-main">
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
                <span className="toggle-field-title">同时录制系统音频</span>
              </span>
              <span className="toggle-field-hint">
                {capability?.supportsSystemAudio
                  ? "录制应用播放声音，适合课程讲解和产品演示。"
                  : "当前系统暂不支持系统音频采集。"}
              </span>
            </label>
          </div>

          <div className="recording-hotkeys">
            <p className="eyebrow eyebrow-sm">快捷键</p>
            <div className="form-grid-two">
              <label>
                开始/停止
                <input
                  className="mono hotkey-input"
                  value={hotkeys.startStop}
                  onChange={(event) =>
                    setHotkeys((prev) => ({ ...prev, startStop: event.target.value }))
                  }
                />
              </label>
              <label>
                暂停/继续
                <input
                  className="mono hotkey-input"
                  value={hotkeys.pauseResume}
                  onChange={(event) =>
                    setHotkeys((prev) => ({ ...prev, pauseResume: event.target.value }))
                  }
                />
              </label>
            </div>
            <Button
              variant="outline"
              icon={<Icons.Settings size={14} />}
              onClick={() =>
                void onRunAction("saveHotkeys", async () => {
                  await onSaveHotkeys(hotkeys);
                })
              }
              loading={busyAction === "saveHotkeys"}
            >
              保存快捷键
            </Button>
          </div>

          <div className="recording-main-actions">
            {(status === "idle" ||
              status === "error" ||
              status === "stopped") && (
              <Button
                variant="primary"
                className="record-button"
                onClick={() =>
                  void onRunAction("start", () =>
                    onStartRecording({
                      ...recordingProfile,
                      hotkeys
                    })
                  )
                }
                disabled={busyAction !== null}
                icon={<Icons.Record size={24} />}
              >
                {busyAction === "start" ? "正在启动..." : "立即开始录制"}
              </Button>
            )}
            {status === "recording" && (
              <div className="action-split">
                <Button
                  variant="outline"
                  className="btn-fill"
                  onClick={() => void onRunAction("pause", onPauseRecording)}
                  disabled={busyAction !== null}
                  icon={<Icons.Pause />}
                >
                  暂停
                </Button>
                <Button
                  variant="danger"
                  className="btn-fill"
                  onClick={() => void onRunAction("stop", onStopRecording)}
                  disabled={busyAction !== null}
                  icon={<Icons.Stop />}
                >
                  停止录制
                </Button>
              </div>
            )}
            {status === "paused" && (
              <div className="action-split">
                <Button
                  variant="primary"
                  className="btn-fill"
                  onClick={() => void onRunAction("resume", onResumeRecording)}
                  disabled={busyAction !== null}
                  icon={<Icons.Resume />}
                >
                  继续
                </Button>
                <Button
                  variant="danger"
                  className="btn-fill"
                  onClick={() => void onRunAction("stop", onStopRecording)}
                  disabled={busyAction !== null}
                  icon={<Icons.Stop />}
                >
                  停止录制
                </Button>
              </div>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
