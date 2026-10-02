# 0.1.8 第五轮：共享 Canvas geometry 的剩余消费者

基线：`23fce23829c6694c3e077a61a20a9038b87f0759`，对照 `f6e936a5` 与更新后的 ADR 0022。

本子域确认 **1 个需要修复的 P2**：PanZoom 指针锚点仍使用外层容器几何。另记录 **1 个 P3／非阻断数值能力边界**，不计入本轮主要发布阻断数量。新的实际 drawable、SelectionArea inverse 和 suspend Runtime lease 在本轮三个正常交叉场景中表现正确。

没有修改产品、正式测试、历史审计或别人临时目录。全部构建复用 `tests/native-keyboard/target`，没有新建大型缓存或 profile。

## 证据与复跑

- [原生交互源码](src/lib.rs)、[原生日志](native-probes.log)：**3 passed / 1 failed**。唯一失败对应下述非阻断小缩放边界。
- [PanZoom 实际 paint 源码](src/bin/round5_pan_geometry.rs)、[paint 日志](paint-geometry.log)：对称 inset 的锚点正对照通过；非对称 inset 锚点断言失败。
- [Cargo manifest](Cargo.toml)、[锁文件](Cargo.lock)。

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round5/transforms-selection/Cargo.toml \
  --locked --offline --lib -- --nocapture

CARGO_TARGET_DIR=tests/native-keyboard/target cargo run \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round5/transforms-selection/Cargo.toml \
  --locked --offline --bin round5_pan_geometry
```

paint 探针在 macOS 主线程创建 HeadlessAppContext，向实际 GPUI Window 投递 ScrollWheelEvent，记录产品 Canvas renderer 生成的 CPU Scene path bounds。测试 renderer 只记录 Scene，不做 GPU 光栅化；返回的占位图像没有用于判断或作为截图保存。结论来自实际绘制路径，而非只读变换 signal。

## R5-TS-1 / P2：PanZoom 未接入实际 drawable，普通非对称 padding 会使缩放锚点漂移

定位：

- [pan_zoom.rs:233](../../../../crates/gpui-rhai/src/pan_zoom.rs#L233) 至 256：wheel 仍读取 `element_bounds(viewport_ref)`，以外层 origin 和 width/height 构造 zoom anchor 与缩放中心。
- [pan_zoom.rhai:67](../../../../registry/components/pan_zoom.rhai#L67) 至 95：传入的是 root viewport ref；content 可单独应用 part style，但没有向 primitive 提供对应 drawable ref。

Rotatable 和 SelectionArea 已改用新 Canvas drawable，PanZoom 的 viewport 数学仍使用旧外层盒子。当 Canvas drawable 与外层容器不共用中心时，指针锚定缩放的假设失效。

### 真实 paint 复现

300×220 PanZoom，初始 transform `(x=0,y=0,scale=1)`。Canvas 中放一个 10×10 marker，把指针定位在 **实际绘制的 marker 中心**，发送将 preview 缩放到约 2× 的 wheel Started。

| content padding（左、右、上、下） | 指针／marker 原中心 | 缩放后实际 marker 中心 | 漂移 |
|---|---|---|---|
| `(20,20,20,20)`，正对照 | `(106,66)` | `(106,66)` | `(0,0)` |
| `(40,0,20,0)` | `(126,66)` | `(106,56)` | **`(-20,-10)`** |

预期：两种样式下，被指针锚定的 content 点都应停留在同一屏幕位置。实际：普通非对称 padding 已造成可见跳动；不涉及极端数值、任意 GPUI 子树或大量对象。

### 整改要求

让 PanZoom 的指针 zoom、键盘 centered zoom 及后续 fit/reset 的坐标定义使用与 paint 相同的 drawable/origin/transform。沿用本轮已经引入的 Canvas geometry 能力，不在 PanZoom 再手工猜 padding。viewport 的事件命中范围可以继续是外层区域，但事件命中区域不等于缩放数学的 Canvas drawable。

验收应包含对称与非对称 padding/border、content part 尺寸覆盖、ancestor 偏移、已有 pan 值及最小/最大缩放夹取；直接比较缩放前后指针下对象的实际绘制位置。

## R5-TS-2 / P3：很小但可逆的合法 scale 被绝对 determinant 阈值当成奇异矩阵

定位：[geometry.rs:208](../../../../crates/gpui-rhai/src/geometry.rs#L208) 至 216。

`inverse()` 使用 `abs(determinant) <= f64::EPSILON`，因此均匀 scale=`1e-9` 的 determinant=`1e-18` 被拒绝，尽管该矩阵可逆且条件并不差。

本轮使用公开 Canvas constant Motion + SelectionArea，把 Canvas-local 对象尺寸与缩放成反比调整；按 forward affine 计算，最终目标尺寸均为 **5×5 逻辑像素**，再向对应中心投递真实 pointer 输入：

| Canvas scale | 最终目标尺寸 | inverse | 原生点击结果 |
|---:|---:|---|---|
| `1e-7`，正对照 | 约 `5×5px` | 可用 | `["a"]` |
| `1e-9` | `5×5px` | None | `[]` |

公开 Canvas 校验没有把这组有限值拒绝为非法；PanZoom 自定义 min_scale 同样只有正数约束，但本探针直接使用 Canvas Motion，没有声称已测试两种组件嵌套。对象数量固定为两个，不涉及大规模渲染。

**优先级说明：** `1e-9` 是极端自定义缩放，远超默认 PanZoom `0.25..8` 的普通用途。本项作为数值能力边界记录，不独立阻断 0.1.8，也不要求本轮承诺任意尺度世界坐标。后续可采用尺度归一化/相对条件的逆矩阵判断，或者明确并统一校验支持的最小 scale；不要一面接受输入，一面在 inverse 层隐式丢弃交互。

## 本轮正常交叉验证通过

1. **非对称 inset + 负缩放 + 非均匀缩放 + 旋转 + 非整数 DPI。** Canvas 分别设置 left/right/top/bottom padding `17/3/13/5`，border `2/4/3/1`，scale `(-1.5,0.75)`、rotate `30°`、窗口 DPI scale factor `1.25`。原生点击实际目标中心，正确选中 a。该结果支持新 drawable inverse 的正常组合行为。
2. **两轮 active rotation → suspend → resume → resize。** 每轮先确认真实原生 preview 从 source `20°` 改为约 `85.9892°`，suspend 后恢复 `20°`，resume 后分别把窗口改为 600、900px 宽；迟到 mouseup 均不提交。两轮累计 `commits=0`，未出现 Runtime/Entity 重入异常。
3. **小缩放的较保守正对照。** scale=`1e-7` 时，上述相同屏幕尺寸的目标点击正确。
4. **实际 paint 的对称 inset 正对照。** wheel 2× 后 marker 不漂移，排除了无效事件或未缩放导致的假阳性。

这些测试包含组件现有 root clip 下的正常可见对象；没有把它们宣称为复杂多层裁剪、任意缩放或所有生命周期组合的完整证明。主审另外复跑历史缺陷探针。
