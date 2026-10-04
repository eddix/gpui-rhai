# Release candidate：虚拟运行时与新增 API 组合复核

基线：`736025a8b2b1ac44d41d893bc9a929c990b97532`，对照 R7 `cda11ce7`。只读产品与历史报告，仅新增本目录材料。排除 disktree。

**结论：已修的 batch/final-manifest/read-contribution 结构维持；4 组 slot + theme 组合通过。确认一个影响范围较窄的 Table helper 外部装饰完整性缺口，建议按 P2 或明确范围后的后续项处理，不视为官方 Table 普遍退化。**

## 通过的相邻组合

独立 probe 注册一个同时支持 `PrimitiveValue::Node` 与 `PrimitiveValue::Nodes` 的原生 primitive 描述，让其结构槽承载 virtual_collection。分别使用 raw row 和正式 Counter row，共四组。

每组断言通过：

1. 初始 seed 后请求此前未实现的 row 10，正确穿透 Node/Nodes 槽查找与替换；正式/根状态回调均返回 7。
2. row 在 render 中调用新增 `ctx.theme_variant()`。Host 选择 Light 后，dirty owner 精确为当前正式 Counter，或 raw row 的真实 root；移出的旧 Counter 没有被重新标 dirty。
3. 相同 theme metadata 在 Event phase 读取，不产生额外订阅。
4. 执行 dirty render 后，row 10 文本从 Dark 更新为 Light。
5. 完整 target 清空后，正式 Counter 旧 callback 被拒绝；再改主题没有遗留 reader 导致 dirty。
6. 一个候选 row 先读取 theme metadata 再抛错，回滚后 target 仍为空，后续主题更新也没有幽灵依赖。

证据：[probe.log](probe.log)、[源程序](probe/src/main.rs)。这些是在实际 RuntimeEngine / ScriptLifecycle 上执行的逻辑槽、调度、生命周期及依赖测试。dummy native handler 没有绘制子槽，因此这里不把结果说成 GPUI 物理布局或 Table 列宽的独立验收；真实 native suite 由主审统一执行。

另测正式 `RefRow` 的 element_ref 声明：目标 `[10]` 替换旧窗口，以及 `[0,10]` 保留旧行并新增行，两者均实现成功。这与新 final manifest 保留正式组件资源的路径一致。几何测量、logical ref 绑定切换和 native notification 由 transforms 专项覆盖。

## API 完整性 · P2：Table 标记作为正式组件返回值的外部装饰时，dirty render 后丢失

定位：`crates/gpui-rhai/src/node.rs:1934-1945`、`:1950-1964`；子树替换的 presentation replay 在 `:1118-1121`。

新增 `with_table_track` / `with_table_column` 已注册为可调用的 Rhai UiNode 方法，接受合法 Box/column index，但直接写 `table_layout` / `table_column`，没有进入 `NodePresentationMutation`。调用方把标记附加到正式 Panel 的返回节点后，Panel 的自身 state 更新使该子树重建，原标记没有被 replay。

最小行为对照：同一个正式 Panel 显示 count；把 count 从 0 更新到 1。外部同时附加普通 `with_style(width=123)` 作为 presentation 保留对照。

| 标记位置 | track before→after | column before→after | 外部 width |
|---|---|---|---|
| Panel render 内部设置 | true→true | true→true | 123→123 |
| 调用方装饰 Panel 返回值 | **true→false** | **true→false** | 123→123 |

count 确实更新为 1，普通 width 装饰仍在，所以不是更新没执行或整个节点被任意丢弃。column 场景还把该 Panel 放在真实 `with_table_track` 容器下，避免仅测试一个没有所属 track 的无意义 annotation。

原始观测：[table-modifiers.log](table-modifiers.log)。使用 UiNode 的 Debug 观察真实字段：`table_layout: Some(Columns(...))` / `table_column: Some(0)` 在外部装饰场景消失。源程序的 `table_modifier` 可直接复现，未修改产品或通过自建模型替代它。

示意调用：

```rhai
// Panel 自身状态改变后，这个调用方附加的 track 目前会丢失。
Panel(#{key:"panel"}).with_table_track(columns, [])

// Cell 是正式组件；列标记目前也会在 Cell 自身更新后丢失。
column([Cell(#{key:"cell"}).with_table_column(0)])
    .with_table_track(columns, [])
```

**影响范围：** 当前官方 `registry/components/table.rhai` 在自身 render 内设置 track，在 raw cell box 上设置 column，不走上述外部正式组件装饰路径。本轮没有证明官方 Table 的正常使用因此受损，也没有测量这条路径的实际 GPUI 列宽变化。检索 USER_GUIDE/组件文档未找到承诺这些 helper 可用于外部装饰的专门说明。

处置选择：如果维持可组合 UiNode modifier 的一般语义，给两个标记补齐 presentation replay；如果它们只打算作为 registry 内部构建接口，则明确内部边界、限制或校验不支持的正式组件外部装饰，避免首次接受、更新后静默改变。按这个限定范围评估 P2/后续完整性工作，不应升级为“整个 Table 无法出货”的 P1。

## 排除的假设：raw virtual row 直接声明 element_ref

保留了 [原始诊断](raw-ref-rejected.log) 与 [当时源程序](raw-ref-rejected.rs)。该探索在初次 mount 就被现有规则拒绝：`element refs may be declared only by formal components`，尚未进入“保留旧行 + 新行”的候选路径。

因此不能把这份 panic/exit 101 统计为产品回归。最终有效日志 `probe.log` 不包含它；合法正式 RefRow 两组对照成功。这里没有要求放宽已有资源声明边界。

## 源码范围判断

- StateStore batch delta、最终 invocation graph、nested collection 的完整 target 路径没有被本轮修改替换；R7 两个 P1 的修复结构仍在。本分支未重复跑整套旧批次压力矩阵。
- theme metadata 使用已有 ReadDependency，且 Render-only；没有新建绕过 contribution 的旁路订阅。
- element_ref 的逻辑订阅与 native geometry reader 被分开，provider/reader 清理进入 final manifest，snapshot 继续保留事务回滚边界。详细 native 几何结论以专项报告为准。
- Table 新增字段属于节点布局标记，Node/Nodes 子树遍历与 collection 查找/替换仍覆盖 Custom 槽。上面的 helper replay 缺口与该遍历不是同一问题。

复现：

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-04-release-candidate-review/virtual-runtime/probe/Cargo.toml --offline
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-04-release-candidate-review/virtual-runtime/probe/Cargo.toml --offline -- table
```

第一条的有效组合断言通过；第二条为表征输出，exit 0 不意味着外部装饰的 marker 丢失符合预期。
