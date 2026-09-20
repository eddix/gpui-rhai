# Motion Runtime 2：设计与实现审查

基线：`9f4ba04fc9c5198d7164359f47139a5d306de1bc`，2026-09-20，版本0.1.3 / Runtime API 2。对照前一提交`46384d83`，涉及127个文件、7,629行新增、1,623行删除。

**结论：设计方向值得保留，但当前实现不能按“Motion Runtime 2完整交付”验收。** 当前不是少量视觉参数需要调整，而是身份、采样、策略、提交和呈现路径尚未共享一致的契约。部分公开功能已可直接复现错误；官方Motion Gallery也无法实际挂载。

本次继续遵守维护者的策略：不把删除旧API或Runtime API 2的破坏性变化作为问题，不建议添加历史兼容层。没有修改产品代码。审查依据是实际源码、公开API探针和GPUI测试平台，不以代码量或清单完成数推导质量。

## 1. 验证与证据边界

| 检查 | 结果 |
| --- | --- |
| workspace/all-targets/all-features，locked/offline | 473 passed |
| 现有native-keyboard工作区 | 53 passed |
| fmt、严格全目标全特性Clippy | 通过 |
| 新增公开API特性化探针 | 复现下文时间线、句柄、采样、预算、ghost和虚拟项问题 |
| 新增GPUI测试平台用例 | 复现Host None下的layout motion，以及实际Gallery挂载错误 |
| exact commit的GitHub CI | 查询未返回运行记录；本次不据此宣称CI通过 |

原始日志：[workspace](evidence/workspace-tests.log)、[native](evidence/native-tests.log)、[Clippy](evidence/clippy.log)、[公开API探针](evidence/probes.log)、[GPUI特性化](evidence/native-policy.log)。[元数据](evidence/metadata.json)记录范围。

固定依赖为Rhai1.26.0、GPUI0.2.2、Rust1.94.0。GPUI `Window::request_animation_frame`与`on_next_frame`的实际实现已核验。没有重新执行截图比较、屏幕阅读器/IME、全平台GPU或受控120Hz帧认证。

**注意：新增native测试刻意断言“缺陷确实发生”，所以日志里的2 passed表示复现成功，不表示产品行为正确。** 正式修复应把这些用例改成期望正确行为的回归测试。

证据标记：R=实测；S=明确代码路径、未新做完整平台复现；G=设计或验收缺口。

## 2. 设计判断

以下方向合理：

- Rhai描述运动意图，Rust执行逐帧采样；不用逐帧脚本回调。
- 基础控件与可选`motion/*`源码包分开，允许应用和Rust Host使用同一底层能力。
- 明确限制普通GPUI子树的scale/rotate、通用blur、任意shader和3D；没有用命名掩盖GPUI能力限制。
- 将replay、暂停、物理源、减少动态效果、质量、预算纳入显式模型。
- 用paint transform承载layout motion，并让真实节点与exit视觉残影分离。

但“统一引擎”的核心尚未实现完整。现在至少有三套运动状态：

| 状态拥有者 | 保存的运动状态 | 缺少的统一约束 |
| --- | --- | --- |
| MotionRuntime | property source、timeline、active/settled、回放 | 句柄有效期、完整时间位置采样、所有命令的策略/预算检查 |
| GeometryRegistry | LayoutMotionState、TriggerMotionState、shared-layout历史 | 同一MotionPreference、预算、view暂停、统一属性所有权 |
| NodeSlotRuntime/renderer | 槽位时钟与采样快照、路径拼装 | Host RuntimeClock、统一节点身份、Canvas命中变换 |

再加上直接spring/inertia与timeline内部使用不同采样器，当前更接近“若干原生动效机制共用一组API名称”。下一步应先收敛这些契约，再扩充effect组件。

## 3. 主要发现

| ID | 优先级 | 证据 | 问题 |
| --- | --- | --- | --- |
| M01 | P1 | R | 官方Motion Gallery实际挂载失败，prepare测试漏检 |
| M02 | P1 | R | MotionHandle缺少实例/程序有效期，跨view控制未拒绝 |
| M03 | P1 | R/S | Host None可被restart绕过，layout/trigger路径绕过中央策略 |
| M04 | P1 | R | timeline pause/seek/play不构成一致的时间位置模型 |
| M05 | P1 | R | 物理采样、retarget及完成值不一致，合法输入可产生NaN |
| M06 | P1 | R | 属性单一所有者与timeline目标校验未落实 |
| M07 | P2 | R/S | play绕过active预算，公开Host协调失败会留下部分状态 |
| M08 | P1 | R/S | exit ghost读取错误几何域，身份与呈现清理也不完整 |
| M09 | P1 | R/S | 暂停view仍被共享tick推进，槽位另用系统时钟 |
| M10 | P1 | R/S | 虚拟项motion按index登记、按data key查找 |
| M11 | P2 | S/G | 帧采样调度和timeline回调没有真正的frame-commit边界 |
| M12 | P2 | S | Canvas运动变换没有在绘制与命中之间一致应用 |

### M01：Gallery的测试没有执行展示入口

位置：`examples/motion_gallery.rs:57, 92–96`。

Gallery入口使用`style().size_full()`，当前Rhai Style没有这个方法。复用了**实际示例MAIN字符串、全部motion源码、Tabs、默认主题和英文locale**，在GPUI测试平台执行prepare后mount，得到：

```text
Function not found: size_full (Style) (line 46, position 27)
```

当前`complete_motion_pack_prepares_together`仅执行`prepared()`和组件数量断言，未运行`view`，所以workspace全绿仍能交付不能启动的示例。

整改：用真实存在的Style API修复入口，并将Gallery测试至少推进到mount和一帧。随后给Tabs、重排、SharedLayoutCards添加受控状态和操作；当前它们使用常量value/items/selected，无法作为切换、重排和共享布局的验收应用。

### M02：MotionHandle重新引入了“同名即同一对象”的问题

位置：`motion.rs:377–399, 863–886, 1115–1119`；`context.rs:2581–2669`。

MotionHandle只有component path和name；没有timeline实例token、component incarnation、generation或runtime identity。`timeline_mut`仅按map key查询。UiContext控制命令也不核对handle是否属于当前view。

实测：取消一个node scope后创建同名timeline，旧handle等于新handle；用旧handle取消会让新timeline进入Cancelled。另一个view的UiContext也能取消该timeline：

```text
old_handle_reused=true new_state=Some(Cancelled)
foreign_view_handle_cancel_accepted=true
```

另外，`ctx.motion_handle(name)`查整个view，不能区分多个组件各自声明的同名局部timeline；文档宣称的component-scoped句柄与当前lookup范围不一致。

整改：区分逻辑名称与一次活timeline实例，绑定明确的view/component incarnation及程序迁移规则。连续rerender可保留实例，remove/remount必须新建，跨view必须在执行命令时校验。Rust Host若需要跨域管理，提供独立且明确的管理入口，不把它混入组件句柄。

### M03：减少动态效果策略只覆盖部分入口

位置：`motion.rs:654–694, 889–930, 941–959, 1010–1024`；`renderer.rs:1801–1821, 3618–3731`；`geometry.rs:156–226, 276–327`。

实测一：MotionRuntime使用Host `None`，创建的timeline已Completed；随后调用restart，它重新Playing，250ms采样为0.25且needs_frame=true。

实测二：真实GPUI视图使用`MotionPreference::None`，一个带layout_motion的节点从x=0移到x=100。布局已为100，视觉仍为0；推进Host手动时钟500ms并触发一次无状态变化的重绘后，视觉为50。它确实仍在执行1秒动画。

原因是GeometryRegistry自己的layout/trigger采样不接收中央preference；控制命令play/restart也不重新应用策略。MotionIntent虽然存在，但多数reduced fallback没有按decorative/feedback/essential执行ADR描述的分类。系统Reduce Motion自动桥接仍未实现，现有accessibility文档承认依赖Host/env，而ADR声称系统策略是默认上限，两者应统一。

整改：所有采样和控制路径共用一次有效策略决策，覆盖普通源、timeline、layout、hover/focus/press、scroll、span、ghost及effect primitive。None可以立即呈现目标状态，但不能继续推进中间帧。质量和策略能力声明也应被执行，而不只是传入PrimitiveTheme供调用者自行记住。

### M04：timeline缺少“任意时间点的完整呈现状态”

位置：`motion.rs:941–1004, 1229–1240, 1418–1465`。

`advance_timeline`只采样Playing；Paused/Idle/Cancelled直接返回空map，snapshot再用上一次tick留下的settled值补齐。结果是：

- 100ms tick后，在300ms暂停，值停在0.1而非0.3；seek到700ms仍是0.1。
- 在400ms对已经Playing的timeline再次play，值从0.4跳回0；虽然已有独立restart API，play也隐式重启了它。
- sequence在第二条轨道走到x=50后seek回0，尚未开始的后续轨道仍保留x=50，而非初始0。

这些不是单个插值参数问题。当前结果依赖之前调用过哪些tick，而不是明确的playback状态与时间位置。repeat/reverse、暂停拖动进度条和重启也会受到相同模型影响。

整改：编译时建立每个目标属性的初始/分段/终态；在任意合法时间位置都能得到完整快照。暂停控制时先采样到命令时刻，seek立即更新Paused/Idle呈现，play对Playing保持明确语义，不能用旧settled map填补未来轨道。

### M05：直接属性与timeline的物理、终态语义不同

位置：`motion.rs:801–825, 1364–1383, 1621–1722, 2221–2344`。

本轮复现了四类问题：

| 用例 | 实际结果 |
| --- | --- |
| 同一默认spring，from0→to1，一秒只采一次 | 直接源0.45；timeline约1.00005 |
| spring在运动中改目标 | 原速度2.88被重置为0 |
| opacity 0→1，2次autoreverse，到200ms结束 | tick最终值0；紧接snapshot变成1 |
| finite正参数 stiffness=1e308、mass=1e-308 | start接受，后续tick得到NaN |

直接spring使用单步半隐式Euler并将elapsed截断为50ms，timeline使用解析解；inertia同样有两套不同的采样语义。`tick`完成后把直接源写入`source.target()`，因此反向结束和惯性实际落点可能被另一套“目标计算”覆盖。App恰好是tick后马上snapshot，最终画出的不是tick报告的终值。

整改：确定一个规范采样器及明确的时间推进规则，直接源和timeline共用。若需要数值积分，用有界子步或经过验证的求解方式，明确大dt策略；retarget继承当前position/velocity，除非显式reset。完成时保存实际终态，保证tick与snapshot一致。输入有限不代表计算结果有限，还需物理参数域、派生系数和输出有限性保护。

完成记录还应保留源/回放签名，而不能仅通过`settled_value == source.target()`判断声明是否未变；否则仅修正autoreverse终值，会使下一次普通rerender再次启动同一动画。

验收应包括不同时步划分、临界/过阻尼、反复retarget、bounce/snap、偶数autoreverse、长时间间隔、极端但可接受参数；不能只跑60次小步并检查“最后接近目标”。

### M06：单一PropertySource还只是设计描述

位置：`node.rs:424–442`；`motion.rs:1496–1532, 2029–2110`；`renderer.rs:1746–1760`。

目前motion、signal、progress和timeline各自存放。只校验了部分motion/progress冲突和**同一timeline内部**的重叠；没有统一解析目标后的所有者检查。

实测：同一节点同时声明opacity motion和指向`.`的opacity timeline，协调成功。timeline引用不存在的child并对其声明rotate，也协调成功；既没有目标存在性检查，也没有按真实目标类型执行能力校验。多个timeline之间同属性覆盖同样不受单timeline的区间检查保护。

整改：在候选提交前，将相对target解析为当前scene内的真实NodeId，再按`(scene,node,property,time-range)`验证唯一来源及目标能力。literal/signal/motion/trigger之间的override若是合法设计，应有显式规则；否则拒绝。不能靠BTreeMap迭代顺序决定哪一条运动生效。

### M07：预算与原子性没有覆盖控制命令和公开Host路径

位置：`context.rs:2581–2669`；`lifecycle.rs:1039–1053`；`motion.rs:1814–1852`。

实测一：active_motions预算设为1，声明一个autoplay=false、包含2条轨道的timeline可成功mount；`ctx.play_motion`返回成功，active变成2，绕过预算。检查只在候选reconcile阶段执行，Idle→Playing并未重新接纳。

实测二：公开`reconcile_node_motion`先启动合法属性源，再处理一个非法重叠timeline；返回Err后仍留下1条active source。高层ScriptLifecycle有事务兜底，但公共Rust接口自己的“出错前不修改runtime”契约没有成立。

此外，layout/trigger状态位于GeometryRegistry，不进入MotionRuntime.active统计；shared历史缓存超过1,024时直接删最小key，未按Host预算拒绝。effect实例/cost检查发生在原生构建阶段，需要明确它与last-good UI提交的边界。

整改：先构建完整可验证plan，再原子提交；play/restart/seek等会增加活动成本的命令也经过同一接纳规则。区分声明数量、活动工作、历史快照与ghost持有成本；所有层的统计应与真实资源一致。

### M08：exit ghost当前在正常view路径不会建立

位置：`lifecycle.rs:1058–1120, 1578–1600`；`node.rs:1436–1469`；`app.rs:3248–3261`；`context.rs:364–367`。

exit收集从`runtime.geometry`读取旧节点几何，但正常视图将几何写入`presentation_geometry(view)`。实测向正确view域提交panel几何后移除它，exit motion活动数仍为0。

修正这一处后还要处理以下明确路径问题：

- 通过全树裸key集合判断移除，另一个父级下相同key会掩盖真正卸载；合法的局部key也可能被判为“几何不唯一”而跳过。
- ghost path带`ghost:`前缀，不属于普通`window:...`的motion scope；窗口关闭只移除ghost描述，可能留下对应active source。
- `render_snapshot`复制整个共享runtime的ghost列表，各view没有筛选归属。
- Custom/Overlay/VirtualCollection等返回None而静默跳过；普通节点保留的tab/语义属性也没有在renderer中统一执行“ghost不可交互”策略。它不是通用的已提交像素/绘制快照。

整改：以retained reconciliation的unmounted身份和presentation domain为入口；由明确的PaintGhost类型持有静态呈现数据，而非删掉UiNode几个字段后继续走普通节点渲染。暂不支持的快照类型应在声明/能力校验时说明或拒绝，不能接受exit后悄悄不执行。

### M09：共享runtime和槽位破坏了统一时钟与暂停

位置：`lifecycle.rs:527–554, 604–614`；`app.rs:4378–4380`；`motion.rs:1200–1225`；`slot_runtime.rs:38–41`。

view suspend没有立即冻结其MotionRuntime状态，只在resume时把started/last_tick向后平移。standalone窗口共享UiRuntimeState，另一个活窗口的poll会tick整个MotionRuntime。

实测：宽度0→100，在20暂停；模拟另一个窗口推进共享runtime200ms，再resume，值已从20变为100。已完成并移入settled的状态无法靠平移时间恢复。

与此同时，NodeSlotRuntime重新使用`Instant::now()`，而普通view renderer使用Host RuntimeClock。overlay/virtual/custom slot内的layout/trigger动画因此不再完全受手动时钟控制。

整改：暂停是一种持有在运行时内的scope状态，tick必须跳过/冻结该scope；不能只靠暂停某个poll task。传递同一frame sampling context进入所有slots，禁止在下游重新取墙钟。

### M10：虚拟项的注册路径与渲染路径不相同

位置：`motion.rs:2110–2114`；`virtual_list_element.rs`的`runtime.render(node, "item:{key}")`；`slot_runtime.rs:62–68`。

motion收集用`item:{index}`，渲染用`item:{data_key}`。对于key为alpha的首项，实测存储`root/item:0`，renderer所查的`root/item:alpha`不存在。采样器可以持续工作，但绘制取不到值。

整改：复用retained NodeId/统一路径解析，不分别拼接身份字符串；覆盖普通children、fragment、overlay、virtual item、span与custom slot。虚拟项重排必须跟随稳定数据身份，而不是当前位置。

### M11：没有真正的帧提交协调者

位置：`app.rs:2730–2754, 3803–3837, 4375–4438`；GPUI0.2.2 `window.rs:1644–1656`。

普通property/timeline靠16ms定时poll推进，layout/trigger又通过GPUI request_animation_frame推进。前者不能作为120Hz采样的证据，后者也不自动保证MotionRuntime在每个显示帧更新。

timeline事件在“下一次poll”交付，注释仅说上一帧“有机会提交”。任务/订阅的wake也会触发poll，两个poll可能发生于同一次真正绘制之前；这不是ADR要求的after-frame-commit保证。

事件还被全量drain，再在一个事务中依次调用；其中一条callback出错会跳过余下事件，而快照已不包含它们，重新引入先前异步批次丢失的问题。共享多窗口也没有按event所属view分发；unmount后旧component/generation的on_cancel接收者是否合法尚未定义清楚。

整改：以真实呈现域的frame epoch协调采样与提交，利用明确的提交回调/确认交付terminal events。终态通知应有独立失败策略与归属，不依赖“下一轮循环通常已经画完”。增加完成帧→callback顺序、A/失败B/C、多窗口、unmount/reload的端到端用例。

### M12：Canvas绘制和命中没有共用运动变换

位置：`renderer.rs:420–453, 2445–2495, 2728–2753`。

Canvas绘制中的node_canvas_point应用动态rotate/scale/skew；pointer payload却只减节点visual平移，然后直接对原CanvasScene做hit_test，没有反变换这组动态仿射参数。因此旋转/缩放后可见路径与canvas_key命中位置会分离。

另外，Rect只变换原点并缩放轴对齐宽高，Circle将非均匀scale取平均半径；这些不等价于公开2D仿射变换的几何结果。路径顶点与基本形状的行为并不一致。

整改：同一个采样后的仿射矩阵驱动paint、clip、hit testing和geometry投影；不能支持的形状变换应明确限制。本条基于绘制/命中控制流分析，本轮没有新做像素级Canvas认证。

## 4. 方案中尚未真正覆盖的能力

这些是设计与交付范围的差异，不应混成上面的已复现bug：

- ADR中的统一PropertySource包含literal/signal/derived来源，当前仍由多个并列容器和GeometryRegistry管理。
- RichText目前只支持span opacity；ADR更宽的offset/color/effect能力尚未落地。现有motion.md承认opacity范围，应让最终验收契约一致。
- 系统策略自动读取、按intent的crossfade fallback、direct-manipulation layout抑制没有完整接通；仅有字段或入口不代表已经实现策略。
- SharedLayout目前主要是位置历史插值，尺寸直接snap；这是GPUI限制下可以接受的明确子集，但不能据此宣称完整共享视觉快照转换已经认证。
- NumberTicker当前是数值文本重放fade，Particles是固定点集整体旋转；可以作为简单效果，但没有验证通用数值插值/独立粒子状态机制。
- Gallery没有timeline控制、enter/exit交互、连续retarget、可操作重排/shared切换；当前prepare-only测试不是公共能力的完整性证明。

建议保持一个“承诺→入口→状态拥有者→采样器→渲染消费者→回归用例”的能力矩阵；没有最后两列的功能不标记为完成。

## 5. 推荐的收敛方案与实施顺序

1. **先建立真实验收入口。** 修Gallery启动，并让示例具有时间线控制、状态切换和重排操作；将本次探针转成正确行为断言。
2. **统一身份与编译计划。** `scene + retained NodeId + property + motion instance`区分定位、所有权与存活；解析全部timeline targets，在同一plan中校验冲突、能力和预算。避免把身份继续编码成字符串路径。
3. **统一采样与playback状态。** 一个TimePosition/PlaybackState模型产生完整快照，直接源和timeline使用同一规范插值/物理求解；retarget与replay明确分开。
4. **统一SamplingContext。** 同一个Host now、有效preference/intent/quality、scope暂停和提交epoch传至普通节点、geometry驱动、slot、Canvas和ghost。可以模块化实现，不必把所有代码塞入一个大对象。
5. **接通呈现生命周期。** ghost使用真实卸载身份和受支持的静态绘制数据；terminal事件按view与提交epoch交付；绘制与命中共用transform。
6. **最后认证性能与效果包。** 测量真实GPUI帧中的Rhai调用数、采样时间、分配、active/ghost/shared持有量及layout/paint成本，再做受控120Hz测试。禁止用“采样循环无Rhai”代替“整帧达标”。

无需恢复旧动画API。当前更值得投入的是让这套新模型真正统一，而不是继续增加新的动效名称或配置字段。

## 6. 复跑

参见[复跑说明](reproduce.md)。公开API探针为[probes.rs](probes.rs)；GPUI测试平台用例为[native-policy.rs](native-policy.rs)。原始审计数据不回写到旧版本验收结论。
