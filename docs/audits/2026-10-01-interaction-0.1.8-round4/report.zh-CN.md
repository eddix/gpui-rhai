# 0.1.8 第四轮对抗式审计

日期：2026-10-01。审计提交：`f6e936a509d9aaf75f2e28836bedd439fe83887e`，对比上一轮 `815bb82b8103c88ef6ba50620128780c9d64b216`。

**结论：第三轮原反例已修复，但当前提交仍未达到出货要求。本轮确认 3 个 P1、5 个 P2。** 本轮最重要的阻断集中在虚拟列表增量生命周期，以及 Canvas 实际绘制区域与交互几何的分叉。应先关闭这些公共模型的缺口，再验收其所有入口。

本轮沿用现行 ADR 0022；不要求历史兼容，不重新引入已撤销的 awaiting-control 设计，也不把明确推迟的功能缺失列为 bug。本文中的“新发现”表示此次审计确认，不表示都由当前 commit 新引入；未在旧版本运行同构对照的发现，不宣称为新增回归。

## 已确认的整改与验证

- 前三轮 **69/69 GPUI 探针、6/6 异步探针通过**；历史测试源与断言未更改，source SHA 记录在 [baseline/results.json](baseline/results.json)。69 项中包含历史身份对齐的诊断对照，不能将它另算成一个产品覆盖面。
- 新增 RangeSlider/Table **6/6**：横/纵 End 到达特殊端点 97；无 callback 与 noop callback 拒绝后真实 thumb 绘制位置恢复；已有本地宽度的 Table 在手势中替换 source、收紧约束，旧 preview 不覆盖新契约。
- effect 的旧 activation success/error 被丢弃；正常 producer 返回、同 activation 重绘、失败替换/暂停的补偿、成功恢复后的新交付通过。
- 在线/离线 schema 接受结果 **12,870 组差异为 0**；普通 map/array/object 路径的全量错误构造问题已修复。
- missing collection 依赖的 event 不跟踪、名称校验、64 个名称/reader、4096 个 pair、64KiB 名称预算、拒绝不变更、回滚与精确唤醒通过。
- Tree 的禁用祖先索引与输入顺序对照通过，原深链明显退化已有改善，细节见子报告。
- 同一 App 下不同窗口的 Escape 隔离、空闲 Escape 透传、View dispose 后解绑通过。全局 interceptor 本身不应被笼统判为错误。

[PR #94](https://github.com/eddix/gpui-rhai/pull/94) 的 head 与审计提交一致，远端 `portable` CI 为 SUCCESS；状态快照见 [evidence/pr-94.json](evidence/pr-94.json)。本轮独立运行的是上述探针及后文的新反例，未把实现方所列完整 package、release smoke、视觉或真实设备检查冒称为自己重跑的结果。CI 绿灯没有覆盖后文的反例。

## 1. P1：虚拟列表把“这次新增的行”误当成“整个作用域仍存活的组件”

位置：[`lifecycle.rs:450`](../../../crates/gpui-rhai/src/lifecycle.rs)、[`engine.rs:1066`](../../../crates/gpui-rhai/src/engine.rs)。精确行号以本节文本为准，Markdown 源码链接打开文件。

`realize_virtual_requests` 保留目标索引中的旧 UiNode，只将 `missing` 交给 engine。engine 却以整个 `VirtualCollection` 为 render root，在 `finish_component_render` 用本次 `seen/invocations/effects/timers/signals/refs` 替换整个作用域。UiNode 仍保留和组件仍存活由两套不同集合决定。

两方向的公开 API 独立证据：

- 初始正式 Counter 行 state=7，自己的 `read_count` callback 返回 7。目标变为 `[0,10]`，保留 row0、增加 row10；row0 仍在 realized 集合，state 却变为 None，原 self callback 返回 `StaleComponentCallback`。
- 另一实例仅裁剪到 `[0]`，没有 missing。row1 已从 realized 集合消失，但 state 仍为 7，保存的 row1 self callback 仍可返回 7。

这不依赖父级 callback 转发，直接证明生命周期集合错误。原生 DropZone 进一步复现：滚动前 Lane1 可拖入；滚动 45px 后，仍可见 Lane2 的 `drop_zone_commit` 报 unmounted incarnation；新显示行亦有相同症状。应合并为一个生命周期根因，不能按三个 callback 报三条 bug。

影响：正常滚动可让可见交互失效，纯裁剪则不能及时卸载组件；同一范围内 effects/timers 等 metadata 也使用替换逻辑，修复必须一起审查，不能只恢复 state 或放宽 callback 身份验证。

整改要求：一次虚拟化提交显式持有完整目标集合 T，按旧集合 O 计算保留 `O∩T`、新增 `T−O`、删除 `O−T`。只执行新增行脚本可以保留，但提交必须基于完整 T：保留行的 incarnation/state/依赖/资源不能被删除，移除行必须清理。纯裁剪同样进入生命周期提交；失败回滚同时覆盖 retained 节点和组件资源。

证据：[collections-tree/report.zh-CN.md](collections-tree/report.zh-CN.md)、[probe.log](collections-tree/probe.log)、[原生 drag-sort 日志](drag-sort/probe-results.log)。

## 2. P1：滚动后真正新建的行把父级回调重新绑定到了 synthetic 作用域

位置：`engine.rs:1503–1506`，对照首次构造的 `:3335–3339`；`context.rs:1148–1152` 与 `:1174–1183`。

初次构造通过 `for_structural_scope` 保留真实 callback owner；延迟 `realize_virtual_collection` 却重新调用 `for_component`，覆盖已保留的 owner。两条路径仍不是同一语义。

隔离掉前一条“正式行的 self callback 被卸载”的问题：正式 Action 只把 root 的 `on_action` 直接转发至 `on_click`，root `clicked` 读取并累加自己的 count。初始 Action1 点击成功，count=1。探针读取实际 `VirtualCollection.spec.realized`，确认 seed 为 `{0,1,2,3,4,5,6}`，再滚动并选取不在 seed 中的 Action13。

点击 Action13 后，callback 确实进入 `clicked`，却拿到 `/View[audit-drag-sort]/VirtualCollection[actions]` 的 ctx，报这个 component 没有 count 字段；root count 仍为1。它不同于前一项的正式行 self callback stale，因此需要独立修复。早期 Action5 对照处于 seed 范围内，已被更严格的此对照取代，不作为通过证据。

整改要求：seed 与 delayed realization 共用结构作用域构造及真实 callback owner 传播规则，保留真实 component incarnation/events/native context。不能仅在最终 UiNode 上补 bind，因为 callback 可能在正式组件 props 解码/转发时已绑定。修复本项后仍必须验证前一项完整生命周期集合，反之亦然。

验收覆盖 root callback 和 formal parent callback，实际未 seed 的行、滚动回来重新创建的行、命名空间相同但 incarnation 已变化的情况；断言 callback 的读写确实落到原 owner，不只断言没有错误。

证据：[drag-sort/report.zh-CN.md](drag-sort/report.zh-CN.md)、[probe-results.log](drag-sort/probe-results.log)。

## 3. P1：Canvas 外框几何仍被当成实际绘制区域，样式改变后旋转和命中错位

位置：`renderer.rs:2004` 的外层 geometry tracking、`renderer.rs:2989` 的 `render_canvas`、`rotatable.rs:250`/`:408` 的 viewport 读取与 pivot 补偿；SelectionArea 的 content ref 命中也受同一分叉影响。

第三轮改为读取 content ref，解决了“整个控件尺寸代替 Canvas 尺寸”的旧反例。但 styled UiNode 外框与其内部 GPUI Canvas 的 drawable bounds 仍不是同一个区域；padding/border 会改变内层原点和大小。

直接记录 GPUI CPU paint Scene 中 pivot marker 的 path，而非只比较理论公式或 props：

| Canvas 样式 | angle=0° 的实际中心 | angle=90° 的实际中心 |
| --- | --- | --- |
| 无 padding/border 对照 | `(101,21)` | `(101,21)` |
| padding20、border10 | `(131,51)` | `(71,51)` |

marker 以自身中心作为 pivot，旋转后却移动 **60px**。另一原生 SelectionArea 探针点击实际绘制目标中心，返回 `[]`，期望 `["a"]`。

整改要求：geometry 层明确区分 layout/border bounds 与 Canvas drawable/content bounds，让绘制、pivot、window↔content 坐标变换、点击和 marquee 使用同一份实际测量数据及 revision。不要分别在 Rotatable/SelectionArea 再各减一遍 padding；否则边框、非对称 padding、主题切换、布局变更会继续分叉。也不应通过禁止已有 part style 或忽略主题配置掩盖问题。

验收必须读取实际 paint/hit 结果，包含无装饰、padding、border、非对称内边距、布局/主题切换和手势中 geometry revision 改变。

证据：[transforms-selection/report.zh-CN.md](transforms-selection/report.zh-CN.md)、[paint-geometry.log](transforms-selection/paint-geometry.log)、[native-probes.log](transforms-selection/native-probes.log)。

## 4. P2：虚拟行内读取 NativeCollection 没有可执行的依赖所有者

位置：`context.rs:1174` 的 structural scope、`:1758` 的 reader 登记，以及虚拟化 render/资源保留边界。

单行 renderer 中 `ctx.get_native_collection("late")`，未注册时显示 missing。Host 注册一条数据后：普通 view 的相同函数 `render_dirty=true`，显示 ready:1；放入 `virtual_collection` 后 `render_dirty=false`，仍显示 missing。强制全量 render 后能显示 ready:1，再将 collection 替换为两条数据：普通 view 刷成 ready:2，虚拟行仍停留 ready:1。

已修复的 missing reader 预算本身通过。这里的问题是 synthetic 结构路径与真实可重绘 owner 未对齐；只保留 callback owner 并不能保证 render 依赖有归属。

整改要求：每条读取依赖必须映射到可调度的真实 owner，或定义可独立重新执行的行 render recipe，并参与完整目标集合的保留/清理。补齐 missing→register、已存在→replace、保留行及移除行四类验收。

范围限定：本轮实证是 **row renderer 内直接 ctx 读取另一 NativeCollection**，不泛称虚拟列表 `config.data` 的所有更新都失效。证据见 [collections-tree 报告](collections-tree/report.zh-CN.md)。

## 5. P2：虚拟自动滚动用矩形相交猜祖先，能滚动前景后面的无关列表

位置：`interaction.rs:1089–1125` 的 `resolve_virtual_scroll_target`。

同一 View 中，虚拟列表上覆盖一个与它无父子关系、显式 `.occlude()` 的 DropZone。在前景下缘拖动：

- 前景接受：原生 drop 正确落到 cover，但背后列表从 item0 滚到 **29**。
- 前景拒绝该 payload：原生 drop 正确 rejected，背后列表仍滚到 **29**。

drop 的正向对照证明遮挡确实有效，问题在滚动路径独立选择目标：same view + bounds 相交不等于祖先关系；fallback 还会扫描没有通过真实 hover/遮挡判定的接受目标。

整改要求：从实际命中的 accepting destination 沿 retained/注册树的真实 scroll ancestry 向上解析，注册信息带明确容器身份；同时遵守遮挡和接受规则。需要 empty-list 例外时也应显式表达可接受的容器目标，不能退回“同 View 任意相交 target”。

证据：[drag-sort/report.zh-CN.md](drag-sort/report.zh-CN.md)。

## 6. P2：marquee 的新容差仍依赖绝对坐标，缩放后可见选区被误判退化

位置：`selection_area.rs:517–530`。

1000×1000 Canvas，相同 **5×5px 可见目标**和 **20×20px enclose 选区**：scale=10,000 选中 a，scale=1,000,000 返回空。参数在当前支持范围内；不是要求处理无穷缩放。

新的 `EPSILON × max(abs(coordinate))²` 门槛随坐标原点放大。窗口中的选区已经判为有面积，转到局部约499的坐标后，面积判定又将其拒绝；鞋带公式的绝对坐标乘积也存在相消误差。

整改要求：几何谓词保持平移不变性，先平移到局部锚点再计算面积/叉积，以边长与计算误差确定容差，并与窗口像素退化规则统一。增加“相同屏幕形状，改变内容偏移/zoom/Canvas大小”的变形测试，不再仅调一个阈值。

证据：[transforms-selection 报告](transforms-selection/report.zh-CN.md)。

## 7. P2：PanZoom debounce 已完成，却残留 Escape 取消所有权

位置：`pan_zoom.rs:288–303`；对照显式结束 `:310` 与清理 `:645`。

precise wheel 无显式 Ended，80ms debounce 完成后，无 callback 的受控 scale 已由 1.10517 恢复为 1.0。此时第一次 Escape 被 Host interceptor 消费，第二次才到应用。空闲和显式 Ended 对照第一次均到应用。

原因：timeout 清除 pending、恢复 preview/发送 proposal，却没有注销 auxiliary registration。业务成功重绘可能顺带清理，掩盖无观察者或拒绝更新路径。

整改要求：即时、显式 Ended、debounce 三个完成入口共用终止逻辑；先校验 session generation，再清除当前 owner，避免旧 timeout 清掉新会话。清理不依赖业务是否接受提议。

证据：[runtime-core 报告](runtime-core/report.zh-CN.md)。

## 8. P2：已取消 effect 的消息在组件卸载后仍变成应用错误

位置：`lifecycle.rs:785–793`。

真实 subscription 消息已经 drain，之后组件正常卸载，producer 已 `ScopeDisposed`。交付这条消息却先校验 callback owner，返回 `StaleComponentCallback`，到不了新增的 activation 过滤分支。

实测到 lifecycle 错误；App 在 `app.rs:5093–5115` 将其汇总为 ScriptFailure，再调用 set_failure，是源码推导，未额外声称实际错误横幅已原生测量。它不会执行旧 callback，却把正常取消误报为故障。

整改要求：异步路径先判断 effect activation 是否仍有交付资格，再对有效消息严格验证 callback owner/generation。保留“同步显式调用已卸载 callback 应报错”的正对照，不能全局吞掉 stale callback 错误。

证据：[rhai-async 报告](rhai-async/report.zh-CN.md)。

## 实施次序与核心模型

建议按三个改动单元实施，分别补契约测试：

1. **虚拟化的完整目标集合和所有者模型**：节点保留、正式组件 incarnation/state、effects/timers/signals/refs、render 依赖必须同时提交；callback 转发的 owner 与结构命名空间明确分工。覆盖新增、保留、仅删除、重排、source replace、失败回滚。本轮三类虚拟化发现应在同一轮验证，避免只修函数调用点。
2. **Canvas 的实际内容几何**：单一测量来源提供 paint/hit/pivot 共用的变换和 revision；用原生绘制证据与平移/缩放不变性检验，而非复制实现公式。
3. **会话结束与交付资格**：明确成功结束、取消、替换、卸载都释放什么；正常取消的消息与非法主动调用分别处理；滚动归属使用真实树关系。

当前仍值得保留的通用修复包括 canonical retained identity、每项 async commit 的失效事实、effect activation 验证、missing reader 预算和 Tree parent-first 索引。无需推翻整套 Interaction Runtime；需要让上述尚未统一的边界真正共享模型。

## 当前 GitHub issue 的安排

本轮只读拉取，未发评论或修改 issue。完整快照见 [evidence/open-issues.json](evidence/open-issues.json)。

| Issue | 当前判断 | 本轮建议 |
| --- | --- | --- |
| [#93](https://github.com/eddix/gpui-rhai/issues/93) 非空 map 默认值/诊断 | 当前提交仍复现；递归 typed values 对照通过，错误仍不指认字段 | 已知 P2，建议本轮补文档示例和带路径诊断，不必支持两种历史语法；不计入本轮新发现 |
| [#95](https://github.com/eddix/gpui-rhai/issues/95) 主题 monospace/列字体 | 主题排版 API 的独立功能设计 | 后续独立实现；应统一 typography role、Table 两种数据源与主题切换，避免新硬编码字体；不阻断当前交互整改 |
| [#83](https://github.com/eddix/gpui-rhai/issues/83) SplitPane 自定义 grip | 既定后续设计 | 维持后续安排 |
| [#89](https://github.com/eddix/gpui-rhai/issues/89) Table context request | 既定后续设计 | 维持后续安排 |
| [#91](https://github.com/eddix/gpui-rhai/issues/91) capability invocation origin | 既定后续设计 | 维持后续安排，不用缺少新 API 推导权限漏洞 |

## 证据与复验方式

| 本轮新增套件 | 运行结果 | 判读 |
| --- | --- | --- |
| runtime-core 原生 | 4 pass / 1 fail | debounce Escape 失败，其余隔离/解绑对照通过 |
| resize-controls 原生 | 6 pass / 0 fail | 本组没有确认新缺陷 |
| drag-sort 原生 | 0 pass / 6 fail | 6 个正确行为断言失败，分别归并到 3 个根因；初始回调、真实未 seed 行、遮挡接受/拒绝前提均通过 |
| transforms-selection 原生 | 1 pass / 2 fail | 低 zoom 对照通过，padding 命中和高 zoom 失败 |
| Canvas CPU Scene recorder | 预期断言失败 | 无装饰 pivot 固定；带装饰 pivot 实际漂移60px |
| rhai-async | 5 项程序断言通过 | 4 个正向测试，1 个记录旧消息被误报的缺陷特征测试 |
| schema | 2 项程序断言通过 | 12,870 组接受域一致性；另1项记录已知 #93 契约/诊断问题 |
| collections-tree | 程序完成，含已确认失败行为输出 | 配额/事务对照通过；虚拟化生命周期和依赖反例见实际输出；另有192组 Tree oracle 通过 |

- 历史正确性矩阵：[run-regressions.py](run-regressions.py)。只写本轮 `baseline/`，不覆盖上一轮审计证据。
- 新原生 probes 的失败断言保留正确期望；实现修复后应转绿。部分 headless/async/schema 程序使用现状特征断言，即使进程退出0也可能记录缺陷；不要用总退出码掩盖报告中的期望/实际差异。
- 各子报告提供唯一 runner/manifest；共用已有依赖缓存，各自只操作自己创建的临时 workspace。没有修改产品、正式测试、历史审计或提交 Git 变更。
- 本轮没有复跑真实 Linux 桌面、真实触控板全序列或完整视觉美学审计；CPU paint Scene 验证的是布局/绘制几何，不是截图美观评估。没有确认新的独立安全漏洞，也不据此声称安全审计穷尽。

发布验收应先让本轮正确行为断言转绿，再在修复后的同一最终提交执行项目既有 workspace/native/Clippy/package/跨平台 smoke gates。当前提交有稳定可复现的 P1，不能仅凭历史矩阵与 CI 通过批准出货。
