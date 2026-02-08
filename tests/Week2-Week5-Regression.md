# Week2-Week5 回归步骤

## 基础编译

1. `npm run build`
2. `cargo test --manifest-path src-tauri/Cargo.toml`

## 录制链路（Week2）

1. 进入录制页，选择麦克风、系统音频和分辨率。
2. 点击开始录制，确认状态事件持续刷新时长。
3. 暂停/继续/停止后，确认生成：
   - `projects/<project-id>/project.json`
   - `projects/<project-id>/assets/recording_raw.mp4`
   - `projects/<project-id>/assets/cursor_track.json`

## 编辑与镜头（Week3）

1. 在编辑页加载项目。
2. 修改 trim/aspect/cursor highlight/camera motion。
3. 点击“评估镜头质量”，检查时延与抖动输出。

## 导出链路（Week4）

1. 在导出页输入 `projectId`，点击开始导出。
2. 观察导出进度事件：
   - queued -> running -> (fallback?) -> success
3. 失败时点击重试，确认可再次进入导出流程。
4. 检查产物：
   - `projects/<project-id>/renders/output.mp4`
   - `projects/<project-id>/renders/export-<task-id>.log`

## 质量门槛（Week5）

1. 在导出页点击“检查质量门槛”。
2. 验证 `validate_quality_gate` 返回：
   - `passed=true` 或
   - `passed=false` + 原因列表（A/V、平均掉帧、峰值掉帧）

