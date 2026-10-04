# R8-T1 / R8-T2 局部修复独立复验

日期：2026-10-04。结论：**APPROVE，仅限这两个局部 Table 修复**；没有发现该范围内需阻断的 P0/P1/P2 问题。此结论不是整个 0.1.8 RC 的发布批准。

## 源码与审查边界

- 对照基线：`736025a8`。
- 首次独立执行：`3fd6157272b3b133b40c76bf76e03653bb81c844`。
- 最终源码/测试冻结及追加独立执行：`43732b89c48ec6add95cc341d9e1090fe862cec4`。
- `3fd → 437` 仅修改 `lifecycle.rs` 的 `#[cfg(test)] mod tests`：抽出断言 helper、采用安全数字转换、调整等价字符串比较；检查到的断言与场景未减少。产品修复完全相同：两 head 的 `node.rs` Git blob 均为 `46c4f1267b4c7044940936de6b4bf27650b0dec0`，`table_layout.rs` 均为 `c0668cd8c77434f05602580798bbcb78cf94ee65`。
- 本轮只读产品、测试及文档；仅新增本报告与自己的测试日志。未修改原审计、旧 probe、断言、依赖或产品，未操作 UI、Rift、系统外观或其他进程，未 commit、merge、publish 或 tag。

按已批准计划复验 R8-T1（方向切换时保留逻辑滚动位置）和 R8-T2（caller Table 元数据在增量更新中重放）。未扩展成 Runtime 重构、来源系统或键盘路由审计。使用 `code-review-expert`、`gpui-layout-and-style`、`rhai-api` 的只读审查流程，并核验本项目 pinned GPUI pre-0.3.7 / Rhai 1.26.0 官方 crate 源码；未套用其他 GPUI 库的约定。

## 源码结论

### R8-T1：PASS

`crates/gpui-rhai/src/table_layout.rs:419` 先按旧方向解码逻辑距离，再按当前实际帧的范围夹取，最后编码为新方向的物理偏移：LTR 为 `-old_x`，RTL 为 `old_maximum + old_x`；结果夹入 `[0, new_maximum]`，再写为 LTR `-distance` / RTL `distance - new_maximum`。只有首次测量、旧方向为 `None` 时初始化到逻辑起点；locale 改变不再被当作首次初始化。纵向偏移保持不变。

这与 pinned GPUI `src/elements/div.rs:4394` 的物理 offset 定义及 `:2475` 的 `[-scroll_max, 0]` 夹取一致。Table 的既有 width plan、direct extent、border 修正、正常 prepaint 顺序及有界跟进帧需求均未变。测试使用呈现后的表头/表格几何判断距离，不是仅检查字段或借截图推断横滚功能。

### R8-T2：PASS

`crates/gpui-rhai/src/node.rs:409` 新增 `TableTrack` / `TableColumn` presentation mutation；`:1948` / `:1970` 的 builder 继续保留原 Box、列定义和索引验证，改为进入既有 `apply_presentation_mutation`。

核对以下既有边界后，这一接入没有改变组件执行模型：

- `apply_presentation_mutation`（`:996`）在 caller 修改前激活组件自有快照，仅正式 component root 记录外侧 mutation。
- `with_component_root_snapshot`（`:1102`）在组件内部 render 完成后绑定 root 并清空 presentation；因此内部 track/column 成为新 raw root 的一部分，不会被旧内部值冻结。
- `replace_component_subtree` / `hydrate_component_subtrees`（`:1118` / `:1184`）对新组件自有节点重放外侧 metadata；两个新 mutation 是字段赋值，没有新增脚本调用、callback context 或可变布局共享入口。
- `lifecycle.rs:298` 起的 candidate root / retained clone 只在成功时提交；失败仍恢复 runtime 和 engine checkpoint。`engine.rs:1395` 起的 checkpoint 包含 component-owned snapshots，恢复路径未被本次修复改动。

定向测试分别验证内部值跟随新组件 render、外侧值保持 caller 权威、已有 width=123 style 控制不丢、连续 child-only dirty 更新、失败后 last-good 保留以及随后重试成功。没有要求通用来源系统、跨卸载 Ref 兼容或改变已通过的 scoped batch / ReadDependency 模型。

## 独立执行证据

四次执行均使用已有 target，没有新建 target/profile。最终 `437` 的两个进程均 exit 0；core 等待共享 Cargo artifact 锁属于正常串行，不影响测试结果。

| 精确源码 | 命令范围 | 独立结果 | 日志 |
| --- | --- | --- | --- |
| `3fd61572` | core `--lib table_modifiers` | 2/2 PASS，473 filtered | [core 原始日志](review-evidence/table-modifiers-3fd61572.log) |
| `3fd61572` | native `table_extent` 指定方向连续性测试 | 1/1 PASS，10 filtered | [native 原始日志](review-evidence/direction-3fd61572.log) |
| `43732b89` | core `--lib table_modifiers` | 2/2 PASS，473 filtered，0.83 s | [core 最终日志](review-evidence/table-modifiers-43732b89.log) |
| `43732b89` | native `table_extent` 指定方向连续性测试 | 1/1 PASS，10 filtered，2.11 s | [native 最终日志](review-evidence/direction-43732b89.log) |

最终复跑命令：

```sh
cargo test -p gpui-rhai --lib table_modifiers --locked -- --nocapture
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --test table_extent direction_changes_preserve_logical_scroll_distance_and_clamp -- --exact --nocapture --test-threads=1
```

core 两项各循环 internal/external 元数据；native 一项循环 Array/Native × LTR/RTL 两个起始方向，实际测得滚动距离 80 在方向切换后保留，再扩大 viewport 将范围降为 72，夹取后切回仍为 72，并保留表头/首行宽度对齐控制。该 native 结果来自 GPUI test-support 帧与几何，不冒充完整 macOS OS 人工验收。

日志 SHA-256：

```text
table-modifiers-3fd61572.log  34bb645088ac1ee28fefbb4a500a264e74f8594b45fbf0084cc663675e37f07c
direction-3fd61572.log        0b5da1a6c2ec451f13bd6293620e6aa1c68777617e65fde487601c852404e0b4
table-modifiers-43732b89.log  949362059789ff2a92f10bf64608868d26b45cf01c3a9fcc904091bed227d10c
direction-43732b89.log        f422bf68c80eee520c79bbae44579e1457dee4c1973af139a32df04e443d9184
```

## 未覆盖与交付条件

本 reviewer 本轮未独立重跑完整 workspace/native、strict Clippy、既有 Table boundary/extent/frame 全矩阵或全部主题/同列混合控制；主实施者的结果与本报告独立执行明确分开。修前失败由主实施者另存证据，本轮 reviewer 未独立执行修前版本。

键盘建议只改文档，路由未改；本轮未重新证明更广键盘行为。同源正式 baseline 重拍与归档仍由主流程执行，本报告不批准截图替换，也不把既有截图当方向连续性的证明。完整 OS 人工矩阵及物理 120 Hz 仍 pending，整个 RC 不因这次局部 APPROVE 而变为发布 ready。历史失败/后续修复报告继续保留，不重写为绿。
