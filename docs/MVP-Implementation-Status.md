# MVP 实施状态（Week2-Week5）

更新时间：2026-02-07

## Week2：录制 P0 功能

1. 录制引擎接入 FFmpeg 进程：
   - `start_recording` / `pause_recording` / `resume_recording` / `stop_recording`
   - 录制状态事件 `recording/status`（状态、时长、输入源、降级信息）
2. 平台能力与降级：
   - `get_platform_capability` 返回平台矩阵
   - Windows 保持系统音频路径（WASAPI）
   - macOS 系统音频自动降级提示并继续录制
3. 设备与快捷键：
   - `list_audio_input_devices`
   - `load_hotkeys` / `save_hotkeys`（可持久化）
4. 项目初始化与恢复：
   - 录制产物写入 `assets/recording_raw.mp4`
   - `project.json` + `recovery.marker` 机制

## Week3：编辑与镜头运动

1. 编辑能力：
   - Trim、比例、光标高亮配置写回 `project.json`
2. 自动镜头运动：
   - 轨迹平滑算法（低/中/高）
   - 指标评估命令 `evaluate_camera_motion`
   - 质量指标：过渡时延 / 静止抖动比
3. 预览联动：
   - 编辑页实时反映参数变更

## Week4：导出链路

1. 导出接入真实 FFmpeg：
   - 输入录制资产 -> 输出 `renders/output.mp4`
2. 编码策略：
   - 硬件编码优先，失败后回退 `libx264`
3. 进度与错误：
   - `export/progress` 事件
   - 权限/空间/编码失败分类
   - 导出日志落盘 `renders/export-<task>.log`
4. 重试：
   - `retry_export` 无需重录

## Week5：质量冲刺

1. 质量指标计算：
   - ffprobe A/V 时长探测与偏移计算
   - 导出日志掉帧率解析（平均/峰值）
2. 质量门槛：
   - `validate_quality_gate`（A/V <=100ms、平均掉帧<=2%、峰值<=5%）
3. 兼容与恢复：
   - `schemaVersion` 迁移与高版本保护
   - 异常恢复扫描仅返回可恢复项目（manifest + marker + raw asset）

## 自动化验证

1. Rust 单测：17 项通过（状态机、平滑算法、质量门槛、迁移、错误分类、录制命令构建等）
2. 前端构建：`npm run build` 通过

## 说明

1. 当前版本已完成 Week2-Week5 的工程实现闭环与验收接口。
2. 由于本地硬件/权限环境差异，真实采集成功率需在目标机器执行 `tauri:dev` 进行端到端回归。

