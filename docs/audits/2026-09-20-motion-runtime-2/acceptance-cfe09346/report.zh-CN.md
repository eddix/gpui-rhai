# Motion Runtime 2 第三轮复验：cfe09346

日期：2026-09-21（Asia/Shanghai）。审计提交：`cfe09346c19074c5180cc7a723e5bc87a375f881`，`fix: close Motion Runtime 2 acceptance gaps`。对照上一轮 `23874744ccdf92c887785a13a83716b24a3925c6`。

**结论：主要整改有效，性能改善已复现，但还不能全量验收通过。** 本轮发现 4 组问题：2 组 P1、2 组 P2。它们涉及目标绑定、窗口资源清理、完整计划的预算接纳，以及解析 inertia 的停止语义；都有独立程序证据。

这轮没有修改产品源码，只新增本目录的报告、探针与日志。不将项目早期的 API 破坏性变更列为缺陷。特别是 F01 的 headless 问题，是当前版本内部两条路径不一致，与旧版兼容无关。

## 验证结果

| 项目 | 本轮结果 |
| --- | --- |
| `cargo test --workspace --all-targets --all-features --locked --offline` | **501 通过** |
| `tests/native-keyboard` | **58 通过** |
| `tests/performance` | **2 通过、2 ignored** |
| fmt / 严格 all-features Clippy | 通过 |
| 上轮公共 API / lifecycle 探针 | 完整复跑，原复现输出回到期望 |
| 上轮 Canvas morph 原生正确行为测试 | **通过**：终点可点、起点不可点 |
| 本轮扩展原生测试 | **1 通过、1 失败**：Canvas 通过；timeline 子目标替换后未继续呈现动效 |
| release 微基准 | 1000 个 inertia：1 s p50 **0.600 ms**；8 s p50 **0.611 ms** |
| exact-commit GitHub Actions | 查询返回 `[]`，没有该 SHA 的运行记录，不能记为远端通过 |

版本仍为 GPUI `=0.2.2`、Rhai `=1.26.0`；测试使用本项目锁定依赖。**R** 表示本轮独立程序复现，**S** 表示源码调用链证据。原生测试驱动实际 GPUI 布局、prepaint、paint 与输入路径；物理屏幕观感及 display-link 时序不由测试平台替代。

复现入口：[reproduce.md](reproduce.md)。本轮原始行为：[probes.log](evidence/probes.log)；原生正确行为测试：[native-probes.log](evidence/native-probes.log)。本报告只收录通过运行或完整调用链确认的问题。

## F01 · P1 · R/S：目标已经解析到新 NodeId，但兼容 timeline 仍使用旧绑定

定位：[motion.rs:1009](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1009)、[motion.rs:1024](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1024)、[motion.rs:2002](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2002)。关联上轮 N04。

### 挂载后的 target replacement

每次 reconcile 都编译新的 tracks 并解析 `resolved_path`，但是兼容分支只更新 callback，然后把旧 `ActiveTimeline` 放回 map。新编译的 tracks 被丢弃；兼容判断只比较声明、owner 与 component incarnation，没有比较已解析的目标身份。

复现保持 parent key 和 timeline 声明不变，只把同 key `title` 的子节点从 Text 改为 Box：

- RetainedUiTree 正确把旧 `NodeId(2)` 卸载，并创建 `NodeId(3)`。
- 在 250 ms，motion snapshot 仍包含旧节点 width=125，**新节点没有样本**。
- 新增原生帧测试将声明设为 width 100→200 / 1000 ms，在 250 ms 替换子节点，再推进到 500 ms。真实呈现宽度为 **[100, 100, 100]**，当前子节点没有继续接收轨道。

这会影响保持容器 timeline、替换内容类型的 UI，也会影响新 generation 中声明相同但目标树变化的场景。单测 `retained_timeline_targets_resolve_to_the_target_node_identity` 只覆盖首次解析，未覆盖后续目标重建。

### 无 retained tree 的 helper 同样未消费计划结果

`reconcile_node_motion` 已把 direct declaration 编码为 `root/key:7469746c65`（key=`title`），并按这种编码验证 timeline target。但是在 `identities=None` 的情况下没有填充 `resolved_path`，采样退回 `root/title`。

本轮用当前公开 API，分别给相同 child 添加 direct opacity 和 timeline opacity，得到不同的 MotionKey。即使 target 验证通过，轨道也没有使用它验证过的那个属性身份。

**修改建议：**把已解析的 target/property binding 作为编译计划的正式输出，mounted 和 headless 都必须消费这一输出。兼容 rerender 复用时间进度时，仍需验证或更新 binding；保留了 timeline owner 不代表所有 child incarnation 都没变。已卸载目标不能继续出现在提交后的样本中。

**验收条件：**

- 首次绑定、同 NodeId rerender、子目标类型替换、兼容 reload 后 target NodeId 改变均覆盖。
- 父 timeline 保持连续时，新目标按当前 timeline position 接收样本；若项目选择重启策略，必须明确并测试，不能静默丢失轨道。
- 旧 target key 从样本消失，新 target 持续更新；原生测试确认实际尺寸变化。
- 同一 scene 的 direct 与 timeline 声明，headless/mounted 各自使用一致的属性身份；编码应只完成一次。

## F02 · P1 · R/S：窗口 teardown 既会误删邻居，也会给新窗口留下暂停状态

定位：[context.rs:370](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/context.rs:370)、[context.rs:374](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/context.rs:374)、[motion.rs:1574](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1574)、[motion.rs:1632](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1632)。属于本轮扩展的生命周期发现，关联之前的 domain/exit/suspend 设计。

### `w` 误删 `w2` 的 exit ghost

`release_window("w", …)` 取消普通 motion 时使用有分段边界的 scope 匹配，ghost 筛选却直接调用 `ghost.domain.starts_with("window:w")`。因此 `window:w2/view:v2/root` 也被当成待释放对象；随后不仅删除 descriptor，还调用 `cancel_node_scope` 删除它的实际 motion source。

本轮在同一 UiRuntimeState 中创建两个 lifecycle，令 `w2` 的 panel 开始退出，再 dispose/release `w`。`w2` 的 active ghost 从 **1 变为 0**，snapshot 变为空。`w2` 并未关闭。

### 挂起后 release，再用同名窗口创建新视图会冻结

`release_window` 调用 `cancel_node_scope`，但后者没有清理 `suspended_scopes`。新视图即使有新的 component incarnation、retained tree 和 source，仍命中旧 presentation scope 的暂停标记。

复现：运行 width 0→100 / 1000 ms，200 ms 时 suspend，随后 dispose/release；在同一 runtime、同一窗口与 view ID 下创建新 lifecycle。推进新动画 200 ms，期望 width=20，实际 **0，needs_frame=false**。

窗口 ID 的复用是公开窗口管理能力中的正常情形；完整 release 后，旧窗口的暂停状态不应跨越 teardown 边界。

**修改建议：**让 window/domain teardown 使用同一份精确所有权标识与统一清理清单，涵盖 live/exit、暂停标记、事件、预算预留及 presentation state。不要把普通“取消某些动画”和“永久销毁一个 presentation domain”混为同一种清理语义；前者在已暂停 view 中可能需要保留暂停状态，后者必须清空它。

**验收条件：**

- `w` / `w2`、`settings` / `settings2` 等相邻前缀窗口互不影响。
- suspend→dispose→release→同 ID reopen，新源正常推进，旧 handle/event 不复活。
- 一个窗口释放时，其他窗口的 ghost、普通动画、timeline、预算和暂停状态保持不变。

## F03 · P2 · R：N07 只解决了原遍历顺序，两个 timeline 之间仍会误拒绝

定位：[motion.rs:2473](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2473)、[motion.rs:2475](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2475)、[motion.rs:1080](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1080)。上轮 N07 未完全关闭。

本轮修复把 timeline 放到 direct source 前面处理，原来的“active timeline→idle timeline + direct”复现已通过。但它仍然逐个修改候选 runtime、逐个调用 `ensure_active_capacity`，没有计算整个候选计划的最终 active 数量。

新复现只使用一个节点和两个不冲突的 timeline：

| 状态 | timeline a（opacity） | timeline b（width） | active 总数 |
| --- | --- | --- | --- |
| 旧计划 | active | idle | 1 |
| 新计划 | idle | active | 1 |

Host budget=1。如果声明顺序是 **b,a**，先激活 b 时旧 a 还占着预算，返回 `ActiveBudget { actual: 2, limit: 1 }`；换成 **a,b** 则提交成功。两者最终计划完全等价。

这不是需要放宽 Host 限额，也不是回滚失效。问题在于合法 candidate 的可接纳性取决于内部遍历顺序。

**修改建议：**先形成包含保留、移除、替换及目标 playback state 的完整候选计划，按提交后的总资源做预算判断，再安装它。预算还要包括保留下来的 exit/Host 源与其他 domain 的预留，但不能同时收取待替换旧状态和新状态。

**验收条件：**同一个合法计划随机排列 declaration/timeline 顺序，结果和最终 resource usage 一致；同时覆盖 active→idle、轨道增减、direct/timeline/geometry 互换。超限计划仍需原子失败。只测试一种 source 类别的先后顺序不足以关闭此项。

## F04 · P2 · R：解析 inertia 在 10 秒上限处跳到渐近终点

定位：[motion.rs:2226](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2226)、[motion.rs:2315](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2315)。这是性能修复后的行为问题；不否认 N09 的采样成本改善。

无边界 inertia 已用解析指数解计算当前 position，但到 `inertia_settle_ms` 后，无论有没有配置 snap_points，都会直接把 position 改成 `inertia_target`。而 settle duration 最多 10 秒，即使此时速度远未衰减到阈值。

本轮使用合法且非极端的参数：from=0、velocity=100、friction=0.1，无 min/max，无 snap_points：

| 时刻 | 返回位置 |
| --- | --- |
| 9999 ms | 632.083769 |
| 10000 ms | 1000.000000 |

**1 ms 跳跃约 367.916**。按当前公式，10 秒时物理位置约 632.12、速度仍约 36.79；到达保护时长不等于到达渐近位置。此处没有用户配置的吸附行为。

**修改建议：**把完成状态与位置投影分开。若上限强制停止，保留停止时的实际样本；若要收敛到某个目标，需有明确、连续的终段或合法吸附语义。解析式的限值不能作为任意 duration cap 上的瞬时位置。

**验收条件：**无 snap_points 的低 friction 源，cap 前/中/后的 position 连续或误差受明确约束；direct 与 timeline 使用一致规则。覆盖 bounded/unbounded、bounce/no-bounce、真实 settle 与时长上限，确保成本优化不会改变停止边界的几何行为。

## 上轮 N01–N09 复验归档

| 上轮问题 | 本轮独立结果与结论 |
| --- | --- |
| N01 candidate generation | 新 generation 能取得新 handle，旧 handle 失效，250 ms 进度仍为 0.25；原问题通过 |
| N02 resume_reload | 恢复后 width=40、needs_frame=true；原问题通过。teardown 后重新创建的暂停清理另见 F02 |
| N03 unrelated rerender 清 ghost | 100 ms 后无关更新仍 active=1、opacity=0.9；原问题通过。跨窗口释放边界另见 F02 |
| N04 retained identity / key alias | root remount 产生新 handle 且从 0 开始；slash key 不再别名。新身份向 timeline sampler 传递仍有 F01 |
| N05 Canvas morph 命中 | 原生正确行为测试通过，终点可点、起点不可点；原问题通过。曲线极端缩放和完整视觉精度未作额外认证 |
| N06 policy projection | Reduced→None 的 direct 终态为 0；timeline 外层 autoreverse 的 None 与 Normal 终态均为 0；原问题通过 |
| N07 预算候选顺序 | 原“timeline→direct”复现通过；两个 timeline 交换状态的 F03 证明总体约束未闭合 |
| N08 terminal batch | render 时冻结 event Vec，closure 只交付捕获 batch；新增单元测试通过。原先“旧帧 drain 新事件”的调用链已修复，真实 display-link 的完整时序验证仍单列 |
| N09 inertia 历史积分成本 | release 结果基本不随动画年龄增长；原性能问题通过。无吸附情况下的停止跳跃另见 F04 |

### N09 的量化结果

使用上一轮原样微基准：release + thin LTO，10 次预热、100 次测量，仅 `MotionRuntime::tick + snapshot`，1000 个源：

| 动画年龄 | transition p50 | inertia p50 | inertia p95 |
| --- | --- | --- | --- |
| 1 s | 0.678 ms | 0.600 ms | 0.611 ms |
| 8 s | 0.592 ms | 0.611 ms | 0.636 ms |

上一轮 8 s inertia p50 约 6.097 ms，这次约 0.611 ms，约降低一个数量级。数据见 [bench-release.log](evidence/bench-release.log)。该结果仅认证无边界 inertia 的这个采样场景；不代表布局/GPU/所有 bounce 路径或 120 Hz 应用帧率都已达标。

## 实施建议

先处理 F01 的目标绑定和 F02 的 teardown，再处理 F03 完整计划预算、F04 停止语义。现有 generation、live/exit retention、Canvas 修复和解析采样可以保留；无需为了这几项重新设计整套 API。

后续测试最值得补充三个性质：**所有合法排列得到同一候选结果；所有已卸载身份不再被采样；所有已释放 domain 不再影响重建实例或邻居。** 这些性质比单独为某次 for 循环调整添加示例，更能避免下一轮出现同类遗漏。

本次保留的验证边界：未运行物理屏幕视觉比对、受控 display-link 或 120 Hz 测量。它们不被伪装成通过，也不因缺少测量而单独计为产品 bug。详细报告、复现程序和日志均留在本目录；产品源码未改动。
