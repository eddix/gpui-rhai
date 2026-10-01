# 0.1.8 第三轮：虚拟拖动租约与目标滚动复审

基线 `815bb82b`，对照 `d77b4b49`。本分支新增 5 个原生探针，最终 **3 passed / 2 failed**；不重复计数前两轮已经修复的原始用例。未修改产品、正式测试或历史审计。

确认两个新场景仍未闭合：离屏源虽然继续存活，但 Escape 已不能可靠取消；虚拟自动滚动只根据源 collection 工作，普通 typed DragSource 进入虚拟目的地时没有对应滚动。

## R3-DS-1 · P1：离屏拖动租约继续存活，但取消键随源焦点路由一起离屏

定位：

- `crates/gpui-rhai/src/interaction.rs:712`–`730` 新的逻辑源租约允许未绘制的源继续持有 active drag。
- `crates/gpui-rhai/src/app.rs:249`–`261` 的 Escape 取消只注册在 Host 容器的 `on_key_down` 路由中，依赖当前聚焦节点的派发祖先链。
- `crates/gpui-rhai/src/virtual_list_element.rs:319`–`324` 由 GPUI list 只绘制当前窗口的行；源仍在逻辑集合及 retained/pin 集合中，不代表该焦点在当前绘制派发树内仍有通向 Host 的路径。

复现：

1. 100 项虚拟 Sortable，180px 视口，从 Item 0 开始拖动至底部。
2. 此后不再发送鼠标移动，推进 12 次 16ms 绘制周期。`scroll_item` 从 0 前进到 6，证明新静止指针自动滚动已经生效，源也确实离开初始窗口。
3. 按 Escape，继续推进 12 个绘制周期。`scroll_item` 从 6 继续前进到 15。
4. 松开鼠标，仍收到 `item-0:before:item-20` 排序提交，而不是取消。

两个正向对照均通过：

- 源仍然可见时，同样发送完整 down/move/Escape/up，不会提交。
- 源离屏后，只在测试中调用公开 `view.focus(window,cx)` 恢复 Host 焦点，再按同样的 Escape，滚动保持 7→7，随后 up 的状态仍为 `ready`。

因此不应把这个失败解释为测试按键不正确或“自动滚动只剩余一帧”。取消后的 12 帧持续滚动，最终也确实提交了业务提案。另一个容易误判的细节是 `window.focused(cx)` 在失败场景仍返回 `Some`：焦点句柄存活不等于该句柄仍拥有当前绘制树内的正确派发路径。GPUI 0.3.7 的 `DispatchTree::focus_path/focusable_node_id` 均基于当前帧的 focus-node 映射。

影响：用户明确按 Escape 取消后，列表继续滚动并在松手时重排数据。新逻辑源 lease 修复了过早取消，却没有同时保证存活期间取消输入的可达性。

整改方向：active gesture 的取消路由必须有稳定的 Host/window 归属，不能只寄托在可能离屏的源焦点祖先上。可以给会话提供稳定的键盘派发拥有者并正确恢复原焦点，或让已授权的 Host 域统一接收会话取消；不要为了接收 Escape 强制把所有离屏行继续绘制，也不要取消其他 Host 的手势。源逻辑存活、目标可见命中、焦点派发三者应分别建模并在手势结束时统一清理。

验收：保留本次失败探针及两个正向对照；补离屏期间 Escape、窗口失焦、源被删除、Host/view suspend、鼠标丢失与新手势覆盖。取消必须停止继续滚动、清除租约，后续 up 不得提交。

## R3-DS-2 · P2：虚拟 auto-scroll 绑定源 collection，外部 typed drag 无法滚动虚拟目的地

定位：

- `crates/gpui-rhai/src/interaction.rs:561`–`566` 按 `drag.spec.collection` 查找 `virtual_scrolls`。
- `interaction.rs:572`–`580` 普通滚动从当前目标的 ancestry 查找，但虚拟滚动从源 spec 查找，两个分支的归属不同。
- `ApplicationDragSpec::with_collection` 仅由 Sortable 填充；普通 DragSource 合法地没有 source collection。

复现：把普通 DragSource 放在虚拟列表外。虚拟列表有 100 个 lane，每行是一个接受相同 payload type/operation 的 DropZone，视口高 180px。拖入底边内 5px，连续 30 次小幅 pointer move 并绘制，`scroll_item` 始终为 0。

此场景不是目标没有注册或 typed drop 无效：源的 `on_drag_end` 收到 `accepted=true,target_id="lane-4"`，且 `last_error=None`。测试不依赖目标回调修改业务数据；它只以原生接受结果证明命中与协议有效，再断言实际目的地应滚动。

影响：新 ListState 支持只对“源和目标恰好同属一个 Sortable”的特例成立。资源面板拖入虚拟分组、跨 View 资源拖放、虚拟长列表接收外部对象等正常组合仍无法在拖动过程中到达初始窗口之外的目标。不能要求普通 DragSource 伪造内部 collection 字段来启用滚动。

整改方向：源 collection/index/snapshot 只负责源租约；自动滚动应由当前有效目标及其滚动祖先决定。DropTarget 应能携带统一的普通/虚拟滚动 ancestry 或绑定有作用域的目标 viewport identity，再由各自 ScrollHandle/ListState 执行。有明确边缘策略时，即使指针处于行间空隙，也应使用仍有效的目标视口，而不是把虚拟滚动身份绑到 source。

验收：保留外部 source→虚拟 destination 的探针；补不同 collection/View 的目的地、嵌套滚动、鼠标离开视口和取消后停止、不接受 payload 的目标不被无故滚动。仍需保持每帧有界，不扫描全体数据或调用 Rhai 计算滚动。

## 正向结果及边界

- 静止指针确实能持续自动滚动；新 ListState 接入与帧刷新并非完全缺失。R3-DS-1 的问题是取消路由失联。
- 对离屏源分别进行 Host 集合替换：删除源、调整投影顺序、禁用源、保留同 key 但替换内容，旧拖动都没有继续提交。四种情景共用一个参数化原生探针，全部通过。
- 上述同 key 替换是数据内容替换，未冒充逻辑 unmount/remount 的独立实证。其他世代与跨 View 场景由主审查的历史回归及相应分支覆盖。
- 普通 Normal hitbox 和显式 keyboard_target 的政策继续沿用第二轮结论，不重新列为安全缺陷。

## 另交集合分支的问题

首次构造外部 typed drag 探针时，在 raw `virtual_collection` 的行 renderer 内组合正式 DropZone，并设置 `on_drop:Fn("dropped")`，真实 drop 触发了：

```text
callback `dropped` belongs to an unmounted incarnation of component `/View[audit-drag-sort]/VirtualCollection[targets]`
```

原始源码和日志分别保留为 [raw-virtual-callback.rs](raw-virtual-callback.rs)、[raw-virtual-callback.log](raw-virtual-callback.log)。集合审查分支已接手 synthetic scope/incarnation 的根因分析；本分支不重复计数。

为隔离 R3-DS-2，最终探针移除了目标的可选回调，改由外部 DragSource 自身在真实 root scope 中的 `on_drag_end` 验证接受结果。这没有修改产品，也没有将 callback failure 当作 auto-scroll 的证据。

## 重跑

```sh
cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round3/drag-sort/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

最终 [probe-results.log](probe-results.log) 为 3 passed / 2 failed；源码在 [probe/src/lib.rs](probe/src/lib.rs)。独立 package/源码目录复用了原生测试依赖缓存。失败项是产品预期行为断言，应在产品修复后转绿，不应改成接受错误行为。
