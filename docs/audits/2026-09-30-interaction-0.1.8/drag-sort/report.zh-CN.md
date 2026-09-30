# 0.1.8 DragSource / DropZone / Sortable 对抗审查

审查基线：`0b9b887c`。产品依赖为 `gpui-pre = 0.3.7`，工具链为 Rust 1.95.0。
本报告不修改产品代码、既有测试或历史审计材料。所有探针在独立 workspace 中直接调用本地产品代码。

结论：发现 2 项 P1 和 1 项 P2。发布前应修复。5 项原生探针中，1 项正向对照通过，4 项期望行为断言失败；失败是本报告保留的产品缺陷复现，不应改成断言错误行为通过。

## DS-1 · P1：拖放命中绕过实际裁剪与前后遮挡，隐藏目标可以接收操作

定位：

- `crates/gpui-rhai/src/drag_drop.rs:290` 插入 hitbox 后丢弃，仅保留完整布局 bounds；`323` 起将其作为 drop target 注册。
- `crates/gpui-rhai/src/interaction.rs:333` 的 `DropTargetRegistration` 没有有效 clip、hitbox 或绘制前后顺序。
- `crates/gpui-rhai/src/interaction.rs:787` 的 `resolve_drop_target` 只测矩形包含；`807` 的比较规则只有 priority 和面积。

独立原生复现一：父容器 `overflow_hidden`，可见 Y 范围为 `0..80`，DropZone 完全放在其外部 Y=`120..180`。把源拖到 `(290,150)`，本应没有目标，实际收到 `drop:hidden`。探针直接读取真实布局坐标，然后发送鼠标 down/move/up，未直接调用目标回调。

独立原生复现二：两个完全重合、默认 priority 相同的 DropZone，先绘制 `zz-back`，再绘制 `aa-front`。拖到用户看到的前景区域，实际收到 `drop:zz-back`。等 priority、等面积时，BTreeMap 的 owner 字典序决定最后胜者，目标命名会改变交互对象。

影响：滚动裁剪区外的行、被覆盖的面板仍可触发业务移动。输入看到的目标与收到提交的目标不一致。Sortable 也复用同一 resolver，不能仅在 Gallery 修补。

整改方向：在已有 presented geometry / 原生命中模型上产生可命中的目标集合。先过滤有效可见区域与遮挡，再在符合条件的目标中应用显式 priority 和嵌套规则；不要用布局矩形或 key 顺序替代命中。交互上下文必须携带这一帧的 geometry/clip/owner，而不是另建与渲染脱节的命中世界。已裁剪到空的目标可以保留语义身份供键盘操作使用，但不得作为指针目标。

验收：保留本次两项探针；增加滚动前后、部分裁剪、嵌套目标、被普通可交互前景/模态层遮挡、目标被移除的组合。改名和交换无关组件声明顺序不应改变同一绘制结果下的命中。

## DS-2 · P1：Sortable 用组件局部 key 作为 Host 全局拖放通道，独立列表相互污染

定位：

- `registry/components/sortable.rhai:140` 将 `props.key` 作为 list_id 传入每个 native item。
- `crates/gpui-rhai/src/sortable.rs:248` 将 list_id 直接拼入 payload type；`252` 拼出全域 target_id。
- `crates/gpui-rhai/src/sortable.rs:330` 以该类型注册目标；`339` 收到源后只排除自身/相邻 no-op，没有验证源属于同一已挂载列表。
- `crates/gpui-rhai/src/primitive.rs:969` 的 `interaction_owner` 只包含 view、primitive 类型与字符串 key，没有完整组件实例路径或挂载世代。这里只据实指出 DragSource/Sortable 的调用方式；不把它推广为所有交互组件都缺少路径。

独立原生复现：创建两个合法的 `AuditPanel` 组件实例 `left`、`right`，各自内部都有局部 `key:"queue"` 的 Sortable。两者处于不同组件作用域及不同 retained 父节点，产品正常挂载。左列表包含 `left-a/left-b`，右列表包含 `right-a/right-b`。将 `left-a` 拖到右列表的 `right-b` 后，收到：

```text
left-a:after:right-b
```

这不是测试误用重复兄弟 key：探针明确建立两个独立组件实例与各自根容器。早期无组件边界的实验被产品正确拒绝，已从最终复现中排除。

影响：可复用组件中自然出现的局部 key 复用，会把本不支持跨列表移动的 Sortable 连接起来。目标回调收到自身集合中不存在的 source_key；应用按组件契约处理 reorder 时可能报错或错误修改数据。源/目标 item key 也相同时，还会使 owner 注册相互覆盖。不得要求使用者把每个封装组件内部 key 改成应用全局唯一来绕开问题。

整改方向：引入绑定已挂载列表实例的原生身份，至少包括 Host/view、完整组件实例身份与 mount generation。Sortable 的内部源/目标必须验证同一列表实例和有效成员身份；公共 DragSource 的业务 payload type 与 Sortable 内部排序身份分开。若未来支持跨列表移动，必须有明确 group/collection 归属和原子业务协议，不能由偶然相同的 key 自动开启。虚拟 pin 也应消费同一身份，避免只按 source_id 搜索所有列表。

验收：本次合法嵌套复用；同 Host 跨 View 同名列表；两个相同组件实例各使用 `item-0`；移除后相同 key 重建；同一列表内正常 pointer/keyboard reorder；使用者明确配置的 DragSource/DropZone 同 Host 跨 View 仍按其公开协议工作。

## DS-3 · P2：虚拟 Sortable 的点击焦点与键盘路径没有闭合

独立原生证据：同一 harness、同样完整的 mouse down/up、同样 Alt+Down 与 Alt+End：

| 模式 | 点击后的 GPUI focus | Alt+Down | Alt+End |
|---|---|---|---|
| 3 项 bounded items | 存在 | `item-0:after:item-1` | `item-0:after:item-2` |
| 100 项 virtual_data，180px 视口 | `None` | 没有事件 | 没有事件 |

虚拟模式随后将 Item 0 用鼠标拖到可见 Item 2，成功收到 `item-0:after:item-2`。因此不是脚本未挂载、on_reorder 回调无效或整个 native 路径不可用。失败是点击后没有可用焦点，公开的键盘操作无法继续。

相关定位：

- `registry/components/sortable.rhai:98`–`109` 虚拟项覆盖整行的原生交互层，没有 bounded grip 的 focus style/焦点所有者配置。
- `crates/gpui-rhai/src/renderer.rs:2330` 仅沿 `focus_styled` 祖先查找可共享的焦点身份；`app.rs:3961` 维护这类句柄。
- `crates/gpui-rhai/src/sortable.rs:465` 原生 fallback focus、`519` Render 的 track_focus 与实际虚拟行的 dispatch tree 需要一起修复。

本报告已证实外部行为，尚未通过修改产品验证“究竟哪一步失去焦点”。上述焦点身份配置是定位入口，不把尚未单独验证的解释写成唯一根因。

另外有独立的静态闭合问题：`sortable.rs:496`–`514` 的 Home/End 只构造首尾 target_id，`interaction.rs:559`–`566` 只查当前已绘制注册表。虚拟首尾通常未 realized。**本次动态 Alt+End 被前置焦点故障挡住，因此未单列为第二项已实证缺陷。** 焦点修复后，仍须以集合语义完成首尾移动或有界 reveal，不能把键盘排序等同于当前屏幕上的指针目标集合。

验收：保持 bounded 正向对照；虚拟项点击与 Tab 可获得真实焦点，Alt+Arrow 可操作已实现邻居；首尾不在 realized 窗口时 Home/End 仍可完成；键盘移动、重排和 scroll/reveal 后焦点保持在源 key，且不能实现全部 100k 项来凑目标注册表。

## 性能与边界备注

`virtual_list_element.rs:105` 按 active source_id 在每个虚拟集合 prepaint 线性扫描所有数据，并且没有集合归属。已经交给 collection 审查分支量化，本报告不重复计数。该问题与 DS-2 应共享身份修复，但 O(N) 热路径还需要索引/有界定位机制。

未发现 payload 可携带 Rhai 可执行对象的已证实绕过：`ValueSchema::UiValue` 经 `UiValue::from_dynamic` 做类型与资源限制。本报告也没有把“回调拒绝业务更新”自动解释为 coordinator 的 accepted 字段错误；目前它报告的是目标协议接受提案，不应据此让源单独删除数据。

## 复现

```sh
cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8/drag-sort/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

执行日志：[probe-results.log](probe-results.log)。探针：[probe/src/lib.rs](probe/src/lib.rs)。
Cargo.lock 从已有原生测试锁文件起步，避免引入与产品验收不同的新依赖版本；最终锁文件属于独立 probe workspace。此次没有进行实际 GUI 人工视觉验收或真实操作系统 drag/drop 测试，不把它们计入通过项。
