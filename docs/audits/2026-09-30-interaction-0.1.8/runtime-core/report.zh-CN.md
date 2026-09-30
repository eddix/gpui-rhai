# 0.1.8 对抗式审查：Interaction Runtime 核心

审查日期：2026-09-30。基线：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`，对比 `v0.1.7`。
核验范围：ADR 0022、Interaction coordinator、PrimitiveContext、批量 signal preview、Host/View 的路由与生命周期。

## 结论

统一工作已经完成了公共 move/up 分发和信号批量写入，但尚未形成 ADR 0022 所要求的完整生命周期模型。已通过实际 ScriptView 和 GPUI 输入确认两个 P1：同 Host 多 View 会丢失前面 View 的 pointer capture 路由；卸载应用拖放源后仍能提交旧 payload。兄弟审查另已确认取消回调在 suspend 路径重入同一个 ScriptHostView，发生 panic。

这几项应在 0.1.8 解决。它们直接影响既有嵌入模式和新增拖放契约，不是预 1.0 API 兼容性问题。

## RC-1 [P1，原生探针确认] 每个 View 覆盖 Host 唯一 capture 路由，前面 View 的捕获失效

定位：

- `crates/gpui-rhai/src/interaction.rs:415`：`set_pointer_routes` 无条件覆盖两个 `Option`。
- `crates/gpui-rhai/src/renderer.rs:1049`：每个 View 自带独立 capture registry、handler map、dispatcher，再向共享 coordinator 注册。
- `crates/gpui-rhai/src/renderer.rs:1171`：每个 pointer router 的 paint 都执行注册。
- `crates/gpui-rhai/src/interaction.rs:746` / `:764`：move/up 只能查询最后注册的路由。

触发：同一个 ScriptViewHost 并列两个 ScriptView。第一个有 80×80 的节点，pointer_down 返回 `event_response().capture_pointer()`，move/up 记录 captured 状态；第二个仅显示一行文字。按下第一个节点后，将鼠标移动到它边界之外并松开。

期望：事件继续送往第一 View 捕获的节点，最后释放捕获，与是否存在第二 View 无关。

实测：

| 布局 | 移到节点外 | 松手 |
| --- | --- | --- |
| 单 View 对照 | `captured-move` | `captured-up` |
| 同 Host 两个 View | 仍为 `down` | 仍为 `down` |

第二个 View 没有任何事件或 capture，也会使第一个 View 丢失路由。旧版本每个 View 安装各自的监听器；本次整合为一组 window listeners 时，没有一并实现按捕获所有者选路的注册表。

修复应保留 Host 单路由设计，但维护所有 View 的 frame-local routes，并按实际 owner 选路；不能要求使用者为每个 View 创建不同 Host，否则会破坏同 Host 跨 View 拖放及共享 overlay 设计。generic pointer capture 与 NativeGesture 的所有权转移、冲突和释放也应有明确规则。

验收：同 Host 两个以上 View，捕获来自首/中/末 View；另一 View 重绘、排序、暂停或卸载；同 Host 嵌套 HostSlot；move/up 离开原节点之后仍只送到实际所有者，up/cancel 后没有残留捕获。

## RC-2 [P1，原生探针确认] 应用拖放源卸载后，旧 payload 仍可提交到目标

定位：

- `crates/gpui-rhai/src/interaction.rs:429`：`finish_frame` 对 `drag.spec.source == active.owner` 豁免了“owner 已不在 presented 集合”的取消条件。
- `crates/gpui-rhai/src/drag_drop.rs:498`：源 primitive 卸载仅从实例表删除 Entity，没有通知 coordinator。
- `crates/gpui-rhai/src/interaction.rs:527`：结束时使用保留的 application drag 和当前目标执行 commit。

触发：按下 DragSource，移动超过阈值并进入 DropZone。通过正常的 Rhai 状态事务移除源节点，使新一帧包含 `source removed`，保留目标节点，然后在目标上松手。

期望：源的逻辑 mount 已结束，拖放必须取消；后续 pointer_up 不得发送旧源 payload。

实测：目标回调仍运行，状态变为 `dropped:deleted-resource`。对照测试中源仍挂载时也能正常 drop，所以不是目标坐标或模拟事件无效。

“虚拟化行暂时离开可见区域”与“逻辑源卸载”不能共用“不在本帧也继续”的豁免。若 Sortable 需要 pin 正在拖动的虚拟行，应由仍然存活的逻辑 owner 证明其有效性；源 collection、组件或 mount incarnation 消失必须无条件取消。只删除当前豁免、而不解决虚拟源身份保持，也可能引入另一类回归。

验收：逻辑卸载、删除源数据、同 key 重挂载、源替换、disable、view suspend/dispose、Host teardown 均不允许提交陈旧 payload；虚拟化只暂时隐藏源时按明确 pin 契约处理。取消不能影响另一个同名但不同 mount 的会话，也不能留住已销毁的 source Entity。

## 统一模型核验：完成了什么，尚缺什么

### 已完成的有效基础

- 公共 pointer move/up 的分发和基础 threshold/armed/active/finished/cancelled 状态已集中到 `interaction.rs`。
- `signal.rs:438` 的 batch 写入先完整验证所有 member，再在一次 foreground 调用内修改；重复 ID、类型错误、非有限值和 stale handle 会在写入前返回错误。`app.rs:3279` 使用一次 Entity update 和一次 notify 发布整组写入。本轮源码检查未发现这段原子写入存在部分更新问题。
- PrimitiveContext 封装了 typed props 辅助访问、信号、几何和语义 proposal；Rhai 未获得 GPUI 对象或 coordinator 的直接句柄。

### 架构承诺尚未兑现，不应宣称“完全统一”

| ADR 0022 的要求 | 当前实现 |
| --- | --- |
| owner 包含组件路径、mount incarnation、retained/native identity | `InteractionOwner` 只有 `view` 和字符串 `key`；`PrimitiveContext` 未保存已经可取得的 retained ID / mount 信息。 |
| shared awaiting-control / source revision / proposal generation | `GesturePhase` 无 awaiting-control；`GestureSession` 无 source revision、pending proposal 或 generation。 |
| 所有 cancel 进入同一安全生命周期边界 | 普通 unmount 的 frame 检查、application drag 豁免、discard_view 直接丢弃、suspend 内同步回调走不同路径。 |
| 旧 Host acknowledgement 不清除新手势 | 目前依靠各 primitive 的本地 source 比较和本地 preview 逻辑，没有公共 acknowledgement 身份。 |

这里需要精确区分：`PrimitiveContext::interaction_owner` (`primitive.rs:969`) 拼的是 `view_id + primitive 类型 + 调用者传入的字符串`。部分调用者已把 component path 放进字符串，例如 Draggable 在 `draggable.rs:437` 使用 signal component path 和 key，因此不能泛称所有组件的相同局部 key 都会碰撞。但这些 owner 仍没有统一 incarnation；DragSource 的字符串只含 `source_id`，其他调用者也各自自行拼接。不同 primitive 解决身份问题的程度不一致，本身就是统一边界未闭合的证据。

RegisteredPrimitiveElement 在 `primitive.rs:1818` 创建 context 时没有传入 retained ID，而紧随其后的 `PrimitiveRenderIdentity` 已具有它。应由基础设施提供结构化 owner identity，而不是让每个行为继续猜字符串格式。

controlled preview 也未实现 ADR 描述的保留协议。例如 `draggable.rs:241`、`resizable.rs:374` 都先清除 preview，再 defer semantic proposal。不能把这视为已经支持异步 Host acknowledgement。本报告没有以此单独制造一个未验证的视觉闪烁 finding；整改时应明确实现并测试接受、钳制、拒绝、延迟、旧 revision/new gesture 的协议，或者经过明确设计修订收缩其承诺，不能用立即回写示例证明通用协议存在。

### 与并行审查交叉确认的 P1：suspend 取消重入

独立证据见 [`resize-controls/probe.log`](../resize-controls/probe.log)，测试 `active_draggable_suspend_is_safe`。确认错误为：`cannot update gpui_rhai::app::ScriptHostView while it is already being updated`。

路径为 `ScriptViewHandle::suspend → ScriptHostView::suspend_view → quiesce_view → cancel_view → active.cancel → clear_preview → PrimitiveContext::write_signals → 原 ScriptHostView Entity::update`。`app.rs:4344` 位于该 Entity 的 update 内部；`app.rs:3283` 再次请求同 Entity update。应将协调器取消和 Runtime 内部预览清理设计成不会重入的统一事务/阶段，而不是在不同组件分别补一个 defer。具体严重级别与复现由 resize-controls 报告主记录，避免重复计数。

## 建议的整改单位

优先把以下边界一起补齐：

1. Host 保存所有 view 的路由与当前真实 owner，统一转移和释放所有权。
2. owner 使用结构化 view/component/mount/retained identity；presented geometry 和逻辑 mount 存活分别建模。
3. cancel 有明确的阶段和幂等性，在 entity 更新、render/prepaint、卸载、暂停中均不重入，不把旧 cleanup 应用到新 generation。
4. controlled proposal 有源 revision、手势 generation 和 pending acknowledgement；各行为只提供尺寸/位置/值的纯计算政策。

这些是当前中央协调器应承担的职责，不需要再增加一层庞大通用框架。把 move/up listener 收到一个文件只是第一步。

## 独立探针与运行结果

执行：

```sh
python3 docs/audits/2026-09-30-interaction-0.1.8/runtime-core/run-probes.py
```

脚手架复制当前 native-keyboard manifest/lock 到临时独立 workspace，路径改为绝对路径，复用既有 target 缓存。只使用公开的 ScriptView/Rhai/GPUI 输入接口；未改产品源码或正式测试文件。

源码：[`probes.rs`](probes.rs)。完整输出：[`probes.log`](probes.log)。

| 探针 | 结果 |
| --- | --- |
| `capture_single_view_control` | PASS，正常对照 |
| `capture_first_view_survives_second_view_paint` | FAIL，正确性断言确认 RC-1 |
| `application_drag_live_source_control` | PASS，正常对照 |
| `application_drag_must_cancel_when_source_unmounts` | FAIL，正确性断言确认 RC-2 |

最终一次执行共 4 项、2 通过、2 失败；失败是保留的正确契约断言，不能称为“产品测试全部通过”。本轮未重新执行整个 workspace 的全量 suite，未把 headless 原生探针当作真实设备 GUI/VoiceOver 验收。产品基线在测试结束时仍为 `0b9b887c`。
