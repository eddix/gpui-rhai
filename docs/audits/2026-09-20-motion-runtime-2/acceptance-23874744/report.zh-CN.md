# Motion Runtime 2 整改复验：23874744

审计日期：2026-09-20。基线：`23874744ccdf92c887785a13a83716b24a3925c6`（`fix: unify Motion Runtime 2 lifecycle`），对照上一版 `9f4ba04fc9c5198d7164359f47139a5d306de1bc`。

**结论：整改方向正确，多个基础缺陷已修复，但本轮不能全量验收通过。** 发现 9 组需继续处理的问题，其中 5 组 P1、4 组 P2。主要问题从单个 API 的错误，转移到了热重载、保留树、退出对象、策略投影和帧提交之间的交接。

本轮没有修改产品代码。只新增本目录的报告、复现程序和证据。不将 pre-1.0 API 破坏性变更列为缺陷，不要求历史兼容层，也不要求重写整个动效系统。

## 证据范围

| 检查 | 结果 | 能证明什么 |
| --- | --- | --- |
| workspace/all-targets/all-features，locked/offline | **490 通过** | 当前产品测试基线无失败 |
| native-keyboard，locked/offline | **57 通过** | 包括官方 Gallery 真实 mount、首帧、reorder/play，以及 None 下 layout 的回归 |
| performance，locked/offline | **2 通过、2 ignored** | 执行到的结构与功能测试通过；两项被忽略的测试未执行 |
| fmt、严格 Clippy | 通过 | 格式和静态检查通过 |
| 新公共 API / ScriptLifecycle 探针 | 完成 | 下文 N01–N04、N06–N07 的实际输出；探针打印缺陷不意味着验收通过 |
| 新原生 Canvas 正确行为测试 | **1 失败** | 真正鼠标输入仍命中 morph 起始路径，不能命中最终路径 |
| release 采样微基准 | 完成 | inertia 的重复积分成本；不等于整机应用帧率测试 |
| exact-commit GitHub Actions 查询 | 返回 `[]` | 未找到这个 SHA 的运行记录；不是 CI 失败，也不能记为远端通过 |

证据标记：**R** = 本轮独立程序复现；**S** = 根据完整调用链及项目锁定的依赖源码确认；**G** = 尚需补充的验证。原生测试使用 GPUI 0.2.2 测试平台，可检查布局、绘制调用和输入分派，不是物理屏幕截图或 120 Hz 验证。

复现方法见 [reproduce.md](reproduce.md)，原始输出在 [evidence](evidence/)。旧版报告及证据保持不变。

## 本轮发现

### N01 · P1 · R：实际热重载入口仍把 timeline 注册在旧 generation

位置：[lifecycle.rs:867](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/lifecycle.rs:867)、[lifecycle.rs:1077](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/lifecycle.rs:1077)。关联 M02。

`reload` 用 candidate generation 执行新脚本，但 `reconcile_motion` 从 `self.compiled.generation()` 取所有权代次；`self.compiled = candidate` 要到事务成功后才执行。结果是新 view 已进入 generation 2，timeline handle 仍属于 generation 1。`resume_reload` 有同样的问题。

本轮通过真实 `ScriptLifecycle::reload` 复现：保持完全相同的 timeline 声明，在 250 ms 重载后，进度仍为 0.25，但新 generation 的 `ctx.motion_handle("intro")` 返回 `UnknownTimeline`，旧 handle 在 MotionRuntime 中仍为 `Playing`。

现有 `compatible_generation_migration_preserves_progress_but_rekeys_authority` 测到了底层 helper 能正确迁移，未覆盖 lifecycle 是否把正确的 generation 传进去。下一次碰巧触发的普通 rerender 可能修正代次，这不是可靠的交接协议。

**修改方向：**把 candidate generation 显式纳入 reconcile context，和新树、callback、incarnation 在同一事务中提交。保持连续进度，并在提交成功时使旧 handle 失效。

**验收：**分别通过 `reload`、`resume_reload` 重载相同声明；新回调立即取得并控制新 handle，旧 handle 明确失效，进度连续；失败重载保留旧权限和旧进度。不要只测内部迁移 helper。

### N02 · P1 · R：挂起期间发生重载，恢复后动效永久冻结

位置：[lifecycle.rs:973](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/lifecycle.rs:973)。关联 M09。

普通 `resume` 会调用 `motions.resume_node_scope`、`geometry.delay_motion`；`resume_reload` 只恢复 timers，随后把 lifecycle 状态设为 Running、清掉 `suspended_at`，没有解除 MotionRuntime 内的 suspended scope，也没有平移 geometry 的时钟。

复现：width 0→100 / 1000 ms，运行 200 ms 后挂起，500 ms 后用相同脚本 `resume_reload`，再推进 200 ms。期望 width=40，实际仍为 **20**，`needs_frame=false`，但 lifecycle 已为 **Running**。因此不是隐藏窗口正常暂停，而是公开恢复入口未恢复动效。

**修改方向：**让普通恢复与带重载恢复共享一个事务内的 motion/geometry 恢复步骤，明确旧源与新源各自的时间基准。解除暂停不能早于可回滚的 candidate 接纳，也不能遗漏 geometry。

**验收：**旧源、新增源、timeline、layout/trigger 都覆盖 suspend→resume_reload；另一个窗口持续采样不得改变暂停位置；失败恢复必须仍保持暂停。

### N03 · P1 · R：退出动画被下一次无关 rerender 提前删除

位置：[motion.rs:2177](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2177)、[motion.rs:1340](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1340)、[lifecycle.rs:1118](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/lifecycle.rs:1118)。关联 M08。

ghost 路径现在放在 `window:…/view:…/root/ghost:{NodeId}` 下，关闭 view 时能正确取消。但普通 reconcile 对同一个 root 执行 `retain_node_scope(root, live_declaration_keys)`；ghost 不在新 UiNode 树的声明集合中，因此也被一起清掉。

复现：移除一个带 1000 ms opacity exit 的 panel，正确创建了 active=1 的 ghost；100 ms 后只更新仍存活的 counter，active 变为 **0**、samples 变为空。下一次原生渲染清除 ghost descriptor，退出动画提前结束。

**修改方向：**让 scope 表示生命周期归属，让 live tree 与 exit scene 分别管理自己的源集合。普通 live reconcile 只能回收自己拥有的源；view/window teardown 才同时回收两者。不建议用难以维护的字符串前缀特判补丁。

**验收：**exit 中多次无关 state/store 更新、多个 sibling 先后退出、旧 key 重新出现、窗口关闭；分别断言 ghost 数量、采样进度、预算释放和非交互性。当前只验证“刚移除后存在 ghost”不够。

### N04 · P1 · R：动效身份仍不等于 retained node 身份

位置：[motion.rs:2161](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2161)、[motion.rs:919](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:919)、[motion.rs:2802](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2802)。关联 M02、M06、M10。

两个复现揭示同一个根因：

- **真实 remount 没有新身份。** 根节点 key 从 `one` 换成 `two`，RetainedUiTree 明确报告旧 `NodeId(1)` unmounted；motion 根路径仍固定为 `root`，同 owner/incarnation 下的相同 timeline spec 被当作兼容 rerender。旧 handle 保留，500 ms 时进度为 0.5，而新节点应从自己的 mount 起点开始。
- **不同合法 key 路径发生别名。** 一个 child 的 key 是 `a/b`，另一个 child key 是 `a`、其 child key 为 `b`。保留树中的节点不同，motion 都被编码为 `root/a/b`，触发错误的 `PropertyOwnerConflict`。`NodeKey` 没有禁止 `/`；文件路径或外部数据 key 很容易包含它。

增加 runtime/generation/instance 字段改善了 handle 权限，但不能弥补根层缺少 node incarnation，以及路径序列化不唯一的问题。

**修改方向：**从本次 retained reconcile 的候选 NodeId / 节点 incarnation 建立场景身份，再绑定 property 和 allocated motion instance；timeline 的人类可读 target 只用于解析。若保留路径键，必须统一采用可逆编码的分段结构，且纳入根节点真实 mount 身份。不要在每个 renderer/virtual/ghost 路径各自拼字符串。

**验收：**同 key 普通 rerender 连续；root remount/type replacement、新 component incarnation 重启并使旧 handle 失效；包含 `/`、数字及 `item:` 等字面值的数据 key 不别名；虚拟项 reorder 保持正确身份。

### N05 · P1 · R/S：Canvas 的 affine 修好了，morph/trim 命中仍使用旧形状

位置：[renderer.rs:464](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/renderer.rs:464)、[renderer.rs:2594](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/renderer.rs:2594)、[canvas.rs:371](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/canvas.rs:371)。关联 M12。

新的 inverse affine 确实修复了动态 rotate/scale/skew 的坐标逆变换。但绘制根据 `motion.path_progress` 插值 MorphPath，pointer context 只保存 affine 参数，然后调用没有 progress 入参的 `scene.hit_test`；后者对 MorphPath 始终使用 `from`。

原生复现：水平线从 y=20 morph 到 y=80，固定 `path_progress=1`。发送真实鼠标输入，y=80 返回 `canvas_key=null`，已经不再绘制的 y=20 反而返回 `shape`。新增正确行为测试稳定失败。

同一调用链中，普通 Path 的 trim 只影响 paint，hit_test 仍使用完整路径；这是源码确认的相邻问题。Line/Path 的 stroke 宽度在 paint 中保持原宽，而 inverse hit test 会连同距离一起反缩放，非等比缩放下也需要统一 stroke 语义；这部分尚未做独立原生复现。

**修改方向：**在一帧内生成包含当前 morph/trim/stroke/clip 的呈现场景，由 paint 和 hit test 共用。不要只在坐标入口增加 inverse matrix，而让几何形状继续各自计算。

**验收：**morph 首/中/末帧、trim 未绘制段、非等比 stroke、clip，以及它们与 affine 的组合；同时验证“新形状可点”和“旧形状不可点”。

### N06 · P2 · R：None 的终态投影依赖进入路径，且忽略 timeline 外层 autoreverse

位置：[motion.rs:696](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:696)、[motion.rs:1739](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1739)。关联 M03、M05。

两种不一致：

- direct opacity 0→1，2 次迭代、autoreverse。Reduced 先投影到 1；随后 Host 改为 None，`apply_preference` 只处理 active，不重算已 settled 声明，因此仍为 1。直接以 None 接纳同一声明会得到实际终态 0。
- track 是 opacity 0→1，外层 timeline 设置 2 次迭代、autoreverse。Normal 完成得到 0；None 分支逐条取 track terminal，完全跳过 timeline 的外层时间映射，得到 1。

None 下确实不再持续请求动画帧；问题是它没有得到一致的声明终态，可能让本应结束隐藏的节点保持可见。此处不是要求实现更复杂的 reduced-motion crossfade。

**修改方向：**有限 timeline 的静态终态复用正常时间映射在最终位置的完整采样，再叠加显式 reduced fallback；策略变更需要重新投影仍有效的声明，包括已由旧策略 settle 的源，且不重复发完成事件。

**验收：**Normal/Reduced/None 的初始接纳与运行中切换矩阵；source 与外层 timeline 的 repeat/autoreverse、sequence 同 property 多段、无限循环 fallback。相同最终策略和相同声明应有同一静态结果。

### N07 · P2 · R：候选预算检查仍按接纳顺序计费，拒绝合法替换

位置：[motion.rs:2182](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2182)。关联 M07。

旧计划有一个 active timeline `intro`。新计划将同名 timeline 改为 `autoplay:false`，同时新增一条 direct width 动效。预算 active_motions=1，新计划的实际 active 数是 1，应能提交。

当前逻辑先只删除“不再出现在新 logical key 集合”的 timeline，保留旧 active `intro`；随后先启动 direct source，尚未把旧 timeline 替换成 idle，因此返回 **ActiveBudget { actual: 2, limit: 1 }**。真实 ScriptLifecycle 探针已复现。

回滚本身有效，但“没有污染旧状态”与“完整接纳合法新计划”是两个不同保证。

**修改方向：**在纯计划阶段确定所有源/轨道的移除、替换、保留及目标 playback state，统一算提交后的总资源；再执行候选提交。不要让内部 for 循环顺序决定预算结果，也不要临时放宽 Host 预算。

**验收：**同名 timeline active→idle、减少轨道、与 direct/geometry 源互换；交换声明顺序不影响是否接纳。超限时仍需原子失败。

### N08 · P2 · S/G：on_next_frame 回调仍可能提前交付尚未画出的 terminal event

位置：[app.rs:3293](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/app.rs:3293)、[app.rs:3338](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/app.rs:3338)、[motion.rs:428](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:428)。关联 M11。

每次 render 注册一个 callback，callback 在执行时才 drain 整个 domain 的 event 队列。队列中的事件没有 frame epoch，也没有绑定到注册 callback 的那一帧。

项目锁定的 GPUI 0.2.2 `window.rs:1026` 在下一次 display request 中**先**执行 `next_frame_callbacks`，随后在 `1048` 才 draw/present。于是可能出现：

1. F1 渲染并注册 callback；
2. F1 之后的输入事务把运行中的 timeline 切到 None（或 cancel/replace），产生新 terminal event 并要求重绘；
3. F2 display callback 先执行 F1 的 closure，drain 到刚产生的新 event；
4. 此时静态终态（或取消/替换后的那一帧）尚未 draw。若 on_complete 删除节点，声明的终态可以从未呈现；on_cancel 重入也存在相同的事件归帧问题。

这次改用 GPUI frame 回调是正确进展，但 callback 本身不能替代事件与已呈现帧的关联。回调失败隔离也已改善，不需要推翻那部分。

**修改方向：**在采样/渲染时冻结属于该帧的 event batch，或为事件标注 presentation epoch；提交边界仅交付已经满足 epoch 的 batch。处理重入期间新产生的事件、旧代次、关闭或挂起中的 view。

**验证缺口：**GPUI 0.2.2 test window 的 `on_request_frame` 是空实现，测试支持代码在 effect flush 里直接 draw。因此当前 native suite 的绿色状态不能证明生产 display callback 顺序。本轮该项是源码时序结论，未伪称已做真实 display-link 复现。需有显式可驱动 frame callback 的测试 Host，验证“采样→画出→callback”的顺序，并覆盖两帧之间输入。

### N09 · P2 · R/S：inertia 每帧重新积分历史，采样成本随动画年龄增长

位置：[motion.rs:2027](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2027)、[app.rs:3323](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/app.rs:3323)。关联 M05，也属于新增性能发现。

`sample_inertia_at` 每次从 `spec.from/velocity` 开始，重复 `ceil(elapsed_seconds × 240)` 次积分，最多 2400 步；生产路径先 `tick_scope` 再 `snapshot`，本 view 的 active 源会被重复采样。一个完全没有 min/max/bounce 的指数衰减源，也承担整段历史积分成本。

release + thin LTO、aarch64 macOS、Rust 1.94，10 次预热、100 次测量，1000 个仍 active 的源：

| 动画年龄 | transition p50 | inertia p50 | inertia p95 |
| --- | --- | --- | --- |
| 1 s | 0.626 ms | 1.400 ms | 1.500 ms |
| 8 s | 0.593 ms | 6.097 ms | 6.171 ms |

数据来自 [交付脚本复跑日志](evidence/delivered-bench.log)，首次测量亦保留在 evidence。该测试只测 `tick + snapshot`，**未计入布局、绘制、Rhai 或 GPU**。数值随机器负载变化，但时间复杂度来自源码；无须把调试构建的较大耗时外推到生产。当前 Host 默认 active_motions=8192，1000 个源仍在默认接纳范围内。

**修改方向：**无约束 inertia 使用解析解；有反弹的路径使用预编译分段解或带预算的 checkpoint/incremental 求解，同时保留确定性 seek。每帧复用同一份采样结果，并避免为另一个 domain 的呈现重复计算整个共享 runtime。无需重新引入每帧 Rhai 调用。

**验收：**同源在 1/5/9 秒的采样成本不应线性随历史步数增长；direct/timeline 一致性、seek、反弹、snap、retarget 保持正确。再补真实 frame 的 p95/p99 和内存分配指标。当前普通 performance 测试未覆盖 motion 稳态采样。

## 上轮 M01–M12 的验收状态

| 原发现 | 本轮判断 |
| --- | --- |
| M01 Gallery | 核心入口通过：真实 mount/首帧、reorder/play；完整视觉效果和所有控件仍不等于已全部验证 |
| M02 handle 权限和生命周期 | runtime/view 隔离、显式移除后的旧实例拒绝已有有效修复；N01、N04 未关闭 |
| M03 policy | None restart、None layout 已通过；N06 静态投影不一致未关闭 |
| M04 timeline 状态与快照 | pause/seek、重复 play、future track 复位的原问题已修复；本轮独立 pause/seek 探针通过 |
| M05 物理一致性和终态 | 共用 sampler、spring velocity、direct autoreverse、极端参数防护有实质修复；N06、N09 仍需处理 |
| M06 target / property owner | 普通冲突、缺失目标、类型能力和原子拒绝已落实；N04 身份别名影响计划正确性 |
| M07 预算和事务 | play/restart 复查、geometry reservation、公开 reconcile 回滚已落实；N07 合法替换误拒绝仍存在 |
| M08 exit ghost | 按 view geometry 创建、domain 清理、unsupported subtree 显式拒绝已落实；N03 普通 rerender 提前删除 |
| M09 suspend 与 clock | 普通 suspend/resume 和 slot Host clock 已修；N02 resume_reload 漏恢复 |
| M10 virtual item path | 常规 key 的 item 路径已统一；N04 通用路径编码与真实 remount 身份仍需解决 |
| M11 frame 与 terminal callback | 从 timer 改为原生 frame、分域、独立事务是正确修复；N08 epoch 与生产回调时序仍未闭合 |
| M12 Canvas paint / hit | affine rect 的逆变换和 clipped affine 拒绝已落实；N05 morph/trim/stroke 的呈现几何尚未统一 |

## 建议交给实施模型的顺序

1. **先修 N01、N02、N03。** 这三处是确定的生产调用链漏接，能用小而集中的 lifecycle 回归锁住；不应等待架构重构。
2. **再修 N04、N07。** 把候选 retained identity、property ownership 与资源预算统一成提交计划，消除靠字符串、遍历次序推断生命周期的问题。保持现有 public API 是否调整由项目自行决定，无需兼容旧接口。
3. **修 N05、N06、N08。** 让一帧的采样结果、命中形状、终态投影和 terminal event 具有明确关联；保留已经有效的事务失败隔离。
4. **修 N09 并补稳态基准。** 正确性通过后锁定采样复杂度，再测布局/paint 合成后的帧时间。

测试应围绕交接矩阵，而不是只围绕新函数本身：普通更新 / remount / reload / resume_reload，live node / ghost / virtual item，Normal / Reduced / None，frame 前输入 / frame 后回调。这一轮多个底层测试通过而实际入口仍失败，说明最需要补的是这些交接测试。

额外保留的验证边界：真实 exit scene 外观冻结、嵌套 Host 中的 ghost 坐标、multi-view terminal callback 交付、所有 Gallery 效果的视觉质量，本轮没有给出充分端到端证据，不作为已验收能力，也不在缺少复现时额外扩张缺陷数量。
