import type { AspectRatio, TimelineConfig } from "../types/project";

type TimelineEditorProps = {
  timeline: TimelineConfig;
  onChange: (patch: Partial<TimelineConfig>) => Promise<void>;
};

const aspectOptions: AspectRatio[] = ["16:9", "9:16", "1:1"];

export function TimelineEditor({ timeline, onChange }: TimelineEditorProps) {
  const applyPatch = (patch: Partial<TimelineConfig>) => {
    void onChange(patch).catch(() => undefined);
  };

  return (
    <div className="panel stack">
      <strong>时间线与画布</strong>
      <p className="note">Trim 会直接影响导出片段时长，建议先录制再精调。</p>
      <div className="stack form-grid-two">
        <label>
          起始裁剪（ms）
          <input
            type="number"
            value={timeline.trimStartMs}
            min={0}
            onChange={(event) => {
              const next = Number(event.target.value);
              if (Number.isNaN(next) || next < 0) {
                return;
              }
              applyPatch({ trimStartMs: next });
            }}
          />
        </label>
        <label>
          结束裁剪（ms）
          <input
            type="number"
            value={timeline.trimEndMs}
            min={0}
            onChange={(event) => {
              const next = Number(event.target.value);
              if (Number.isNaN(next) || next < 0) {
                return;
              }
              applyPatch({ trimEndMs: next });
            }}
          />
        </label>
        <label>
          画布比例
          <select
            value={timeline.aspectRatio}
            onChange={(event) => applyPatch({ aspectRatio: event.target.value as AspectRatio })}
          >
            {aspectOptions.map((ratio) => (
              <option key={ratio} value={ratio}>
                {ratio}
              </option>
            ))}
          </select>
        </label>
        <label className="row toggle-row">
          <input
            type="checkbox"
            checked={timeline.cursorHighlightEnabled}
            onChange={(event) => applyPatch({ cursorHighlightEnabled: event.target.checked })}
          />
          光标高亮
        </label>
      </div>
    </div>
  );
}
