# 0.1.8 第五轮独立验收

日期：2026-10-02。已合入基线：`23fce23829c6694c3e077a61a20a9038b87f0759`（PR #94），对比第四轮审计的 `f6e936a5`。

**结论：第四轮原反例已关闭，#93 可以确认解决；当前 main 仍不建议发布。新增确认 1 个 P1、4 个 P2，另有 1 个非阻断 P3 数值边界。** 发布阻断的核心是：合法的直接返回子组件组合，在虚拟列表滚动后仍会丢失子组件状态。其余问题分别位于精确读取依赖、嵌套虚拟集合、内部遮挡延续滚动和 PanZoom 的实际绘制几何。

开放 PR #100/#101 的评审单独列在后文，不计入 main 的缺陷数量。#96/#97/#99 仍为上次审阅的旧 head，本轮不能把它们说成已整改或已验收。仅新增本轮报告和探针，没有修改产品、正式测试、历史审计，未合并或评论 GitHub PR。

## 一、已通过的整改

独立复跑前四轮 **89/89 GPUI 探针**、**11/11 异步探针**。第四轮 Canvas CPU paint recorder 同样通过；带 padding/border 的旋转中心保持 `(131,51)`。schema 接受域 **12,870 组差异为0**。

异步11项包含原6项及第四轮5项；第四轮旧缺陷特征测试通过既有验收runner，在临时副本中把“旧消息应报错”改成正确的“已取消消息正常丢弃”，显式调用旧callback仍应报错的对照保留。没有修改旧探针源。运行命令、源SHA和结果见 [run-regressions.py](run-regressions.py)、[baseline/results.json](baseline/results.json)。

本轮额外确认：

| 整改点 | 验收结果 |
| --- | --- |
| 直接正式 Counter 的完整虚拟目标提交 | `[0,10]` 保留row0的state=7/callback；纯裁剪到`[0]`清掉row1的state并拒绝旧callback |
| 延迟新行的真实callback owner | 原Action13等原生反例通过，首屏与真正未seed行的转发一致 |
| raw row直接读取NativeCollection | missing→register与existing→replace都会刷新，与普通root对照一致 |
| Canvas drawable / inverse | 原反例通过；新增非对称padding/border、负非均匀缩放、30°旋转、DPI1.25的点击对照也通过 |
| wheel完成、取消及generation | 原Escape反例通过；旧timeout不会释放新会话owner，pending debounce暂停恢复后无残留拦截 |
| 活动旋转的生命周期lease | 两轮suspend/resume/resize恢复source，零迟到提交，无Entity重入错误 |
| 失效effect交付 | 同路径重挂载后旧消息丢弃、新activation有效；有效scope配错误owner仍拒绝，未放宽身份验证 |
| #93 默认值诊断 | root/formal及嵌套Map/Array指出所属state字段和递归编码要求；合法编码保持不变，失败候选不污染last-good状态/视图/generation |

直接集合旧反例的本轮输出见 [r4-acceptance.log](collections-tree/r4-acceptance.log)。#93和异步新增固定源码快照探针 **4/4**，见 [rhai-async报告](rhai-async/report.zh-CN.md)。

同一合入提交 `23fce238` 的远端CI为SUCCESS，见 [CI快照](evidence/ci-94.json)。这是远端证据；本轮没有把作者的完整workspace、134项产品原生测试、package/release smoke或视觉检查冒称为自己重新执行的结果。以下新反例解释了为何现有绿灯仍不足以批准出货。

## 二、main 的新发现

### R5-1 · P1：直接返回子组件的Wrapper仍被错误裁掉内部组件生命周期

定位：`engine.rs:2675–2682` 的 `retain_virtual_component_manifest`；`node.rs:511` 的 `ComponentSubtreeIndex::contains`；正式组件返回值在 `engine.rs:2539` 加根标记。

新的保留集合从“上一帧仍可见的正式组件根节点”推导存活组件。但一个UiNode只保存一个component_root：Wrapper直接返回Counter的节点时，外层根标记覆盖内层根标记；Counter仍有独立invocation/state，却不再出现在可见根索引中。

相同Counter、自身state=7、自身callback读取state，只改变组合形态：

| 行renderer输出 | 滑窗目标从seed改为`[0,10]`，row0仍保留 | 结果 |
| --- | --- | --- |
| 直接Counter | state=7，callback返回7 | 正对照通过 |
| Wrapper直接return Counter | **Counter state=None，callback报StaleComponentCallback** | 失败 |
| Wrapper返回`column([Counter])` | state=7，callback返回7 | 正对照通过 |

这是一种正常的组件封装方式，不应要求用户为了保住state额外增加视觉容器。上一轮的直接Counter反例确实修复了；本项证明组件生命周期与视觉根节点仍被混为同一个集合。

整改要求：从已提交的组件调用/所有权关系保留完整生命周期，支持多个组件共享同一可见根节点。保留行的所有合法后代invocations、incarnation、state和资源必须一起保留；退出目标的子树一起清理。不要只在本测试里补一个Wrapper路径，也不要全量保留scope以牺牲卸载正确性。

验收至少包含直接返回、嵌套多层Wrapper、带容器、条件改变组件类型、保留/新增/纯移除、失败回滚；state/callback与effect/timer/signal/ref分别验证。本次直接动态证明的是state与callback，不能把共享路径上的所有资源都记为已实测。

证据：[collections-tree报告](collections-tree/report.zh-CN.md)、[probe.log](collections-tree/probe.log)。

### R5-2 · P2：store路径级读取仍登记到错误owner，虚拟行保持旧值

定位：`context.rs:1811–1823` 的 `get_app_store_path`，`:1913–1929` 的 `get_window_store_path`。

本轮把whole-field读取改为callback_component，但两个path读取入口仍使用self.component。raw virtual row中后者是synthetic结构scope，其reader在生命周期提交中被清除；Host更新实际store成功，却没有可调度读者。

实测app和window两个store变体：whole-field读取在更新后 `render_dirty=Ok(true)`，显示before→after；同样数据改用精确path读取，`render_dirty=Ok(false)`，仍显示before。local `get_state_path` 则通过：它委托给已修复的get_state，本轮不将它误列为遗漏。

整改要求：明确结构身份、callback执行owner与可重绘read owner的职责，所有observable读取入口都从统一的owner规则登记依赖。保留path精确失效，不通过改为整字段订阅掩盖问题。补app/window × whole/path × 普通/虚拟行的对照，并检查卸载和失败回滚。

证据同上。它与上一轮NativeCollection读取是相邻入口遗漏，但不是原集合反例未修复。

### R5-3 · P2：合法嵌套虚拟列表在内层realization时查不到自己

定位：`node.rs:1228–1248` 的 `virtual_collection_spec` 和 `:1263–1292` 的替换遍历；调用方 `lifecycle.rs:444–448`。

外层virtual_collection的一行返回内层virtual_collection；初始构造接受并显示seed。请求内层row10后，`realize_virtual_requests` 返回 `MissingVirtualCollection(inner-a)`。完全相同的内层列表作为root时，row10请求成功。

查找/替换遍历在遇到不匹配的VirtualCollection时没有继续进入其realized子节点，因此无法定位嵌套集合。这是本轮确认的现存组合缺陷，未宣称由当前commit新引入；公开构造没有拒绝这种组合。

整改要求：集合定位、替换、snapshot更新与资源保留使用同一套完整树遍历，覆盖virtual realized内容和允许承载节点的容器。修复lookup之后还要验证外层保留/裁剪时内层recipe、正式子组件资源是否同步处理；后一部分目前是必验边界，不另算已实证的新bug。避免只修改一个lookup而留下update路径不一致。

证据：[collections-tree报告](collections-tree/report.zh-CN.md)。

### R5-4 · P2：列表内部明确遮挡后，row-gap延续滚动仍继续

定位：`virtual_list_element.rs:107–109` 的viewport hitbox插入顺序；`interaction.rs:633–646` 的continuation判断。

先经过接受card的Lane1建立合法目的地，再进入列表底部的内部子DropZone；该子节点明确`.occlude()`且只接受other。指针静止，推进12次16ms native tick，列表仍从 `(item0,offset0)` 滚到 `(item2,offset36)`。

对照：真正空隙应滚动，实际通过；直接进入同一拒绝节点而不先经过接受行时，不滚动且drop rejected；取消旧拖动后，新拖动也不继承旧destination。

viewport Normal hitbox在整个行子树prepaint之后插入，查询它的hover状态无法证明未被自己的occluding子节点遮挡。第四轮外部兄弟覆盖层已修，这次是新continuation路径里的内部遮挡。

整改要求：保留真实ancestry和已验证容器身份，同时用当前帧正确层级的命中判断区分“空隙”和“拒绝/遮挡区域”；target=None不等于开放空隙。不能为修命中顺序又使用上帧viewport或退回矩形相交推断。

证据：[drag-sort报告](drag-sort/report.zh-CN.md)，新增3项原生测试为2通过、1失败。

### R5-5 · P2：PanZoom仍用外框计算锚点，未消费新的Canvas drawable

定位：`pan_zoom.rs:233–256`，`registry/components/pan_zoom.rhai:67–95`。

指针放在实际绘制marker中心，触发2×缩放；比较相同组件的两种content padding：

| 左/右/上/下padding | 缩放前marker中心 | 缩放后实际中心 | 结果 |
| --- | --- | --- | --- |
| 20/20/20/20 | `(106,66)` | `(106,66)` | 正对照通过 |
| 40/0/20/0 | `(126,66)` | `(106,56)` | **漂移(-20,-10)** |

证据直接来自实际GPUI CPU Scene path，既不是只比较signal，也不是极端数值。Rotatable/SelectionArea已消费drawable，PanZoom仍从外层viewport的origin/size计算锚点；非对称样式使两者中心不同。

整改要求：事件命中区域可以保持外层viewport，但wheel/keyboard缩放数学应使用与paint相同的drawable及变换。复用已有geometry能力，不在组件里另猜padding。补已有pan、content尺寸覆盖、缩放夹取及实际paint不漂移断言。

证据：[transforms-selection报告](transforms-selection/report.zh-CN.md)、[paint-geometry.log](transforms-selection/paint-geometry.log)。

### R5-6 · P3／非阻断：极小缩放的inverse能力边界

`geometry.rs:208–216` 以绝对 `abs(det)<=f64::EPSILON` 拒绝inverse。两个有限值场景经forward affine后目标均为5×5px：scale1e-7可点击，scale1e-9的inverse为None，无法点击。

这是远超默认PanZoom0.25..8范围的自定义数值边界，**不独立阻断0.1.8**，也不把它与常规padding缺陷等量处理。后续选择相对条件/归一化算法，或明确统一支持下界。该探针使用Canvas Motion，不宣称已完成所有超小scale的实际GPU光栅化验证。

## 三、新 PR 与上次待办

### #100：CI绿色，但只开放policy不能关闭嵌入Host窗口

评审head `23d009ab`，[PR #100](https://github.com/eddix/gpui-rhai/pull/100)。仅把构造器改为pub及补文档，未合入main。

main公开API对照把逻辑window policy设为同样的ApplicationOwned后，close/focus仍报 `native window embedded-window is unavailable`，真实窗口保留；默认Disabled按设计拒绝。源码确认embedded mount建立空NativeWindowRegistry，standalone/secondary路径才登记真实GPUI WindowHandle。

这是对policy与native注册的隔离测试，**不是PR head端到端构建，也不是main当前默认行为的新bug**。公开pub本身并非unsafe；问题是PR没有完成其说明中的embedded self-close诉求，还缺原生注册、关闭拦截和Host/View所有权连接。

建议：**不原样合入解决该诉求；由主开发接管有限的窗口ownership方案。** 如果当前只需要q关窗，Host action可完成，不必为0.1.8临时开放全部命令。作者可保留入口贡献、修正文档错误并补外部crate集成测试；如果只发布纯策略入口，应明确收窄说明，不能宣称self-close已解决。

详见 [runtime-core报告](runtime-core/report.zh-CN.md)。

### #101：修复方向成立，作者补RTL后建议进入0.1.8

评审head `f9203250`，[PR #101](https://github.com/eddix/gpui-rhai/pull/101)。固定PR源码快照的6项原生测试：4通过、2失败，同属RTL根因。

通过：LTR Array边界拖动、末列拖动、单列Table、NativeCollection末列autofit、不可resize相邻列的owner和末列键盘resize。Native末列autofit为c:313，未误排序，header与body cell宽度一致。

失败：RTL下使用物理left/right摆放手柄。A的实际逻辑边界在x479，A手柄中心却在x339；B也错位到下一列。真实在A边界拖动40px没有触发resize，反而触发B排序，结果为 `none|b`。

建议：**作者小改逻辑方向边界并补Array/NativeCollection的LTR/RTL验收后合入0.1.8**，不需要重写native resize。保留无LocaleManager场景，不能修RTL时引入新的初始化前提。更新到最终主干重跑CI、补受影响视觉基线。当前CI取消停在Linux依赖安装阶段，不能当作代码测试失败，也不能当作已通过。

详见 [resize-controls报告](resize-controls/report.zh-CN.md)。

### 上次PR/issue状态

- #93已关闭，本轮独立验证支持该关闭结论。
- #96/#97/#99的head仍分别为befbe926/31df8387/3209039e，上次分流意见仍有效；此次main整改没有交付这些PR。
- 其余字体、资产、grip、context request和调用来源等独立增强不混入本轮main缺陷统计；排期沿用[上一份分流报告](../2026-10-01-dogfooding-pr-triage/report.zh-CN.md)。

## 四、下一轮应如何收口

1. **以组件调用关系确定生命周期。** 完整目标集合已正确建立，剩下要消除“每个组件必须拥有独立可见根节点”的假设。先修R5-1，再做嵌套组合与卸载/失败回滚的对照。
2. **统一读取owner与树遍历入口。** 把whole/path依赖登记作为同一契约验证；不要逐个改调用点后便假设所有ctx读取已统一。集合定位/替换/资源保留也应覆盖同一树结构。
3. **完成共享geometry的消费者迁移。** paint、选择和旋转统一之后，PanZoom也应使用相同事实；对称样式很容易掩盖偏差，必须保留非对称测试。
4. **保留正向能力。** 修内部遮挡不能破坏真实row-gap静止滚动；修生命周期不能让每次滚动重跑所有旧行Rhai或重复启动effect。

当前无需推翻整套运行时。已通过的预算、身份、异步activation、取消代际与Canvas测量修复应保留；本轮阻断说明组件组合的所有权模型仍需一次完整收口。失败正确性断言应由产品修改转绿，不能改测试期望接受丢状态或错误坐标。

## 五、证据边界

- 基线与PR/CI元数据在 [evidence](evidence)，历史复跑只写本轮baseline，不覆盖旧报告。
- 一些集合程序采用现状特征输出并以0退出；报告中的期望/实际才是判据，不能把进程成功当作产品正确。
- PR100失败与PR101失败独立于main结果；极小scale的P3也单列，避免混淆发布优先级。
- 本轮没有新截图美学评估、真实设备触控板穷举或完整GPU精度证明；CPU Scene记录用于实际绘制路径几何。
- 最终发布门槛仍应在修复后的同一commit执行项目既有完整验证。本轮确认的P1未关闭前，不批准出货。
