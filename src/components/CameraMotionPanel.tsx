import type { CameraIntensity, CameraMotionProfile } from "../types/project";

type CameraMotionPanelProps = {
  profile: CameraMotionProfile;
  onChange: (patch: Partial<CameraMotionProfile>) => Promise<void>;
};

const intensities: CameraIntensity[] = ["low", "medium", "high"];

export function CameraMotionPanel({ profile, onChange }: CameraMotionPanelProps) {
  const zoomValue = Math.min(Math.max(profile.maxZoom, 1), 1.5);
  const applyPatch = (patch: Partial<CameraMotionProfile>) => {
    void onChange(patch).catch(() => undefined);
  };

  return (
    <div className="panel stack">
      <strong>自动镜头运动（Hybrid）</strong>
      <p className="note">建议先使用 medium，再按场景微调平滑和缩放上限。</p>
      <div className="stack form-grid-two">
        <label className="toggle-field toggle-field--full">
          <span className="toggle-field-main">
            <input
              type="checkbox"
              checked={profile.enabled}
              onChange={(event) => applyPatch({ enabled: event.target.checked })}
            />
            <span className="toggle-field-title">启用自动镜头运动</span>
          </span>
          <span className="toggle-field-hint">
            根据操作焦点自动平滑推拉镜头，减少手动裁切成本。
          </span>
        </label>
        <label>
          强度
          <select
            value={profile.intensity}
            onChange={(event) => applyPatch({ intensity: event.target.value as CameraIntensity })}
          >
            {intensities.map((intensity) => (
              <option key={intensity} value={intensity}>
                {intensity === "low" ? "low（全局优先）" : intensity === "medium" ? "medium（平衡）" : "high（聚焦优先）"}
              </option>
            ))}
          </select>
        </label>
        <label>
          平滑系数 ({profile.smoothing.toFixed(2)})
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={profile.smoothing}
            onChange={(event) => applyPatch({ smoothing: Number(event.target.value) })}
          />
        </label>
        <label>
          最大缩放上限 ({zoomValue.toFixed(2)}x)
          <input
            type="range"
            min={1}
            max={1.5}
            step={0.05}
            value={zoomValue}
            onChange={(event) => applyPatch({ maxZoom: Number(event.target.value) })}
          />
        </label>
        <label>
          停止后回中延迟 ({profile.idleThresholdMs}ms)
          <input
            type="range"
            min={200}
            max={1200}
            step={50}
            value={profile.idleThresholdMs}
            onChange={(event) => applyPatch({ idleThresholdMs: Number(event.target.value) })}
          />
        </label>
      </div>
    </div>
  );
}
