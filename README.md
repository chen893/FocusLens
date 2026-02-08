# FocusLens (MVP v0.1)

FocusLens 是一个离线优先、开源透明的产品演示录屏工具。当前版本聚焦 MVP：

- 录制：全屏/窗口、麦克风、Windows 系统音频、macOS 系统音频降级
- 编辑：头尾裁剪、比例切换（16:9/9:16/1:1）、光标高亮
- 自动镜头运动：鼠标跟随、平滑缩放、强度可调
- 导出：本地 MP4（H.264 + AAC-LC），含进度、失败提示、自动回退、重试

## 快速开始

### 1) 环境要求

- Node.js 20+
- Rust stable
- Tauri 2 运行环境（Windows/macOS）

### 2) 安装依赖

```bash
npm install
```

### 3) 启动开发

```bash
npm run tauri:dev
```

### 4) 构建发布包

```bash
npm run tauri:build
```

## 项目结构

```text
src/                    # Web UI（录制/编辑/导出页面）
src-tauri/src/commands/ # Tauri 命令层（冻结 API）
src-tauri/src/core/     # 核心业务逻辑（capture/motion/export/recovery/timeline）
src-tauri/src/domain/   # 领域模型与状态机
src-tauri/src/infra/    # 存储、日志、ffmpeg 能力探测
docs/                   # PRD、Backlog、任务清单
```

## 冻结命令接口（MVP）

- `start_recording`
- `pause_recording`
- `resume_recording`
- `stop_recording`
- `load_project`
- `update_timeline`
- `update_camera_motion`
- `start_export`
- `retry_export`
- `recover_projects`

## 贡献入口

- 贡献规范：`CONTRIBUTING.md`
- Bug 模板：`.github/ISSUE_TEMPLATE/bug_report.yml`
- Feature 模板：`.github/ISSUE_TEMPLATE/feature_request.yml`
- PR 模板：`.github/PULL_REQUEST_TEMPLATE.md`

## 已知限制（MVP）

- 当前已完成 Week2~Week5 关键链路实现；不同机器上的真实采集能力仍需做设备兼容回归。
- 仅支持 Windows/macOS，Linux 不在 MVP 范围内。

## Week2-Week5 状态

详见 `docs/MVP-Implementation-Status.md` 与 `tests/Week2-Week5-Regression.md`。
