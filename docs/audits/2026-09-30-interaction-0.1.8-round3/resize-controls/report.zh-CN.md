# 0.1.8 第三轮对抗审查：控件身份、范围端点与取消语义

- HEAD：`815bb82b8103c88ef6ba50620128780c9d64b216`；对比整改前 `d77b4b49`。
- macOS aarch64，Rust 1.95.0，gpui-pre 0.3.7。
- 沿用修订 ADR 的提议／清理契约，不要求旧 awaiting-control 模型。
- 原 25 项和第二轮 probes 由父审查统一复跑；本目录仅新增反例与正向控制。
- 未修改产品、正式测试、旧审计或 GitHub。独立 Cargo package/test target：`audit-resize-controls-round3` / `resize_controls_round3_probe`。

## 结果

[原生探针](probe.rs)及[完整日志](probe.log)：6 项，其中 3 项失败、2 项产品正向控制通过、1 项仅在 probe 内改正身份键的诊断对照通过。

[生产数值函数性质探针](math-probe.py)及[结果](math-probe.log)：抽取当前生产函数原文，覆盖 40,114 个合法输入组合，bounds、gap、幂等性检查通过（浮点容差分别为 1e-9/1e-8）。该结果证明此次 pair 求解改进在这组范围内有效；它不能证明 pointer/key 的另一套求解路径同样正确。

## R3-RC1 / P1：RangeSlider native 身份与 retained 身份不同，每次绘制丢失 Entity 和焦点

这是本轮最需要先修的核心问题，与本次新 range 数学无关，属于此前没有识别的身份模型缺陷。

### 两套身份键

- `registry/components/range_slider.rhai:62`：构造 `RangeSliderPrimitive` 时 native key 是 `props.key`。
- 同文件 `:74`：随后调用 `.with_key(`${props.key}-range`)`，改变了 retained 节点的 key。
- `node.rs:413`：Key presentation mutation 仅改 `UiNode.key`，不改 `PrimitiveNode.key`。
- `primitive.rs:1675-1683`：native mount 身份使用 `PrimitiveNode.key`。
- `primitive.rs:1837-1847`：`retain_tree` 的存活集合使用 `RetainedNode.key`。

即使 NodeId 没有发生逻辑卸载，registry 也会在下一次 reconcile 认为 native 实例不在树中，调用 RangeSlider handler.unmount 删除内部 Entity；下一次 render 重建新的 Entity/FocusHandle。

### 原生行为证据

`range_keyboard_focus_survives_native_repaint`：

1. 对公开 RangeSlider 逐次 focus_next 后仅请求原生刷新，不改任何 source/state。
2. 每次刷新前有焦点，刷新后焦点已经失效：

```text
frame 0: FocusId(5v1) -> None
frame 1: FocusId(4v5) -> None
frame 2: FocusId(6v7) -> None
```

3. 点击 high=80 thumb 后按 Right，观察到的最后一次提议仍为 `[20,80]`，没有 `[20,90]` 键盘提议。

诊断对照 `range_equal_native_and_retained_key_diagnostic_control` 只在探针加载的字符串中把 native 构造 key 改成 `${props.key}-range`，与紧随其后的 retained key 相同；没有改磁盘上的组件或 Rust 实现。其他代码、主题、输入序列均一致：

```text
frame 0: FocusId(3v1) -> FocusId(3v1)
frame 1: FocusId(4v1) -> FocusId(4v1)
frame 2: FocusId(3v1) -> FocusId(3v1)
click high80 -> Right: [20,80] -> [20,90]
```

它说明焦点丢失来自两套身份不一致，不是测试未聚焦、End 键映射或业务 callback 拒绝。

### 修复要求

在 mount、retain、dispatch、accessibility projection 和 unmount 使用同一个 primitive 实例身份来源。不能只在 RangeSlider 文本里把两个 key 改成一样就认为公共 `.with_key` 问题消失：公开 keyed primitive 经过 caller key presentation mutation 后，仍应有确定的身份与资源生命周期。

测试应覆盖“native key 与外层 node key 相同／不同”、无状态 repaint、Host/主题 repaint、正常脚本 rerender、真正卸载重建，并断言 mount/unmount 次数、FocusHandle 生存与 keyboard callback。不要再只根据 pointer callback 次数断言 keyboard 成功。

### 同源静态排查范围（不冒充新增原生复现）

| 组件 | native 构造 key / retained `.with_key` | 当前生命周期意义 |
| --- | --- | --- |
| PanZoom | `${key}-interaction` / `${key}-pan-zoom`，`pan_zoom.rhai:78,97` | `pan_zoom.rs:881` lifecycle=true，handler 按 PrimitiveInstanceId 缓存 Entity，并在 unmount 删除；与 RangeSlider 有相同身份不一致路径，应一并修复和验证 suspend/resume/wheel/pan |
| Rotatable | `${key}-rotation` / `${key}-rotate-interaction`，`rotatable.rhai:56,64` | `rotatable.rs:519` lifecycle=false 且无 state schema，不进入此 retained mount Entity 路径；不据此断言当前也逐帧重建 Entity |
| Draggable | `${key}-interaction` / `${key}-drag-interaction`，`draggable.rhai:75,85` | 当前 lifecycle=false，适用同一键策略检查，但没有实测到 RangeSlider 式 Entity 丢失 |
| SplitPane | `${key}-handle` / `${key}-separator`，`split_pane.rhai:132,142` | 当前 native lifecycle=false；不能因为现在没有 retained state 就忽略公共身份契约 |

PanZoom 暂无独立完成的生命周期反例，本报告只把它列为同根因受影响路径，不报告“恢复后一定不能 pan”。修正范围应落在通用 PrimitiveRegistry 身份契约；RangeSlider 一行字符串替换只是定位实验。

现有 `tests/native-keyboard/tests/control_visuals.rs:1299-1305` 在第二次 pointer click 后发送 Left，但仍只断言总 commits=2；两个 pointer 操作已经能产生 2 次提交，因此这个断言不能证明键盘产生过提交。应断言键盘后的新增次数／具体值及焦点存活。

## R3-RC2 / P2：RangeSlider 已接受的非整步端点与 pointer 求解不一致

这里不要求所有设计必须允许网格之外的 max。问题是当前实现的两条路径已经选择了不同规则。

**合法已接受 source**：`values=[20,97], min=0, max=97, step=10, minimum_gap=5`。

- `normalize_pair_values` 通过公共 `normalize_value` 保留 `[20,97]`；max 是这里已经被支持的特殊端点。
- 在当前 high thumb 所在的最右端点击、不做实际拖动，pointer 却提议 `[20,90]`。
- 同样输入序列在 `max=100` 的正向对照提议 `[20,100]`。

**位置**：`range_slider.rs:192-200` 将 pointer high 送入 `normalize_constrained_value`；`:683-687` 先吸附为 97，再以最后一个整网格值 `floor(97/10)*10 = 90` 为上界，把它压回 90。

**修复要求**：pair normalization、pointer、keyboard/Home/End 明确共享同一合法值集合。若端点是允许的例外，就必须把 max 纳入 constrained solver；若不允许，schema／受控 source normalization 与文档也要一致。不能出现已经原样呈现的合法值在点击原位置时被另一条求解路径改掉。

探针：`range_pointer_agrees_with_the_accepted_off_grid_endpoint`。未将探索阶段未送达的 End 输入误报为此项证据；键盘焦点缺陷单独归于 R3-RC1。

## R3-RC3 / P2：Table 无 callback 模式第二次 resize 取消，清掉第一次已经接受的尺寸

这轮整改为没有 `on_column_resize` 的 Table 保留 native override，从而明确存在本地接受宽度的模式。但是同一个 signal 仍同时代表“已接受宽度”和“当前手势 preview”。

**实测序列**：

1. 声明初始 width=160，不提供 resize callback。
2. 第一次 drag +40 并释放：separator x149→189，200px 宽度正确保留。
3. 开始第二次 drag +20，再按 Escape 取消：separator 从本应恢复的 x189 跳回 x149，第一次已经接受的 200px 也被抹掉。

原生探针显式聚焦 ScriptView 后驱动真正 Escape 路由，确保不是测试窗口没有焦点导致取消没有执行。

**位置**：`column_resize.rs:289-291` 只在有观察 callback 时清理完成后的 override；`:293-298` 取消却无条件写 `None`。清理 current preview 时不知道此前是否存在一个已经接受的本地尺寸。

**修复要求**：分开当前 gesture preview 与本地 committed width，或在手势开始时记录原 override，取消时恢复它。取消回滚的起点应是第二次手势开始前的状态，不能笼统回到最初 source props。覆盖 first resize→second cancel、keyboard/autofit 后 pointer cancel、source 更新后的取消，确认新 source 仍优先。

探针：`table_uncontrolled_cancel_restores_last_accepted_width`。

## 本轮正向证据与边界

- `range_aligned_endpoint_control`：正常整步端点输入工作。
- `resizable_keyboard_uses_current_boundary_after_window_resize`：窗口宽从 500 缩到 305 后，x=100、width=200 的 east keyboard +8 被正确约束为 width=205，没有使用旧 500px 边界。
- 40,114 组 pair 参数包含不同 min、span、小数 step、非整倍数 gap 与完整范围 gap；没有发现 bounds/gap/幂等性反例。这是有限覆盖，不是形式化证明。
- 相同 key 的 RangeSlider 是故障定位对照，不是产品已通过的证明。

## 重现

从仓库根目录执行，重用缓存但使用本轮唯一 test target：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round3/resize-controls/Cargo.toml \
  --test resize_controls_round3_probe -- --nocapture --test-threads=1

python3 docs/audits/2026-09-30-interaction-0.1.8-round3/resize-controls/math-probe.py
```

数学探针记录原源文件 SHA256，只提取指定函数，使用独立、唯一临时目录编译，退出后移除临时二进制。旧审计材料保持不变。
