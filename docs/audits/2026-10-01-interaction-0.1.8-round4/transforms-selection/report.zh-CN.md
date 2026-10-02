# 0.1.8 第四轮：实际 Canvas 绘制几何与选择容差

基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`，对比 `815bb82b`，按当前 ADR 0022 审查。

确认 **1 个 P1、1 个 P2**。本轮只增加新的 padding/border 和较大 Canvas 数值边界，并包含两个正对照。主审负责旧探针回归；本目录未修改产品、正式 tests 或历史审计。PanZoom auxiliary/debounce 与跨窗口取消由其他审查分工负责，这里不重复列项。

## 证据与复跑

- [原生输入探针](src/lib.rs) / [执行日志](native-probes.log)：**1 passed / 2 failed**。
- [GPUI paint scene 探针](src/bin/paint_geometry.rs) / [执行日志](paint-geometry.log)：无 padding 的 0°/90°中心一致；有 padding/border 的中心不一致，断言失败。
- [Cargo manifest](Cargo.toml)、[锁文件](Cargo.lock)。测试 crate 名字唯一，复用 `tests/native-keyboard/target`，没有新建大构建缓存。

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-10-01-interaction-0.1.8-round4/transforms-selection/Cargo.toml \
  --locked --offline --lib -- --nocapture

CARGO_TARGET_DIR=tests/native-keyboard/target cargo run \
  --manifest-path docs/audits/2026-10-01-interaction-0.1.8-round4/transforms-selection/Cargo.toml \
  --locked --offline --bin paint_geometry
```

第二个程序在 macOS 主线程创建 HeadlessAppContext，用一个只记录 Scene 的测试 renderer 读取 **真实 GPUI 绘制路径 bounds**。没有改产品 renderer；它不做 GPU 光栅化，也不把返回的占位图像作为证据。结论来自实际 Canvas 绘制代码产生的 CPU Scene 路径位置。

## R4-TS-1 / P1：读取实际 UiNode ref 仍不等于读取 GPUI Canvas 的实际 drawable rect

位置：

- [renderer.rs:2997](../../../../crates/gpui-rhai/src/renderer.rs#L2997) 至 3004：真正绘制的是 styled element 内部的 `gpui::canvas(...).size_full()`，传给 `paint_canvas_scene` 的是这个子元素的 bounds。
- [renderer.rs:4340](../../../../crates/gpui-rhai/src/renderer.rs#L4340) 附近：GeometryTrackedElement 登记的是 styled UiNode 外层 layout/visual bounds。
- [app.rs:3428](../../../../crates/gpui-rhai/src/app.rs#L3428) 至 3437：Canvas inverse 从外层原点减坐标，并使用外层宽高构造矩阵。
- [rotatable.rs:408](../../../../crates/gpui-rhai/src/rotatable.rs#L408) 至 420：pivot 补偿同样使用所读 bounds 的宽高中心。

当 content part 有 padding/border 时，外层盒子与实际 drawable rect 的原点、尺寸不同。当前 ref 从 wrapper 改成实际 Canvas UiNode 只关闭了上一轮的 wrapper 尺寸问题，仍没有关闭内部绘制盒子这一层差异。

### 实际绘制证明：声明的 Canvas-local pivot marker 在旋转时移动

Canvas 设置 `width=200,height=100`，pivot 为 `(100,20)`，画一个中心同样为 `(100,20)` 的 10×10 矩形 marker。用正式 `part_styles.content` 改变 padding/border，分别呈现 0° 和 90°。

GPUI 实际 paint Scene 中的逻辑像素中心：

| content 样式 | 0° marker 中心 | 90° marker 中心 |
|---|---:|---:|
| padding=0,border=0（正对照） | `(101,21)` | `(101,21)` |
| padding=20,border=10 | `(131,51)` | `(71,51)` |

同一个 Canvas-local pivot marker 实际漂移 **60 逻辑像素**。带样式时 drawable rect 缩小并内移，但补偿及交互转换仍使用外层盒子。此结论不是从补偿信号单独推断，而是比较实际 GPUI Canvas path。

### 原生输入证明：SelectionArea 点不到可见目标

SelectionArea 的 Canvas content 同样设置 `padding=20,border=10`。目标 a 位于 Canvas command 坐标 `(20,20,10,10)`，其实际绘制中心需要加上 30px 的内移。原生点击该中心，实际 `selected=[]`，预期 `["a"]`。

探针：`round4_selection_hit_matches_padded_canvas_paint`。日志记录区域原点 `(1,27)`、点击点 `(56,82)`；点击位置正是 `origin + border + padding + target_center`。

### 整改边界

在 GeometryRegistry/Canvas 适配层明确记录实际 drawable rect、到 window 的变换和 clip，让 paint、命中、SelectionArea、Rotatable 读同一事实源。不能继续在每个组件补减 padding 或猜测 styled node 的含义。若某种 content style 确实不支持，应在公共边界明确拒绝，而不是接受配置后让绘制与选择使用不同坐标系。

这是现有 Canvas 内外盒模型在 0.1.8 新交互中的缺口，不要求任意 GPUI 子树缩放。验收至少覆盖 padding、border、非对称 inset、part 尺寸覆盖、滚动/偏移、不同角度，并同时检查 actual paint 与原生命中。

## R4-TS-2 / P2：新的相对面积 epsilon 取决于绝对位置，较大 Canvas 下仍错误拒绝有效选区

位置：[selection_area.rs:516](../../../../crates/gpui-rhai/src/selection_area.rs#L516) 至 530。

当前 `coordinate_scale` 取 polygon 顶点 **绝对坐标值**，然后以 `EPSILON * coordinate_scale² * point_count * 16` 判定面积是否退化。这个容差不仅随图形尺寸变化，也随坐标原点和 Canvas 中心位置变化；面积本身则使用易发生大数相减的未平移 shoelace 求和。

新原生对照：Canvas 外层为 1000×1000，内部宽高为 998，目标位于局部 `(499,499)` 附近。两个场景都呈现 **5×5px** 的目标，并用 **20×20px** 屏幕框完整 enclose：

| scale | 调整后的 Canvas-local 目标尺寸 | 最终画面目标尺寸 | 实际选中 |
|---:|---:|---:|---|
| `1e4`（正对照） | `0.0005×0.0005` | 约 `5×5px` | `["a"]` |
| `1e6` | `0.000005×0.000005` | 约 `5×5px` | `[]` |

探针分别为 `round4_large_canvas_lower_zoom_control`、`round4_large_canvas_high_zoom_enclose`。

在第二个场景里 inverse 后 twice-area 约 `8e-10`，绝对坐标约 499 使 `area_epsilon` 增长到约 `3.54e-9`，于是用户可见的正常区域再次被判退化。两个场景使用相同实际屏幕面积与相同显示大小对象，差异由数值判定引入。

整改：对 polygon 先平移到相对原点，用局部边向量/跨度计算面积和容差，或在窗口逻辑像素空间完成已经明确的退化判断，并让后续包含/相交谓词保持尺度一致。不要将“相对绝对坐标的误差阈值”误当作图形平移与缩放不变的几何判断。还应同步检查下方 cross 判定仍使用的固定 `1e-9`，但本报告没有把未单独复现的 cross 误判另列 finding。

本轮证明上一轮 high-zoom case 的修复仍依赖具体 Canvas 尺寸。可靠验收应保持同一屏幕选区/对象尺寸，在不同 Canvas 大小、原点与合法缩放下做一致性比较。
