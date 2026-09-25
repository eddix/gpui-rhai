# 补充：gpui-pre 归属与 ShaderSurface／3D 选型权重

日期：2026-09-24。沿用主报告的精确源码基线，未实施迁移。

**后续范围更新：** 用户已决定 3D 引擎可由 Rust Host／第三方扩展承担，gpui-rhai 不必内置。本文件的 CE 倾向适用于“内置跨平台 3D 是一等产品目标”的条件，不再作为当前项目的选型结论。现行建议见 [上游核心与 Kit 收益拆分](upstream-core-vs-kit.zh-CN.md)。

## 1. gpui-pre 是独立 crate，但不是 Zed 官方维护的发布包

crates.io owner 查询为 `huacnlee`（Jason Lee）；0.3.6 的 `published_by` 同样为 `huacnlee`。它属于 GPUI Kit 侧的上游快照发布链路。

- Zed 维护原始 GPUI 实现与官方 `gpui` crate。
- Kit 侧把上游特定 commit 整理为 `gpui-pre-*` 版本家族，并负责该发布链路。
- gpui-pre 的作者字段、repository 字段仍指向原作者／上游，不能据此判断发布包由 Zed 运营。
- gpui-pre 是独立可依赖的 package，不要求应用必须使用完整 Kit；其定位也不是一个独立改造核心行为的 fork。

来源：[crate owners](https://crates.io/api/v1/crates/gpui-pre/owners)、[0.3.6 发布者](https://crates.io/api/v1/crates/gpui-pre/0.3.6)、[Kit 维护政策](https://github.com/longbridge/gpui-kit/blob/477a8d90bb318cfbf0bbdc0f29517d437ad003e1/CONTRIBUTING.md)。

本次也更正主报告的一点：当前 pinned CONTRIBUTING 明确说快照会报告 Kit 构建／测试结果，但无论验证结果如何都会发布。Kit 应用自身靠精确 pin 选择底层版本。不能将“存在验证过程”表述成“验证必过才发布”。

## 2. 3D 并非临时增加的想法，应纳入本次选型

现有 [ADR 0020](../../adr/0020-motion-runtime-v2.md) 已分出三个纵向层次：

1. Motion：属性、时钟、timeline、手势与生命周期。
2. Effect primitives：有界的 2D Canvas/path/particles 等。
3. ShaderSurface：未来跨平台 typed GPU-effect／3D。

ADR 同时排除了逐帧 CPU 生成完整 RGBA 图像的方案，并要求 Host 注册的 GPU primitive 声明平台、schema、预算、主题、motion policy 和 scoped lifecycle。

因此，上轮主要按成熟桌面控件比较而优选 Kit，对完整路线的权重不充分。**若 ShaderSurface／3D 是确定的一等目标，长期底座应更偏向 CE。** 通用控件更完整和核心 GPU 扩展更合适，是两个不同的评价维度。

## 3. CE 提供的是重要的 GPU 接入底座，不是完整 3D 引擎

CE 的 custom GPU API 提供：

- 复用 GPUI 的 WGPU device/queue。
- 创建可合成的离屏 render target。
- 通过 GPU surface 纳入 UI 布局和合成。
- resize、设备失效标志及资源重建基础。
- 额外设备 feature/limit 请求。

这些能力直接对应 ShaderSurface 最难的 UI/GPU 接缝。[实现与设计](https://github.com/gpui-ce/gpui-ce/pull/237)

我们的 3D 层仍需定义相机、投影、mesh、material、深度缓冲、透明度／MSAA、拾取以及资源和渲染调度。拥有 WGSL 或一个旋转三角形示例，不等于这些 3D 语义已经实现。

建议的结构是：

```text
Rhai 的 typed scene/effect 参数 + Host 注册的 shader/material
                         ↓
Rust ShaderSurface：场景、资源、预算、生命周期与提交身份
                         ↓
WGPU render passes → GPU texture → GPUI 的布局／裁剪／合成
```

Rhai 不持有 device/queue，不在每帧执行 shader 生成回调。沿用既有 ADR 的 typed Host registration；是否允许用户 shader source，应另行明确验证、设备资源和失败边界，不能因底层有接口就自动开放。

## 4. 当前硬边界：macOS/Windows 没有同等完整的 typed 路径

在 CE `c39bf5ab…`：

- `WgpuContextHandle::from_window()` 供 Linux、FreeBSD，以及启用相应能力的 WASM 使用。
- `WgpuRenderTarget::surface()` 有相同的平台条件。
- custom GPU 示例在其他目标仅打印“不支持”的提示。
- macOS 默认仍选择 native Metal renderer。WGPU 库在某个平台能创建 device，不代表该平台已能与 GPUI 窗口共享、同步并合成资源。

来源：[typed GPU context/target](https://github.com/gpui-ce/gpui-ce/blob/c39bf5abfa81e3851367be0830c6caf7360f6af3/crates/gpui_wgpu/src/wgpu_context.rs)、[示例的平台条件](https://github.com/gpui-ce/gpui-ce/blob/c39bf5abfa81e3851367be0830c6caf7360f6af3/crates/gpui_wgpu/examples/custom_gpu.rs)。

不计迁移成本，可以把补齐这些平台适配纳入我们投入的范围；但在它们实际工作前，不能把 CE 宣称为已满足我们跨平台 ShaderSurface 契约。

## 5. 修订后的决策与验证终点

| 目标 | 倾向 |
|---|---|
| 主要交付表单、文本、复杂桌面控件 | Kit＋Base |
| ShaderSurface、自定义 shader、3D 成为产品一等能力 | **CE 为优先底座候选** |
| 立即要求完整 macOS/Windows/Linux 3D 互操作开箱可用 | 当前证据不足以对 CE 或 Kit 作此承诺 |

当前建议：**不先锁定 Kit；以 CE 优先实现一个跨平台 ShaderSurface 验证原型，尤其先验证 macOS。** 若原型达到下列效果，选 CE 的理由会明显强于“需要更多现成控件”；若平台互操作不能达到要求，则回到“UI 底座＋独立 WGPU renderer＋明确纹理互操作”的备选架构。不要让 CPU readback 成为隐藏的长期回退路径。

原型必须证明：

1. 有真实深度缓冲的 3D 场景，包含可调相机、WGSL 材质和拾取。
2. 场景与普通 UI 同屏，scroll/clip/overlay/透明合成正确；不以每帧 GPU→CPU→GPU 搬运实现展示。
3. Resize、DPI、多个窗口和 render target 重建正确。
4. suspend/隐藏后停止不必要 GPU 工作，resume/dispose 与设备失效后正确恢复或释放。
5. Rhai 只更新有界 typed 参数；GPU render hot path 保持 native，主题和 reduced-motion policy 生效。
6. macOS、Windows、Linux 按同一行为契约测试；Web 按目标能力单独认证。

以上是决定最终效果的验收条件，不是迁移成本评分。本轮仅重新核对目标和技术条件，尚未编写或运行这个 GPU 原型。
