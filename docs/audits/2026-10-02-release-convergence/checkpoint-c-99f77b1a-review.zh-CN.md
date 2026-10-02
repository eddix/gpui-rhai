# 0.1.8 检查点 C 独立复验：99f77b1a

日期：2026-10-02。精确源码：`99f77b1afce4c326536f1bbccf67df0890a43854`。本次比较：`c6c1dc77f33406438c0e29c24013f99f72ce754b` → 当前冻结候选。

## 结论

**APPROVE：检查点 C 通过，可以按既有授权进入 D。**

旧报告的 **C-TABLE-01 / P1 已关闭**。原独立失败 probe 未作任何修改，在新源码上 **2/2 通过**；正式 Table frame-demand **2/2 通过**，包括 native follow-up 零 Rhai 与最后无新帧需求的断言。

本轮在精确新 SHA 独立执行：正式 native 相关矩阵 **113/113**（固定矩阵 50 + keyboard 63）、core lib all-features **494/494**、正式 virtual matrix **5/5**，全部通过。没有新增 P0/P1/P2 finding。

本结论只批准计划 A–C 的核心检查点，不等于整个 0.1.8 RC 已就绪，也不是合入、发布或打 tag 的批准。D/E/F/G/H 以及最终独立验收仍按计划推进。

## 比较范围与版本核验

新 diff 仅 3 个文件、259 行新增/1 行删除：

- `table_layout.rs`：用正确的 native frame-demand API 替换 prepaint 中无效的 refresh（1 个行为调用变化及说明）。
- `table_frame_demand.rs`：提升旧独立 probe 为正式 2 项测试，保留实际几何断言，增加零 Rhai 与有界空闲断言。
- `runtime-contract-tests.md`：增加 TABLE-03 索引。

`app/window/context/lifecycle/element_ref/geometry/node/renderer/table.rhai`、依赖和锁文件相对于旧候选均未变化。没有加入 D 的 API、改预算、依赖来源或旧 probe。

本轮仍按 code-review-expert 及 Rhai/GPUI context/entity/layout 技能核查。依赖保持 crates.io 官方 `gpui-pre/gpui-pre-platform =0.3.7`、`rhai =1.26.0`；Rhai default + internals/metadata/serde，无 sync；volatile stored context 仍隔离于 `ScriptInvocationContext`。工具链 rustc/cargo 1.95.0，macOS 27.0.1 / 26A434 / arm64。

## C-TABLE-01 的关闭证据

旧问题：`request_layout` 使用前一测量 viewport；`prepaint` 更新实际 viewport 后调用 `Window::refresh`，但锁定 GPUI 只在 `DrawPhase::None` 允许 refresh，所以未排程后续解算帧。

新 [table_layout.rs:453](../../../crates/gpui-rhai/src/table_layout.rs#L453) 调用 `window.request_animation_frame()`。直接核验锁定官方源码：

- `gpui-pre-0.3.7/src/window.rs:2622–2625` 捕获当前 native view，并在 next-frame callback 中 `cx.notify(entity)`。
- `:2601–2607` 的 `on_next_frame` 注册 callback、调用 platform `schedule_frame` 并唤醒 frame source。
- 测试使用公开 `simulate_next_frame`（`:2633–2640`）交付已登记 callback，而不是用额外 refresh 伪造下一帧。
- 调用仅发生于 viewport/border 真正变化的 `changed` 分支；不更改 Rhai dirty/dependency、state transaction 或全树提交逻辑。

### 原未修改探针

来源仍是 [原 probe](evidence/checkpoint-c-independent/table-frame-probe/src/lib.rs)，新日志单独保存为 [unchanged-table-frame-probe.log](evidence/checkpoint-c-99f77b1a-independent/unchanged-table-frame-probe.log)。

| 场景 | 排程 callback 数 | 实际列宽 | 结果 |
| --- | --- | --- | --- |
| 首次 measured viewport 300 | `[3,1,0]` | `[120,149,29]` | PASS |
| viewport 300→500 | `[8,1,0]` | `[120,249,129]` | PASS |

变化场景实际 presented Table viewport 为 **500px**。推进正常时钟 128ms，交付已经排程的帧后即得到正确新宽度。旧候选在完全同一 probe 上曾保持 `[120,149,29]`，并且只有额外手动刷新才恢复。

额外 refresh 仍只在保存 `actual` 后作 driver 正控；新 actual 的正确性不依赖它。probe 源码 SHA256 前后相同：`ac62a7c0aaa4d99673964fed330815b86ef81e2ce040bf6a7783b4916a993283`。

### 正式 TABLE-03

[正式测试](../../../tests/native-keyboard/tests/table_frame_demand.rs) 保留上述输入/实际几何判据，并增加：

- 清除 source transaction 的已有 timing 后，正常 clock 和 follow-up frame 的 Rhai operation 总数为 **0**。
- 三次交付后最后一个 callback count 为 **0**，没有持续测量→重绘循环。
- 变化场景先断言实际 presented viewport 已为500；不是用状态变量假装布局已完成。
- 实际错误值在正控 refresh 之前已保存；最后几何断言仍比较保存的值，不放宽期望或以正控结果代替。

正式两项独立日志中同样得到 `[3,1,0]` / `[8,1,0]` 和正确几何，零 Rhai/有界空闲断言均通过。修复是 source-verified 的窄 native 排帧，不是通过 Rhai 重执行或全窗口循环刷新代偿。

## 固定矩阵复验

| 契约 | 本 SHA 的独立结果 |
| --- | --- |
| WIN-01/02 | authority 5、origin 2、deliveries 6、ownership 6 全通过；原 origin/target reservation 与 native/deferred 校验没有代码变化 |
| TABLE-01/02/03 | boundary 8、extent 9、frame demand 2 全通过；四态、LTR/RTL、两数据来源、accepted/native resize、实际尾列可达、宽度同步、零 Rhai 热路径及 measured frame demand 保持 |
| REF-01/02/03 | native Ref 8 与对应 core units 通过；unknown/bound/rebind/removed、same-node、formal row、retarget、contribution prune、provider/owner teardown、failed effect rollback/resume 保持 |
| VIRT-01/READ-01 | 正式 virtual 5 与 core lifecycle/read units 通过；scoped delta/final manifest、透明组件、并列/父子目标、旧贡献清理与失败 rollback 未改模型 |
| 几何/正常终止/输入交叉 | transformed selection 4 和 keyboard 63 全通过，包括 Table native resize/autofit、窗口实际宽度、flex、bounded virtual viewport、Ref 自愈及 focus/input/suspend 对照 |

没有展开全部 source/pump/lifecycle 笛卡尔积；沿用批准的必要组合和统一入口审查。没有以当前 green build 单独替代实际窗口、几何或排程结果。

## 独立执行记录

所有命令均显式清除 `RUST_MIN_STACK`，复用现有 root/native target，不增加 profile 或大型 workspace。没有修改产品、正式测试、旧审计、原 probe 或其断言。

| 本轮实际执行 | 结果 | 日志 |
| --- | --- | --- |
| native `table_frame_demand/table_extent/table_boundaries/element_ref_subscriptions/window_authority/window_command_origin/window_command_deliveries/window_ownership/transformed_selection`，locked、单线程 | **50 PASS** | [native-fixed-matrix.log](evidence/checkpoint-c-99f77b1a-independent/native-fixed-matrix.log) |
| native `keyboard`，locked、单线程 | **63 PASS** | [native-keyboard-termination.log](evidence/checkpoint-c-99f77b1a-independent/native-keyboard-termination.log) |
| core `--lib --all-features` + 正式两份 virtual suites，locked | **494 + 5 PASS** | [core-and-virtual-matrix.log](evidence/checkpoint-c-99f77b1a-independent/core-and-virtual-matrix.log) |
| 原未改 measured-frame probe，locked/offline、单线程 | **2 PASS** | [unchanged-table-frame-probe.log](evidence/checkpoint-c-99f77b1a-independent/unchanged-table-frame-probe.log) |

原 probe 与正式两项是重叠语义，不相加声称更多独立契约。完整 native186、原 R7 6/5/4 在旧候选的独立结果仍保存在 [旧报告](checkpoint-c-review.zh-CN.md)，本轮没有把它们改写为新 SHA 的全量188结果。

## 资料保护与未覆盖

- 旧 `c6` 的 REQUEST_CHANGES 报告、失败/重复日志、probe 源码 SHA256 在复验前后完全相同；新通过日志只在 `checkpoint-c-99f77b1a-independent/`。
- 末尾仍确认 HEAD 为 `99f77b1afce4c326536f1bbccf67df0890a43854`，产品/测试/锁文件工作区无未提交改动；已有公共文档 WIP 不参与本结论。
- 本轮没有独立 strict Clippy、完整188 native、package、release smoke、远端 CI、release 性能/120Hz、Linux X11/Wayland、VoiceOver 或五张真实视觉基线结果。实施者自测的 Clippy/其他日志不冒称本 reviewer 执行。
- 没有集成 #96/#97/#99、扩展 #91、跨卸载 Ref 或 per-asset API；disktree-rhai 不在范围。没有 GitHub 操作、commit、merge、publish 或 tag。

**下一步：主实施者可进入 D 的独立集成工作包。** C 已通过不解除用户对合入/发布/tag 的限制；整个 RC 的最终门槛与独立验收仍未完成。
