# PR #104 第六轮验收：Style COW、owned snapshot 与逻辑 inset

基线：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`，对比 PR #102 后的 `c088e96f`。本子域 **未发现新增的已确认缺陷**。

审查不是仅观察 `Rc` strong count 或结构体大小：逐项检查了实际写入入口、快照激活与重放次序，并运行公开 API 的独立值语义探针。产品、正式测试和历史审计未修改；所有新增材料仅在本目录。

## 独立验证

- [公开 API COW 探针](src/lib.rs)
- [执行日志](cow-probes.log)
- [manifest](Cargo.toml)、[锁文件](Cargo.lock)

结果：**3 passed / 0 failed**。首次构建复用 native cache，仅编译小型审计 crate；没有设置 RUST_MIN_STACK、新建 target 或 profile。

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round6/style-geometry/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

覆盖：

1. 节点接收 Style 后，继续修改原始 Style 的 width、font-feature Map、fallback Vec，节点保持原值。修改克隆节点的 width/opacity/logical inset 不污染兄弟节点、旧节点或之后创建的 shared-default 节点。
2. hover/active/focus/disabled 各状态合成保持原优先级；克隆后修改 focus/active 不影响旧树，disabled 仍按既有规则覆盖 focus；嵌套 child 克隆和父节点默认样式互不污染。
3. 使用真实 RuntimeEngine、ScriptLifecycle 和正式 Counter 组件，执行一次 child callback 后局部 rerender：owned width 从 100 变成 110，调用者附加的 opacity/inset 保留；另一组件不变；此前保存的整棵 UiNode clone 仍保持旧文本和旧宽度。这直接覆盖 owned snapshot 更新与 caller presentation 重放，不是手工模拟快照。

## 全部 Style 写入口检查

| 入口 | 当前行为 | 结论 |
|---|---|---|
| `UiNode.style` (`node.rs:298`) | 私有 `Rc<Style>`，公开 `style()` 只返回 `&Style` | 没有可逃逸的 `&mut Style`；本基线不存在 `UiNode::style_mut`。 |
| 构造器 / shared default (`node.rs:315–320`) | 每线程共享只读空 Style | 首次实际修改经 COW；不会把某个节点样式写回默认模板。 |
| `NodePresentationMutation::Style` (`node.rs:421`) | `Rc::make_mut(...).merge_in_place(...)` | 写入前分离共享 Style，overlay 自身也保持不可变。 |
| `motion_ghost_node` (`node.rs:1521`) | clone 后 `Rc::make_mut` 再清理 base | 清理不会污染原节点或 owned snapshot。 |
| Style builder / `merged` (`style.rs:1104–1116`) | 仍是 owned Style 的值语义；`merge_in_place` 仅是内部合成提取 | base、hover、active、focus、disabled 合成次序与旧实现一致。 |
| renderer resolve (`renderer.rs:4529` 附近) | 从不可变节点 Style 解析出本次渲染的 StyleProperties | 主题、尺寸解析或 pseudo 处理没有反写共享 Style。 |

对 `.style` 赋值、`.style.base`、mutable getter、`Rc::get_mut/make_mut` 的定向搜索表明：除构造器初始化外，UiNode 内样式数据实际修改集中在表中的 presentation merge 与 ghost 清理两处。其余公开 Style 的可变字段属于独立的 owned Style 值，不能穿透 `&Style` 修改节点。

没有引入 `RefCell<Style>`；Style 本身也不反向引用 UiNode/Entity。默认样式共享不引入新的引用环或跨线程可变共享。

## owned snapshot 与 ghost 的关键不变量

`apply_presentation_mutation` 先调用 snapshot.activate，再应用 mutation。激活时保存的节点 clone 共享当时的 immutable Style；随后的 `Rc::make_mut` 会分离，故外部 presentation 不会污染组件 owned base。

`hydrate_component_subtrees` 保存 caller presentation，复制当前 owned snapshot，再逐个重放 presentation。重放仍经过同一 COW merge，更新后的组件 base 与调用者样式可以组合，同时保留旧树快照。上面的第三个独立探针实际验证了这一流程。

snapshot 的反向链接仍是 Weak；本 PR 没把它改成强引用。样式变更不改变组件 owner、callback lease 或 snapshot 生命周期。

ghost 路径静态确认：先 clone，后以 COW 清除 hit-test/cursor；root 还清除 margin、position、四条物理 inset、两条新逻辑 inset、translate 和 flex 相关字段。新增 inset_start/inset_end 已一并清理，避免 ghost root 在 portal/overlay 几何中再次套用源布局边。子树仍按既有规则递归处理。这部分本子审查采用写路径检查及产品新增单测代码审查，没有声称独立执行了私有 ghost 单测。

## 逻辑 inset 与栈整改评价

`inset_start/inset_end` 已覆盖 StyleProperties 字段、merge、Rust/Rhai builder、auto/signed/definite 输入、theme length 解析，以及 renderer 的 LTR/RTL 映射。逻辑边优先于对应物理边的规则有明确注释，并复用 margin/padding/border 使用的方向解析函数；没有给 Table 单独创建第二套方向模型。

Style COW 是合理的数据表示整改：大型 Style 不再随每个 UiNode clone 被内联搬过递归调用帧，同时保持节点值语义。它不是只把测试线程栈放大。实施材料记录 `UiNode=592 bytes, Style=5680 bytes` 及默认栈 Gallery 用例通过；这些数据在本报告中属于实施证据，根审另外负责当前 HEAD 的 Gallery 默认栈验收。

结构体尺寸本身不证明所有可能树深度都不会溢栈。保留实际 Gallery/虚拟化组合用例及既有资源深度约束仍然必要。本次未发现因 COW 造成的 alias、snapshot replay 或 ghost 污染，不建议为这次私有表示变化另加兼容层。

## PR #102 geometry 的静态复核

按分工没有重复运行所有旧 geometry suite：

- PanZoom wheel 和 keyboard centered zoom 现在读取 `canvas_bounds(content_ref)`；外层 viewport ref 仅继续承担外部区域等用途。R5 的普通非对称 padding 根因在调用链上已经处理。
- `Affine2D::inverse` 先按线性部分最大绝对系数归一化，再判定 determinant，避免仅因整体 scale 很小而把正常矩阵当成奇异。它仍拒绝零矩阵、近奇异或不能产生有限逆的输入。R5 的绝对 determinant 阈值根因已修；这里不宣称任意病态矩阵或无界精度能力。
- 新 Style 存储通过只读 getter 进入原有 drawable/geometry 解析，没有发现把 Rc allocation identity 当作几何 revision 或节点相等性的路径；UiNode/Style 相等仍按值比较。

## 验收边界

本结论限定为 Style/COW、owned snapshot、ghost/inset 写路径与 geometry 静态衔接。窗口 ownership、Table 实际拖动边界、完整 Gallery 默认栈和固定 viewport 美观截图由其他分工验收。不能将本报告的 3 个小型正向测试替代整项 PR 的发布验收。
