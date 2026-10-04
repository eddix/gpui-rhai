# gpui-rhai 0.2.0 设计系统重做实施计划

日期：2026-10-04。分支：`feat/design-system-0.2`（自 `v0.1.8` / `d87ebb9`）。状态：实施中。

本计划来自 2026-10-04 与维护者的设计 grilling 会话。共识的正式内容写在
[`docs/design/`](../design/principles.md)，本文件只跟踪执行顺序、完成证据和遗留事项。
细节决策记入 [`docs/design/decisions.md`](../design/decisions.md)。

## 授权边界

- 一口气完成下列五步，步骤之间不停下评审。
- 按主题攒批提交到本分支；不推送、不合并到 `main`、不发版、不打 tag。
- 只在三种情况停下问维护者：某条共识被证明做不到、必须改方向；破坏性或对外操作；需要维护者本人操作（权限等）。
- pre-1.0 有意破坏性升级：版本 0.2.0，`runtime_api` 2→3，不保留历史兼容别名或双轨实现。
- 不修改 `oh-my-byted`、`disktree-rhai` 等外部仓库；disktree-rhai 只用 `cargo --config` 临时指向本分支做冒烟。

## 五步

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | `docs/design/` 四份规范 + 决策日志 + Gallery 线框；延迟构造 ADR | 进行中 |
| 2 | 运行时地基：开放 token 与字阶、token 基础层、组件声明 token、声明式环境值（原生解析）与原生继承 disabled、颜色派生与读取、Rust 派生迁到 L0、action 快捷键查询、组合审计、CLI profile | 未开始 |
| 3 | L0 `tokens.rhai` + 内置主题只写颜色 + 新默认主题；全部 62 个组件的组合契约与新视觉 | 未开始 |
| 4 | L2：`layouts/`（Stack/Inline/Toolbar/Region）与 `patterns/`（Section/DescriptionList/Stat/FormLayout/InlineState/DataView/ListDetail/AppShell） | 未开始 |
| 5 | 用 AppShell + L2 重做 Gallery（规格页 + 场景页）；自带设计参照示例进 CI；审计零告警；纯键盘场景测试；重拍基线；文档、CHANGELOG、迁移说明 | 未开始 |

## 每批验收命令

与 CI 一致：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --manifest-path tests/performance/Cargo.toml --locked
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked
bash scripts/audit-visual-baselines.sh
```

视觉改动另外截图自查；Linux 只能在推送后由 CI 验证（本机为 macOS）。

## 进度记录

（按批次追加：日期、提交、内容、证据。）
