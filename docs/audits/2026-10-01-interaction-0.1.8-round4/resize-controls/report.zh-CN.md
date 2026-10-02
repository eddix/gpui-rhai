# 0.1.8 第四轮验收：RangeSlider / Table / primitive identity

- 基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`，对比 `815bb82b`。
- 环境：macOS aarch64 / Rust 1.95.0 / gpui-pre 0.3.7。
- 本分支只新增本目录；未修改产品、正式测试、旧审计或其他会话临时文件。
- 遵循现行 ADR 0022：source/constraint 变更取消旧手势；提议收尾清理；没有重新要求旧 awaiting-control 模型。

**结论：本轮负责范围内没有确认新的缺陷。新增 6 个原生交叉场景全部通过。** 这不是整个 0.1.8 的出货结论；历史复现与其他子系统由总审查汇总。

证据：[原生探针](probe.rs)、[执行日志](probe.log)。没有注入修改后的组件源；所有场景都加载当前正式 registry 组件。

## 对整改实现的判断

1. `primitive.rs:1694-1702` 使用 presented retained key 构造 native instance，已与 retain_tree 的身份来源对齐。它在公共 primitive 边界修正了身份分叉，不是只改 RangeSlider 的一行字符串。RangeSlider 新增真实键盘场景通过，未再出现上轮焦点绘制后失效的症状。PanZoom 同样受益于这个通用修复；本分支没有扩大声称 PanZoom 全部生命周期行为已复验。
2. `range_slider.rs:683-685` 优先保留落在可行域内的 normalized 值，避免再次把合法特殊端点压回普通网格。horizontal 与 vertical 的实际 End 输入均得到 97，与已接受的 max=97 特殊端点一致。
3. `column_resize.rs:251` 保存手势开始时的 override，`:294-300` 在取消时恢复它。新增 source 替换与约束收紧场景验证了旧 rollback 不会反过来覆盖新契约。

## 新增原生场景

| 场景 | 实测结果 | 判断 |
| --- | --- | --- |
| RangeSlider horizontal，min0/max97/step10/gap5，先点击 high80 再发送真实 End KeyDown | 提议 `[20,97]` | 通过：键盘与特殊端点规则一致，native focus 跨重绘有效 |
| 同参数 vertical RangeSlider，先按实际 vertical 几何聚焦 high，再 End | 提议 `[20,97]` | 通过：两个 orientation 使用同一数值域 |
| RangeSlider 不提供 on_change，点击 high80 聚焦，再 Right | 两个真实绘制 thumb 的 x 始终 `[82,370]` | 通过：未被接受的键盘操作没有永久变成 native source |
| RangeSlider 的 on_change 为纯 noop，不改 state/source，再 Right | thumb x 同样保持 `[82,370]` | 通过：callback 存在与否没有改变受控拒绝语义 |
| 无 callback Table 先从160接受本地200；第二次拖动中将 source 改为180；释放旧手势 | separator x149→169，最终 width180 | 通过：新 source 优先于旧手势 rollback=200 |
| 无 callback Table 先接受本地200；第二次拖动中将 max_width300收紧至180；释放旧手势 | separator 回到x149，即当前声明source160，满足新max180 | 通过：旧accepted200和旧preview230均未越过新约束继续呈现 |

thumb 断言来自 `Window::painted_quads()` 的真实边框绘制坐标，不只看 retained props。探针检查恰好存在两个 thumb quad；坐标是本测试窗口的绘制坐标，不冒充可移植的视觉 baseline。

Table 的两种变更通过公开 Automation Dispatch 调用已挂载按钮 handler；手势由 GPUI 的实际 down/move/up 驱动。它们覆盖“已经存在一次本地 accepted override”的场景，而不是只在第一次空 override 上验证取消。

## 覆盖边界

- 原第1—3轮探针由总审查统一重跑，本分支没有重复执行或覆盖其旧日志。
- 本轮新增端点是 max97/step10/gap5，并覆盖 horizontal/vertical；没有宣称穷举所有浮点、任意 min 或任意 step 的特殊端点策略。
- 本轮没有新增 RTL、所有 modifier、真实 OS accessibility 或截图基线测试。当前焦点与按键验收使用 GPUI 原生测试上下文。
- Table 约束变更后回到显式 source160，而不是自动保留为180；本轮验收断言的是“遵循新的最大值并消除旧 override”，没有另行规定其产品策略必须选择180。
- 未承诺框架自动理解业务层异步应答先后顺序；测试的是当前 ADR 定义的原生手势、source 替换和 preview 清理边界。

## 复现

从仓库根目录执行；唯一 package/test target 使用共享依赖缓存，不新建大型 target：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-10-01-interaction-0.1.8-round4/resize-controls/Cargo.toml \
  --test resize_controls_round4_probe -- --nocapture --test-threads=1
```

结果：`6 passed; 0 failed; 0 ignored`。
