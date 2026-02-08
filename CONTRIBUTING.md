# Contributing to FocusLens

## 开发流程

1. 从 `main` 拉分支：`feature/<scope>` 或 `fix/<scope>`
2. 提交前本地执行：
   - `npm run build`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
3. 提交 PR，按模板补齐变更说明与测试证据

## 提交规范

- 推荐前缀：`feat`、`fix`、`refactor`、`test`、`docs`、`chore`
- 示例：`feat(export): add fallback status event`

## 代码规范

- 前端：TypeScript + React，保持 strict 类型安全
- Rust：优先显式错误码，避免 panic 作为业务控制流
- 新增字段必须同步更新：
  - `src/types/project.ts`
  - `src-tauri/src/domain/models.rs`
  - `project.json` 兼容策略与迁移逻辑

## Issue 与 PR 约定

- Bug 必须附复现步骤、期望行为、实际行为
- Feature 必须附业务场景与验收标准
- PR 必须附测试结果或无法测试的原因

