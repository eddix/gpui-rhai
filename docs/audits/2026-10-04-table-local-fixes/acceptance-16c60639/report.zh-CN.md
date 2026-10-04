# 16c60639：R8 两处 Table 修复追加验收

日期：2026-10-04。精确 HEAD：`16c60639dc1be8b7ad7d41255c578379afbce6c8`，候选 [PR #108](https://github.com/eddix/gpui-rhai/pull/108)。对照上次主审基线 `736025a8`。

**结论：R8-T1、R8-T2 均关闭；本次限定范围 APPROVE，没有新增代码 finding。** 修复沿用既有模型，没有扩大 Runtime 重构。整体发布仍等待新 HEAD CI、实际 OS 输入/辅助功能与物理 120Hz 验收；当前结论不代表获准合入或发布。

## R8-T1：方向切换的逻辑滚动位置——通过

`table_layout.rs` 按旧方向解码 logical distance，按新范围夹取，再按新方向编码物理 offset。仅首次测量的 `None` 初始化为起点；横向转换保留纵向 offset。实际 extent、共用列计划、border 修正及既有跟进帧机制未被改动。

本轮直接重跑上次原始探针，没有修改源码、输入或断言：

- LTR→RTL：**80→80**；RTL→LTR：**80→80**。
- 原 R7 四项、混合列 resize 和方向切换共 **6/6** 通过。
- 新正式 `direction_changes_preserve_logical_scroll_distance_and_clamp` 覆盖 Array/Native × 两向；扩大 viewport 后范围降为 72，位置夹到 72，再切回仍为 72，表头/首行保持一致。
- 一并运行完整 `table_extent` **11/11** 与 `table_frame_demand` **2/2**，实际正常调度后帧需求归零，未引入依赖额外 refresh 才更新的问题。

证据：[原始 Table 探针日志](original-table.log)、[正式 Table/native 日志](table-native.log)、[精确命令](native-commands.json)。原生测试均取消 `RUST_MIN_STACK` 覆盖，使用 locked/offline、单测试线程。

## R8-T2：调用方 Table 标记重放——通过

`node.rs` 为既有 `NodePresentationMutation` 添加 `TableTrack` / `TableColumn`，两个 builder 保留原验证并进入 `apply_presentation_mutation`。没有新增 metadata registry 或第二条恢复路径。

该接入保留现有所有权边界：组件内部 render 产生自有快照；调用方装饰单独记录；成功的子组件替换重放装饰；失败保留 last-good。内部设置的布局值继续随新 render 更新，不被外部保留机制冻结。

本轮重新执行：

- 上轮原始四组 characterization：track/column × 内部/外部均为 **marker true→true**，外侧 width=123 仍保留，文本 count 确实更新。
- 两项正式 core 回归 **2/2**：内部/外部值、连续 child-only 更新、普通 style 正控、失败保留和随后重试均通过。

证据：[原始四组输出](original-modifiers.log)、[正式 core 日志](core-replay.log)、[精确命令](core-commands.json)。characterization 不只检查 exit 0；运行器另外确认恰好四组且每组 `marker_before=true, marker_after=true`。

## 来源、视觉与文档

对比 `43732b89` 和当前 HEAD，core/CLI 源码、registry、Cargo.lock 与 native tests 对应 Git tree/blob 一致。上次原始 `table/probe.rs`、`table/round7.rs`、`virtual-runtime/probe/src/main.rs` 的 SHA256 均与上次审查 manifest 一致。

本轮逐张查看当前五张正式 PNG，并核对五组 baseline/raw 文件：正式图 980×752，raw 1960×1504，所有 SHA256 与 capture manifest 一致；当前 release `data_table` binary 的 SHA256 同样匹配。图中表头/正文、四态外壳、前两行选中及 RTL 两端列和分页均无新视觉阻断。

新图采用 Light 原生标题栏，dark/Mocha 正文仍保持各自主题，没有据此误判为主题回退。本轮没有重新捕获或操作 OS/Rift；像素逐点差异范围来自提供材料，不冒称本轮重新计算。来源与完整性记录见 [source-and-visual.json](source-and-visual.json)。

维护文档补充了方向转换规则和新 modifier 的 snapshot/replay 要求；键盘文档也已澄清 Host action 可能先消费 chord，未改变键盘路由。

## 执行归属与剩余门槛

本轮亲自运行 **21 个 Rust 测试（2 core + 6 原始 Table + 13 正式 native）全部通过**，另核验四组原始 modifier characterization。该数量表示执行次数，不宣称互不重叠的覆盖。没有再重复完整 workspace/native、Clippy、package 或 release smoke；交付的 658/475/209 及完整矩阵保持其原执行归属。

查询的 [新 HEAD CI](https://github.com/eddix/gpui-rhai/actions/runs/37170330105) 对应精确 `16c60639`，当前为 **in_progress**，正在执行 Test；见 [原始查询](ci-current.json)。不能将旧 HEAD 绿灯或本地通过当成此 run 已通过。

剩余工作：

1. 新 HEAD CI 实际完成并成功；若失败，针对真实失败处理。
2. 既定实际 OS keyboard/focus、clipboard、IME preedit、VoiceOver 和适用窗口交互验收。
3. 物理 120Hz 门槛：现有设备记录为 60Hz，继续明确 pending。
4. 上述门槛完成后，按既定授权流程处理候选合入、PR/issue 处置及发行；无 patch crates.io 消费验证仍属于获准依赖发布后的步骤。

在没有新失败证据的情况下，不建议为本次验收再扩大功能或重构范围。五张同源基线已通过；不会把旧失败报告重写成成功，也不要求围绕新增审计文字无限重建相同产品。

本轮仅新增本目录；产品、正式测试、先前审计和 disktree-rhai 未修改，未进行 GitHub 外部写入。机器记录见 [verification.json](verification.json)，文件 hash 见 [SHA256SUMS](SHA256SUMS)。
