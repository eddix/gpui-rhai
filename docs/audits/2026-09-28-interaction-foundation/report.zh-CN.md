# 0.1.8 交互基础重复实现审计

日期：2026-09-28。基线：`v0.1.7` / `a5876a9e`。本轮先审计现有
interaction hot path，再实施 ADR 0022；不以新增组件数量代替底层收敛。

## 结论

用户对重复实现的担忧成立。当前没有需要回滚 0.1.7 的 P0 缺陷，但存在
四组 P1 架构问题和三组 P2 重复。直接新增九个交互组件会使事件会话、坐标、
预览和生命周期继续分叉，因此 **0.1.8 必须先完成共享 Interaction Runtime，
再实现公开组件**。

审计集中于会被新组件复用的路径，而非按文件行数进行无目标重写。覆盖
`renderer/event/geometry/signal/primitive`，Resizable、SplitPane、Table 列宽、
Slider、Scrollbar、Chart、Canvas、VirtualCollection/NativeCollection，以及 Rhai
retained callback context。

## P1：发布前必须收敛

### F01 — 多套 pointer gesture 状态机

`resizable.rs:143-513`、`split_resize.rs:45-410` 和
`column_resize.rs:79-379` 分别保存 hovered/dragging/moved/start position，分别
注册 down/move/up window listener，分别处理丢失拖动、preview、defer commit 和
notify。RangeInput、Scrollbar 与 Chart 又维护不同版本。

后果不只是代码重复：释放后的 preview 策略已经不一致。Resizable 和
SplitPane 清空 optional signal，ColumnResize 保留 `Some(width)` 等待受控源变化，
RangeInput 保留实体 preview。新组件会继续放大取消、拒绝和迟到 Host 更新的
差异。

整改：建立 Host-domain coordinator、统一 GestureSession 和受控
acknowledgement；组件只提供纯 preview/constraint/proposal policy。

### F02 — declarative capture 与 native drag 分裂

普通节点已有 `PointerCaptureRegistry` 和 `PointerCaptureRouterElement`
（`event.rs:295-335`、`renderer.rs:949-1129`）。native resize/scroll/range/chart
却绕开该所有权，通过每帧 `Window::on_mouse_event` 和本地布尔值判断 dragging。
GPUI 0.3.7 的 window listener 只在下一帧有效，现实现需要每个 handle 每帧重复
安装完整 listener 集合；八柄 Resizable 单帧可安装 24 个 listener。

整改：保留 GPUI hit testing 的 element down 入口，move/up/cancel 交给一个
Host-domain session router；统一 pointer owner、stale generation 与生命周期取消。

### F03 — 几何与 affine 事实不唯一

`GeometryRegistry` 保存 layout/visual AABB，Canvas 另存 motion transform；
`canvas.rs:688-696` 与 `renderer.rs:3153-3169` 分别计算二维变换，Chart 又拥有
独立像素 viewport 映射。当前 Canvas 已证明 paint/hit 必须共享呈现变换，但没有
可供 PanZoom、Rotatable、SelectionArea、DropZone 共同消费的 forward/inverse
坐标事实。

整改：增加有限、可组合、可逆的 `Affine2D` 与 PresentedGeometry；Canvas/Motion
先迁移。Chart 保留 domain scale，只在呈现边界转换，不强行并入通用 UI 坐标。

### F04 — preview 多信号写入不是原子操作

Resizable 的一个矩形 preview 连续调用四次 `PrimitiveEventEmitter::write_signal`
（`resizable.rs:495-512`）。每次调用都会进入一次 Entity update，并可能 notify。
新 PanZoom/Rotatable 会需要更多关联字段，继续逐字段写会制造中间状态和额外重绘。

整改：SignalRegistry 增加 validate-all/apply-all 的 batch；PrimitiveContext 暴露
一次 `PreviewPatch`，一批变化至多请求一次 repaint。

## P2：在共享内核迁移中清理

### F05 — PrimitiveEventEmitter 职责和 prop reader 重复

`PrimitiveEventEmitter` 同时负责 event、signal 与 geometry，名称和职责已经不符。
`number_prop/string_prop/bool_prop/signal_prop/ref_prop/style_prop` 在 Resizable、
SplitResize、ColumnResize、RangeInput、DocumentView、TextInput、TextArea 与 Chart
重复定义二十余次。

整改：替换为 typed `PrimitiveContext`/`PrimitiveProps` accessor，迁移完成后删除
局部 reader；不保留新旧双路径。

### F06 — collection 的扁平分组事实重复

Array 路径中 Table、Command、Combobox 分别在 Rhai 建 group order/bucket/flat row；
NativeCollection 中 Fuzzy 与 Table 也各有 `grouped_order`。Tree 和 Sortable 若另建
一套 flattened hierarchy，会重复 key、navigation、sticky、collapse 和 reveal。

整改：抽出稳定 flat-order/structural-row helper；Tree 新增 outline projection，
复用 VirtualCollection 的 key/reveal/realization。产品 schema 仍分别保留。

### F07 — 低层数值 helper 有小规模重复

finite number、f64/f32、轴向 delta、方向反转、clamp/snap 与 optional-float equality
散落多处。只抽取拥有相同不变量的小型纯函数，不把 Chart domain、scroll offset、
slider value 与 rectangle constraint 塞进一个万能 solver。

## 额外确认：#80

`ScriptInvocationContext::capture` 保存 `NativeCallContextStore.global.level`。Rhai
1.26 的 `call_within_context` 继续使用该 level，因此“完成一个 task 后注册下一个”
会把并不存在的异步嵌套累积为栈深度。operation counter 已有 base 对齐，call level
没有。

整改应区分同步嵌套和 retained/delayed invocation：仅后者从新 call-depth baseline
开始，不能通过提高 `max_call_levels` 隐藏问题。需要覆盖 task、subscription、
event/effect callback 长链，以及真实同步递归仍被限制。

## 不应进行的重构

- 不按十四万总代码行追求统一率，也不拆纯粹因为文件长。
- 不建立识别所有组件类型的 God object。
- 不统一具有不同产品含义的 Chart domain、scroll position 和 form value。
- 不为 planned component 添加空壳 export。
- 不保留旧、新 gesture runtime 的长期双实现。

## 实施工作包

1. **Characterization**：固定现有六类交互的 accept/reject/cancel/lifecycle 行为。
2. **Primitive boundary**：typed props、PrimitiveContext、deferred proposal、atomic
   PreviewPatch。
3. **Gesture ownership**：Host coordinator、GestureSession、单一 move/up/cancel router；
   迁移 Resizable、SplitPane、ColumnResize、RangeInput、Scrollbar、Chart。
4. **Spatial model**：Affine2D/PresentedGeometry，迁移 Canvas paint/hit/Motion。
5. **Collection model**：flat order/outline projection、virtual pin/anchor/auto-scroll。
6. **Bug fix**：#80 delayed call-depth baseline。
7. **Components**：Draggable、DragSource、DropZone、Sortable、PanZoom、SelectionArea、
   Rotatable、RangeSlider、Tree；最后更新 Gallery、Studio、文档和 release matrix。

每个工作包是内部依赖顺序，不是对外发布的中间态。0.1.8 只在完整矩阵通过后发布。

