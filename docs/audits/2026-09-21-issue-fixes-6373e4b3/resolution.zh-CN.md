# 6373e4b3 扩展审计整改结果

日期：2026-09-21。基线报告见 [report.zh-CN.md](report.zh-CN.md)。本文件记录产品整改，不改写原始失败证据。

## 结论

I01–I04 均已按正确行为整改。原报告附带的独立 `run-native.sh` 从 **5 通过、4 失败**变为 **9/9 通过**：

- 继承 `motion_group` 的正式组件在自身 state 增量更新后仍能点击并呈现新状态；
- `currentColor` 与固定红色都产生正确的最终 BGRA 像素；
- 全透明语义色保留 alpha=0；
- 同 Host、不同 Host、显式 resident container 三种 HostSlot 组合全部无 resident 错误。

产品套件同时通过 workspace all-targets/all-features、**64 项** native-keyboard、performance 结构测试、严格 workspace/native Clippy、`-D warnings` rustdoc、38 张视觉基线审计与 22 个目标清单检查。

## I01：motion group 成为可重放呈现上下文

`motion_group` 不再只递归改写当前树的临时 attributes。组件边界记录 `MotionGroup` presentation mutation；组件 subtree 被增量替换或 snapshot hydrate 时，该上下文递归重放。嵌套正式组件各自记录边界，节点自身显式声明的 group 仍优先。

虚拟集合保存继承 group；后续 viewport realization 返回新行后，生命周期在提交前应用同一上下文。因此修复没有退化为强制根脚本全量重跑，也没有把 Rhai 构树放回高频路径。

这项修复覆盖 #65 中合法的“外层 group + 子组件 `.shared_layout(id)`”失败。一个 presentation domain 内两个同时存活的真实节点仍不得声明相同 `group/id`；hole 与 float shell 同时占有同一 identity 仍是显式错误，不属于本次放宽范围。

## I02/I03：完整 SVG 像素边界适配

删除仅把 `currentColor` 写成 `#BBGGRR` 的局部补偿。现在完整 SVG 先由锁定的 usvg/resvg 在有尺寸和像素预算的边界内栅格化为标准 RGBA PNG，再交给 GPUI 0.2.2 已正确实现的 PNG RGBA→BGRA 解码路径。

`currentColor` 替换保留完整 `#RRGGBBAA`；固定 fill/stroke、局部渐变、SVG opacity 和语义 alpha 由同一次 SVG 合成处理。asset-backed SVG 按 identity+RGBA 缓存，inline SVG 按 source+RGBA 缓存，避免每帧重复栅格化。GPUI 上游修复后可以整体移除适配器，而不需要判断哪些颜色曾被预反转。

## I04：HostSlot 保留 resident Host frame

`HostSlotRegistry::with_script_view` 现在通过 resident handle 自己的 Host 边界生成 element。若 resident Host 已是当前 active frame，则直接复用，避免同 Host 嵌套；否则自动包裹 resident 的 `ScriptViewHost::container`。shell 与 resident 因而可以在同一 GPUI Window 中属于不同 overlay/focus domain。

Rhai 仍只拥有 slot box 的布局声明。resident 的 suspend/dispose、语义树、overlay 和事件域没有转交给 shell。

## 未改变的边界

- #60 的一般 `fill_height` 路径仍未复现故障，本轮不改 Table 高度算法。
- #65 中跨 portal 的重复 live shared identity 仍按 ADR 0020 拒绝；本轮只修复合法继承 group 在增量交接时丢失的问题。
- 原报告提出的 120 Hz resident 拖动性能建议仍需真实应用基准，本轮没有用功能测试代替性能认证。
