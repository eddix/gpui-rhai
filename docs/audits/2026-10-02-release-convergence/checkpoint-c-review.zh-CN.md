# 0.1.8 检查点 C 独立复验

日期：2026-10-02。审查源码：`c6c1dc77f33406438c0e29c24013f99f72ce754b`；产品比较基线：`cda11ce7d80f817e5675543cc7a95f577b977320`。

## 结论

**REQUEST_CHANGES：发现 1 个计划范围内 P1，检查点 C 尚未通过，不能进入 D。**

独立复跑的完整正式 native **186/186**、core lib all-features **494/494**、正式 virtual matrix **5/5**，以及未修改的 R7 窗口 **6/6**、Ref **5/5**、Table **4/4** 均通过。窗口来源和长期 Ref 的整改得到源码及行为证据支持；Table 原宽表反例也确实关闭。

但新独立小探针确认：Table viewport 变化后，测量得到的新 viewport 没有可靠请求下一帧，percent/flex 列保持上一 viewport 的宽度。现有 fixture 的额外手动刷新覆盖了这个排帧缺口。不能用上述绿色结果豁免这一范围内违例。

本结论仅适用于计划 A–C 核心检查点，不是整个 RC、合入或发布批准。没有执行 GitHub review/comment、merge、publish 或 tag。

## 审查范围与方法

- 完整读取实施计划 329 行（包含 A/B/C、DoD、scope freeze 与后续授权边界）、ADR 0022 最终不变量补充、正式契约索引、R7 总报告及三个相关子报告。
- 使用 code-review-expert 的架构、可靠性、边界条件检查，并按 Rhai/GPUI context/entity/layout 技能要求核验本机锁定的 crates.io 官方源码；未用泛化记忆代替版本控制流。
- 产品 diff 为 12 个源码/组件文件，约 1,698 行新增、156 行删除；另审阅 4 个新增 native test 文件、最终模型和契约索引。新增历史审计资料作为输入，不将其视为产品修改。
- 保留已有效的 scoped batch state、最终 invocation manifest、ReadDependency owner/contribution、mount lease 和 Style COW；不要求 #91 通用来源系统或跨已卸载实例的 Ref 兼容。
- 所有测试均清除 `RUST_MIN_STACK` 覆盖，未增加全局 Rhai 预算、改断言、依赖、旧 probe 或正式测试。新探针只在本轮 evidence 下；复用 `tests/native-keyboard/target`，没有新大型 target 或 profile。
- 以上实测针对冻结源码；产品 HEAD 仍为上述 SHA。缺陷上报后，主实施者开始第 450 行的窄修 WIP，尚未纳入此报告或冒称复验通过。工作区的 USER_GUIDE/Table/embedding/multi-window 文档 WIP 不参与产品结论，也未由本 reviewer 修改。

## P1 · C-TABLE-01：测量后请求下一帧在 prepaint 中无效

定位：[table_layout.rs:447](../../../crates/gpui-rhai/src/table_layout.rs#L447)，实际无效调用在第 450 行。

`request_layout` 使用上次记录的 viewport 解析列宽（第 367–375 行）。`prepaint` 才读取实际 layout bounds、更新 viewport/border（第 401–445 行）。因此变化后需要一个真正排程的、有界 native follow-up frame，才能使用新测量值。

当前实现调用 `window.refresh()`。锁定的 `gpui-pre 0.3.7/src/window.rs:2272–2277` 仅在 `invalidator.not_drawing()` 时生效；第 268–269 行明确要求 `DrawPhase::None`。GPUI 的 element prepaint 位于 `DrawPhase::Prepaint`，所以这个请求是 no-op，不是下一帧请求。

### 公共 API 实测

探针：[table-frame-probe/src/lib.rs](evidence/checkpoint-c-independent/table-frame-probe/src/lib.rs)。使用正式 Table props、loading 状态、固定/percent/flex 三列；无虚拟行 realization、geometry Reader 或 ElementRef curry。

1. 先建立正确 300px viewport 基线：边框内宽 298，三列 `[120, 149, 29]`。
2. 正常节点 callback/state transaction 把宿主样式宽度改为 500px。
3. 推进正常后台时钟 128ms、运行 foreground，并用公开 `Window::simulate_next_frame` 交付所有已有 next-frame callbacks。回调数为 `[6, 0, 0]`，后续没有新的帧请求。
4. 实际 AX/presented Table viewport 为 **500px**，但三列仍为 **`[120, 149, 29]`**；期望为 **`[120, 249, 129]`**。
5. 在保存被断言的 actual 后，额外手动刷新一次作为 driver 正控，列宽立即变为 **`[120, 249, 129]`**。探针对正控另有断言，手动刷新不影响之前保存的错误 actual。

最终探针 **1 PASS / 1 FAIL**；精确失败单项重复运行再次 **0 PASS / 1 FAIL**。见 [主日志](evidence/checkpoint-c-independent/table-frame-followup.log)、[重复日志](evidence/checkpoint-c-independent/table-frame-followup-repeat.log)。

初版探针曾多刷一帧而全绿；确认 GPUI `open_window` 自身绘制初帧、普通状态 transaction 自身请求变化帧后，移除了这两个多余 refresh。另一个初版的缺少 app asset namespace 属于 fixture 配置错误，已修正，不计产品 finding。首次出现探针在最终合法驱动下通过，不把源码疑点扩写成未经实测的第二个缺陷。

正式 `table_extent.rs` 的 `command`、`settle` 都显式 refresh；其 `percentage_and_weighted_flex_use_viewport_not_extent` 在 viewport callback 后调用 `settle`，因此可以证明新宽度解算正确，却不能证明 Table 自己请求了后续帧。

### 影响与最小整改

这是 B2/C 已固定的 viewport 变化/实际列宽契约违例：实际容器已变化，但 header/body/extent 使用旧解算结果，需依赖无关输入或外部重绘恢复。不是新增 Table 能力或范围外增强。

只需修复测量发生变化时的有界 native 排帧入口，例如核验并使用锁定版本的 `Window::request_animation_frame`（第 2622–2625 行，它登记 next-frame callback 并通知当前 view）。不要通过全树 Rhai 重执行、全窗口循环重绘或扩大预算代偿。

将“先验证已呈现新 viewport，再交付已排程帧，不额外手动 refresh”的回归加入正式 suite；断言 percent/flex 新几何、后续空闲无连续帧请求和无额外 Rhai。修复后在精确新 SHA 复验此项及原 Table/Ref 交叉，再重新给出 C 结论。

## 三个 R7 工作包的复验

| 工作包 | 独立结论 | 源码/行为依据 |
| --- | --- | --- |
| B1 窗口来源 | 通过已测范围 | `QueuedWindowCommand` 保存 origin 和 target identity；入队 `enqueue_open/enqueue_target` 建立 reservation 连续性；mount 在 init 前 qualify、native bind 只补同身份；执行/defer 检查原 source/current native target；取消 Open 只释放同 reservation。正式窗口组合和原 6 项均通过。 |
| B2 横向范围 | 原 R7 反例关闭，但未达到完整 DoD | 单一 viewport plan、direct track、RTL 逻辑距离、controlled/native override 共用宽度来源；实际尾列可达、四态非零 offset、header/body 同步及 0-Rhai 横滚/preview 通过。新 viewport 排帧 P1 阻止工作包验收。 |
| B3 长期 Ref | 通过已测范围 | logical ref readers 持续存活；reconcile 比较 binding，sync_ref_readers 迁移 native node 观察；direct/ref 独立；reset/prune/final manifest/release scope 同步清理；runtime snapshot 包含 logical/native subscriptions，失败恢复 last-good。正式 8 项、原 5 项和对应 units 通过。 |

### B1 检查细节

- `window.rs:80–123`、`:229–261`、`:422–520` 区分 Rust Host 来源、脚本来源及目标注册实例；Rhai 参数不创建 authority。
- `app.rs:2534–2551`、`:4981–5074` 不再用正在 pump 的 self 重新授予权限；Focus/Close 在 defer 仍验证原 source 和 target。
- 验证 timer 成功→失败/失败→成功，Open/Close/Focus，source dispose/drop/native close/remount，retained clone 正控，live/suspend 正控，subscription/真实鼠标/worker task 入口、同名 target 替换以及取消 Open 后真实重新使用 ID。
- 同批 Open→Focus 和 Open→Focus→Close 验证同 reservation 关联实际 native instance。没有把一次 enqueue 返回 Ok 作为 native 成功。

### B2 检查细节

- `table_layout.rs:205–296` 的 percent 基准是 viewport，不是 extent；flex 在固定/percent 宽度之后分配，feasible minimum/max 与 accepted native signal 合并。数值权重/混合 min-max 的 units 通过。
- `renderer.rs:3656–3727`、`node.rs:1858–1908` 让 header 与 retained virtual body 消费同一计划，direct child track 建立真实 GPUI content extent；不再依赖孙级溢出。
- Native extent 9 项覆盖 Array/Native、LTR/RTL、fixed/mixed、小于/等于/大于 viewport、sole/final column、主题 px/rem inset/typography、四态、resize/autofit、bounded realized rows 与横滚/preview 无 Rhai。
- 原 R7 实测 LTR A.x **1→−79**，RTL C.x **−121→1**，四态固定/fill 高度和已接受宽度保持；普通 scroll driver 正控仍有效。这些成功与新排帧 P1 并不矛盾。

### B3 与旧模型保留

- `element_ref.rs:146–266` 的 logical observer 在 unknown/bound/rebind/removed 间持续；unchanged binding 不重复 dirty。
- `geometry.rs:764–852` 与 `context.rs:650–682` 同步 retarget、direct/item contribution reset/prune、owner/provider teardown；旧 node 不再误唤醒已转绑 reader。
- `context.rs:794–896` 的 snapshot/restore 包含 ElementRefRegistry、GeometryRegistry 和各 presentation snapshot；native failed-effect candidate/retry、suspend/resume 正控通过。
- scoped state/最终 invocation graph 的 direct/透明/有容器组件、并列/父子目标、裁剪和失败重试由既有生命周期 units 及正式 virtual 5 项复跑确认，未要求重写有效模型。

## 独立执行清单

全部来源均为本 reviewer 实际执行；不是复制实施者的结果。默认 feature core 464 与 all-features core 494 是重叠集合，不相加计数。

| 命令/集合 | 结果 | 日志 |
| --- | --- | --- |
| 完整 native workspace，`--locked -- --test-threads=1` | 186 PASS | [native-default-stack.log](evidence/checkpoint-c-independent/native-default-stack.log) |
| core `--lib --locked` | 464 PASS | [core-lib.log](evidence/checkpoint-c-independent/core-lib.log) |
| core `--lib --all-features --locked` | 494 PASS | [core-lib-all-features.log](evidence/checkpoint-c-independent/core-lib-all-features.log) |
| `virtual_transactions` + `virtual_read_contributions` | 3 + 2 PASS | [virtual-fixed-matrix.log](evidence/checkpoint-c-independent/virtual-fixed-matrix.log) |
| 未改 R7 runtime-core runner | 6 PASS | [round7-window.log](evidence/checkpoint-c-independent/round7-window.log) |
| 未改 R7 geometry-reads | 5 PASS | [round7-ref.log](evidence/checkpoint-c-independent/round7-ref.log) |
| 未改 R7 resize-controls | 4 PASS | [round7-table.log](evidence/checkpoint-c-independent/round7-table.log) |
| 新 measured-frame probe，locked/offline/default stack | 1 PASS / 1 FAIL | [table-frame-followup.log](evidence/checkpoint-c-independent/table-frame-followup.log) |
| 精确失败项独立重复 | 1 FAIL | [table-frame-followup-repeat.log](evidence/checkpoint-c-independent/table-frame-followup-repeat.log) |

正式 native 中窗口 authority 5、deliveries 6、origin 2、ownership 6，Ref 8、Table boundary 8、extent 9 均计入 186，不再另加。keyboard 63 项保留正常终止、受控拒绝、Escape、suspend/offscreen 和 focus/input 对照；Canvas transformed selection 4 项覆盖非对称 inset 与变换。

工具链：rustc/cargo **1.95.0**；平台：macOS **27.0.1 / 26A434 / arm64**。依赖仍为 crates.io `gpui-pre/gpui-pre-platform =0.3.7`、`rhai =1.26.0`；Rhai 默认 + internals/metadata/serde，无 sync，stored call context 位于 crate-local `ScriptInvocationContext` volatile adapter。GPUI App/Window/entity/subscription 生命周期及 Taffy direct child scroll 控制流直接核对了 registry 源码。正式 native 的 test-support 仍在独立 workspace，没有进入 release 依赖图。

## 未覆盖与后续边界

- 没有声称执行全部 entry/source/pump/lifecycle 笛卡尔积；按固定矩阵选择必要组合并检查统一入口。未新增随机大型 fuzz 或改变合法目标提交语义。
- 没有本轮独立 strict Clippy、package、release smoke、远端 CI、release 性能/120Hz、Linux X11/Wayland、VoiceOver 或五张真实视觉基线结果；它们属于后续最终 RC 门槛，不能由本报告替代。
- 没有接管 D1/D2/D3，未实施 #96/#97/#99，也未扩入 #91 通用来源、跨已卸载 Ref、per-asset revision 等后续功能。disktree-rhai 完全排除。
- 未修改原审计资料、产品源码、正式测试、依赖、配置；本 reviewer 只新增本报告、独立日志、verification manifest 与小 probe。缺陷上报前，原 R7/产品/锁文件 `git diff --exit-code` 检查为无改动；后续主实施者的窄修 WIP 属于下一精确候选的复验范围。

下一步是修复 **C-TABLE-01** 并接受同范围独立复验，不是开始 D。无需为这一已经授权范围内的常规整改重新等待人工许可；合入/发布/tag 禁令继续保留。
