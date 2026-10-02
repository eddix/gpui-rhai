# 0.1.8 第三轮对抗审查：变换、选择与 wheel 取消

基线：`815bb82b8103c88ef6ba50620128780c9d64b216`，对比 `d77b4b49`。以当前 ADR 0022 为准。没有修改产品源码、正式 tests 或旧审计；不要求任意 GPUI 子树变换。

本轮用六个新原生场景确认 **2 个 P1 整改组、1 个 P2 整改组**。所有失败均由实际状态/几何断言证明，不是纯源码推测。旧探针由主审复跑，本目录没有重复旧案例。

## 证据及复跑

- [探针源码](src/lib.rs)
- [Cargo manifest](Cargo.toml)、[锁文件](Cargo.lock)
- [最终原生日志](native-probes.log)

在仓库根目录复跑，复用已有 native 构建缓存：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round3/transforms-selection/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

最终结果 **0 passed / 6 failed**：测试断言应有行为，因此失败是当前基线存在缺陷的证据。首次使用了本 agent 独立的临时构建目录；进程结束后已清理该一次性构建产物，保留源码、锁文件和最终日志。正式复跑请使用上面的缓存目录。

## R3-TS-1 / P1：PanZoom 局部 Escape 修复绕过了真正的交互 owner

定位：[pan_zoom.rs:359](../../../../crates/gpui-rhai/src/pan_zoom.rs#L359) 至 365。新增分支始终清理当前键盘焦点控件的 wheel signals，并且无条件 stop propagation；既不确认它是否拥有要取消的操作，也不取消 pointer coordinator 中的 active pan。

该修改让上一轮“同一个 PanZoom 持有焦点的 wheel Escape”通过，但产生/保留以下三个真实问题。

| 新探针 | 操作 | 实际结果 | 期望 |
|---|---|---|---|
| `round3_escape_cancels_pointer_pan` | pointer pan 拖动 → Escape → 继续移动 → mouseup | Escape 后 tx 暂时归零；下一次 move 又变成 **70**，最后提交 **1** 次 | Escape 终结该 pointer session，后续 move/up 不得再改变/提交 |
| `round3_wheel_owner_cancels_when_other_control_has_focus` | 点击 Second 取得焦点；在 First 上 control+wheel；Escape | First scale 仍 **1.1051709180756477**，随后 Ended 提交 **1** 次 | 取消实际拥有 wheel session 的 First，而非只清理有键盘焦点的 Second |
| `round3_idle_panzoom_does_not_swallow_dialog_escape` | Dialog 内点击 idle PanZoom 取得焦点；Escape | `dialog open=true` | 没有可取消交互时，Escape 应继续到 Dialog 的 dismiss 路由 |

第二个场景是常见的 wheel 与 keyboard focus 分离，不是非法输入：wheel 命中目标通常并不自动夺取键盘焦点。

整改要求：

1. 把 pointer pan、explicit wheel 和延迟 wheel commit 的 owner/取消纳入统一或明确协作的 window/Host 入口。
2. Escape 必须先确认并终结实际拥有的 session，包括 pointer coordinator 状态、wheel active/pending 和 delayed generation；不能只把预览信号临时写回 source。
3. 只有真正消费了取消动作时才 stop propagation；idle 控件不能阻断对话框/父级的 Escape。
4. 回归同时覆盖同控件焦点、异控件焦点、无活动手势、pointer pan、wheel、迟到 move/up/Ended 和定时提交。

## R3-TS-2 / P1：Rotatable 仍混用 wrapper 几何、Canvas 几何与手势开始时的旧几何

定位：

- [rotatable.rhai:71](../../../../registry/components/rotatable.rhai#L71) 至 75：content part style 应用到 Canvas，而 `content_ref` 挂在固定 100% 大小的外层 wrapper。
- [rotatable.rs:135](../../../../crates/gpui-rhai/src/rotatable.rs#L135) 至 152：mouse-down 时读取一次 viewport 并捕获进 update 闭包；之后 move 一直使用旧 viewport。
- [rotatable.rs:424](../../../../crates/gpui-rhai/src/rotatable.rs#L424) 起：presentation token 添加宽高只覆盖静态同步，不会重建正在运行的 gesture 几何。

### 新场景 A：正式 content part 覆盖尺寸

300×220 Rotatable，`angle=90`，`pivot=(0,0)`；通过正式参数 `part_styles:#{content:style().width(px(200)).height(px(100))}` 修改 Canvas 内容尺寸。

原生测量确认 Canvas 确实为 **200×100**，但补偿仍为 **`(-258,40)`**，来自 wrapper 的 298×218 尺寸。实际绘制中心要求的补偿是 **`(-150,50)`**。声明 pivot 因此偏移约 **`(-108,-10)`**。

探针：`round3_pivot_uses_canvas_content_part_size`。它先断言实际 Canvas 尺寸已改变，再断言补偿，避免把无效样式 fixture 当成产品缺陷。

### 新场景 B：正在旋转时真实窗口 resize

开始 pointer rotation，随后通过 GPUI TestWindow 将窗口宽度缩小，再继续移动同一 pointer。

实际 Canvas 宽度 **1918 → 598**，继续移动得到 angle **78.57881372500071°**，但 preview 写入旧尺寸对应的 `tx=-875.940594059406, ty=852.5940594059406`。按当前实际 Canvas 的旋转矩阵投影，声明 pivot 偏移达到 **`(-529.3069306930693, +646.9306930693069)`**。

探针：`round3_resize_during_rotation_preserves_live_pivot`。检查发生在手势仍 active 的预览帧；不是 mouseup 后的 source 恢复帧。

整改要求：以绘制实际使用的 Canvas 内容矩形和当前 presented transform 为唯一事实源；wrapper 只是容器，不能代替可被 part styles 独立改变的 Canvas。几何变化必须让 active gesture 按明确规则取消或 rebase，并使之后的 update/finish/cancel 都使用同一有效几何版本。仅给 source token 拼接两个尺寸不会更新已经捕获旧 viewport 的闭包。

验收增加 content part 宽高、边框/内边距/偏移、静态 resize、active resize、ancestor 布局/滚动改变；验证 pivot 的实际投影不动性，而不仅检查信号是否“有非零补偿”。

## R3-TS-3 / P2：退化 polygon 的固定面积 epsilon 不具备缩放不变性

定位：[selection_area.rs:513](../../../../crates/gpui-rhai/src/selection_area.rs#L513) 至 528。

上一轮为处理零面积 marquee 加入 `twice_area.abs() <= 1e-9`，但这个面积是在 **Canvas-local 坐标**中计算的，未考虑从窗口到内容的缩放。

触发：支持的 Canvas 缩放 `1e6`；局部目标矩形大小 `0.000005 × 0.000005`，最终画面上是清晰的 **5×5 逻辑像素**；用户拖出 **20×20 逻辑像素**的 `enclose` 框，完整包住它。

预期：选中目标 a。用户框出的屏幕区域不是退化区域。

实际：`selected=[]`。inverse 后 marquee 的 twice-area 约为 `8e-10`，被固定阈值判为退化。日志中真实目标投影尺寸为 `4.9999999873762135 × 5.000000001587068`，确认目标并非小到不可见。

探针：`round3_enclose_remains_valid_at_supported_large_zoom`。

整改要求：在明确定义的窗口逻辑像素空间判断退化，或使用尺度感知的容差与归一化几何计算；不能继续以一个固定内容面积 epsilon 代替几何契约。回归使用同一个屏幕选区，在不同合法 content scale/坐标单位下保证相同选择结果，同时保留真实零面积及延长线误选防护。

## 模型层结论

当前整改仍以具体输入路径打补丁：在 PanZoom 焦点处理器里增加 Escape、用外层 wrapper 代替 handle、给 polygon 加固定 epsilon。每个改动解决了之前的一个样例，但没有统一 owner、绘制几何和坐标单位，新的组合立即绕过原来的验证。

建议开发先分别明确三条不变量，再修改实现：

- 取消作用于实际拥有交互的 owner；没有 owner 时事件继续传播。
- pivot、paint、hit 和 active gesture 消费同一版本的实际 Canvas 几何。
- 选择结果对支持范围内的坐标单位与显示缩放保持一致。

本轮没有新增随机变换或任意子树支持要求；全部案例处于已暴露的 Canvas、part styles、window resize、focus、pointer/wheel 和 Dialog 组合能力内。
