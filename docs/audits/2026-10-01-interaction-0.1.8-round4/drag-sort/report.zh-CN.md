# 0.1.8 第四轮：拖放目标滚动与虚拟回调边界

基线 `f6e936a5`，对照 `815bb82b`，按更新后的 ADR 0022 审查。
本分支新增 6 个原生探针，最终 **0 passed / 6 failed**，各测试均先验证相关正向前提。确认 1 项独立 P1 和 1 项 P2；另外 3 个正式行组件生命周期失败归并到集合分支，不重复计数。没有修改产品、正式测试或历史审计。

## R4-DS-1 · P1：延迟实现新行时重置 callback owner，用户回调在结构作用域中执行

定位：

- `crates/gpui-rhai/src/engine.rs:3334`–`3339`：初始 seeded realization 已使用 `for_structural_scope`，保留真实 callback owner。
- `engine.rs:1502`–`1506`：后续 `realize_virtual_collection` 仍调用 `recipe.context.for_component(scope, ...)`。
- `crates/gpui-rhai/src/context.rs:1139`–`1160`：`for_component` 会把 `callback_component`、`callback_incarnation` 和事件 schema 一并重置到传入的结构 scope。
- `engine.rs:2528`–`2535`：嵌套正式组件的 callback prop 从上述 caller context 捕获 owner。

### 已隔离生命周期删除的原生反例

虚拟行内创建一个正式 `Action` 组件，其 render 直接把 `props.on_action` 转发给文本节点的 `on_click`。没有中间的组件自身事件回调，也不依赖正式组件自己的 state；最终 `clicked(ctx,value)` 只读取并递增真实 root 的 `count`。

1. 初始 Action 1 点击成功，root count 从 0 变为 1。
2. 直接遍历 `view.root().VirtualCollection.spec.realized`，记录实际 seed manifest 为 `{0,1,2,3,4,5,6}`。没有把 AX 当前可见行误当成全部已实现行。
3. 进行真实 wheel/绘制后，选取可见的 Action 13，并断言它不在原 seed manifest 中。
4. 点击 Action 13，回调实际进入 `clicked`，但执行 context 属于 `/View[audit-drag-sort]/VirtualCollection[actions]`。读取 `count` 时报错，root count 仍为 1：

```text
component `/View[audit-drag-sort]/VirtualCollection[actions]` has no state field `count`
```

这是 **callback 被执行在错误 owner 上**，不同于另一个正式行组件 incarnation 被删除后“回调根本不能执行”的生命周期问题。初始正确、延迟错误的两条实现路径已经独立区分。

影响：首屏组件回调正常，滚动后新出现的组件却读取不到原有状态、把事件路由到错误组件。只有少量静态样本或只测 seeded row 的单元测试无法覆盖它。

整改方向：初始与后续虚拟实现必须共享结构 scope 与真实 callback owner 的构造规则，同时保留正确 incarnation、事件 schema 和脚本调用上下文。不要在 synthetic scope 人为创建 `count` 等业务 state，也不要为了回调可用而每次滚动重建整个业务 view。此次 `1505` 的结构上下文修复和集合分支的增量生命周期修复都需要完成，不能用其中一个替代另一个。

验收：保留 `delayed_row_forwarded_callback_preserves_root_owner`，必须验证目标不在实际 seed manifest；同时覆盖首次实现、后续实现、保留行、正式组件事件转发、直接 root callback 和错误回调。新单元测试不能只用永远已 seeded 的单行数据。

## R4-DS-2 · P2：虚拟滚动 ancestry 仍按同 View 的矩形相交猜测，遮挡后的无关列表也会滚动

定位：`crates/gpui-rhai/src/interaction.rs:1097`–`1127`。

`resolve_virtual_scroll_target` 要求指针在 viewport 内，并满足以下任一条件：

- active target 与 viewport 属于同一 View，且矩形相交；
- 同一 View 内存在任意接受 payload 的 target，其矩形与 viewport 相交。

随后选择面积最小的 viewport。这里没有目标到虚拟列表的真实祖先关系，第二个分支也没有检查 target 当前的 hitbox、clip 或显式 occlusion。

### 两个有正向对照的原生反例

场景固定：普通 DragSource 在左侧；右侧是 100 行虚拟列表。列表上方有一个完全覆盖它的兄弟 DropZone，显式使用 `style().occlude()`。覆盖层与列表没有祖先/子孙关系。

| 覆盖层的协议 | 原生 drop 结果（正向对照） | 背后无关列表 scroll_item | 期望 |
|---|---|---:|---:|
| 接受该 payload | accepted，target_id 为 cover | 0 → 29 | 保持 0 |
| 拒绝该 payload | rejected | 0 → 29 | 保持 0 |

两种情况均在底边内进行 30 次小幅移动，每次处理实际 GPUI 绘制，且 `last_error=None`。

这与第三轮未列为缺陷的 Normal hitbox 场景不同：此次前景明确声明 `occlude()`，实际 native drop 的接受/拒绝结果也证明遮挡已经生效。错误发生在滚动目标选择：接受前景时，把相交的背景列表误认为前景目标的滚动祖先；拒绝前景时，fallback 从已被遮挡的背景 targets 中重新找到接受者，仍滚动列表。

影响：在弹出面板、重叠工作区、嵌套视口中拖放，会改变用户没有操作的背景滚动位置。进入不接受该类型的区域，虽然最终正确拒绝 drop，拖动过程却已经改变后台视口。ADR 已明确 auto-scroll 属于接受目标及其 scroll ancestry，当前实现没有满足这个关系。

整改方向：在目标注册时携带由 retained/native 层生成的真实滚动祖先链或带作用域的 viewport identity。普通 ScrollHandle 与虚拟 ListState 应从同一条目标 ancestry 消费滚动策略。行间空隙的延续滚动也必须保留已经验证的目标视口归属及可接受状态，不能扫描任意相交 target 重新猜测。源 lease 的 collection/index/snapshot 不应再次承担目标选择职责。

验收：保留接受/拒绝两种显式遮挡反例；继续覆盖外部源到真实虚拟目的地的正向行为、嵌套目标、目标切换、卸载和取消后的停止。增加“较小矩形优先”等 tie-break 不能建立祖先关系。

## 另交集合分支：正式行组件自身的 incarnation 被错误清除

DropZone 使用内部 `drop_zone_commit` 回调再向调用者发出 drop。初始 Lane 1 成功回调后，以下三个场景失败：

1. 滚动 45px 后，仍然可见的 seed Lane 2 无法触发内部回调。
2. 多次滚动后可见的新 Lane 14 无法触发内部回调。
3. Lane 5 当前可见时通过显式 keyboard_target 选取它，内部回调也失败。此例未独立证明 Lane 5 最初未 seeded，不能作为延迟 owner 的隔离证据。

错误首先指向正式行组件自身，例如：

```text
callback `drop_zone_commit` belongs to an unmounted incarnation of component `/View[audit-drag-sort]/VirtualCollection[lanes]/DropZone[lane-2]`
```

这些失败归并到集合分支的增量实现被当作完整 scope 提交问题：`lifecycle.rs:450` 附近只把 missing indices 交给 engine，但 `finish_component_render` 用整段 VirtualCollection scope 替换 invocations、state 与资源清单。集合分支另有独立 state/self-callback 和 prune-only 对照。本分支只提供真实 DropZone 组合的原生证据，不另行计数。

## 覆盖与证据

读取了新版 window Escape interception、deactivation cancellation、source lease 与 destination scroll 实现；此前正确性探针由主审查统一复跑，这里没有重复运行。普通 Normal hitbox 的默认策略以及显式 keyboard_target 的业务政策不重新列为缺陷。

```sh
cargo test \
  --manifest-path docs/audits/2026-10-01-interaction-0.1.8-round4/drag-sort/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

最终日志：[probe-results.log](probe-results.log)。源码：[probe/src/lib.rs](probe/src/lib.rs)。

6 个独立探针的预期行为断言均失败，各自的初始成功回调、有效新行选择或目标接受/拒绝前提均通过。实际 seed manifest 的检查很重要：只看到 Action 5 首次出现在屏幕上，不能证明它是新实现的行。源码、package 与测试命名属于本轮专用目录，依赖缓存复用既有原生 target。失败断言应通过产品修复转绿，不应改成接受错误行为。
