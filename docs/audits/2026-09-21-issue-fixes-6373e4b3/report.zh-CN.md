# 6373e4b3：近期 issue 修复验收与扩展审计

日期：2026-09-21。基线：`6373e4b3016fc4da2560fe7f106feec812912dde`，工作树开始时干净。本轮核对 #63/#64 对 #56、#57、#58、#59、#61 的修复，并调查仍开放的 #60、#65。此前已验收的 Motion Runtime 修复也做了正向回归。

**结论：近期修复的常规路径有效，但扩展检查确认 4 处需要处理的问题：1 处 P1、3 处 P2。** 最重要的是 `motion_group` 与组件增量更新之间存在明确的交接缺陷，可以复现 #65 所述“加 shared layout 后点击事务失败”的症状。另三处是 SVG 固定颜色、SVG alpha 和 HostSlot 跨 Host 组合的边界问题。

“新发现”不等于全部由最后两个提交引入：I01 是现有动效/增量组件交接的潜在缺陷；I02/I03 是本次 SVG 修复尚未覆盖的颜色语义；I04 是新 HostSlot helper 的组合边界。历史 API 兼容没有作为缺陷。产品源码未修改，本轮只新增报告、可执行测试和证据，也未修改任何 GitHub issue 状态。

## 验证概览

| 检查 | 结果 |
| --- | --- |
| Workspace：all-targets/all-features，locked/offline | **507 通过，0 失败** |
| 产品 native-keyboard | **62 通过，0 失败** |
| performance 默认测试 | **2 通过，2 ignored** |
| fmt / 严格 all-features Clippy | 通过 |
| 上轮 Motion 正向验收程序 | 重绑定、预算排列/回滚、56 组 inertia 参数、重复 teardown 均通过 |
| 本轮独立原生测试 | **9 项：5 通过、4 失败**，失败项逐一对应 I01–I04 |
| 当前 SHA 的 CI | [CI run 35569565777](https://github.com/eddix/gpui-rhai/actions/runs/35569565777) 的 `portable` job 成功 |

本报告中的缺陷均有独立运行证据。Native tests 使用锁定的 GPUI 0.2.2；SVG 检查走实际 GPUI SVG decoder 并读取像素字节，交互与布局检查走实际 mount/布局/prepaint/paint/输入分派。没有把测试平台结果当成物理屏幕或 120 Hz 性能认证。

[复现入口](reproduce.md) · [交付测试源码](native-probes.rs) · [交付测试完整输出](evidence/delivered-native.log)

## I01 · P1：motion_group 的继承属性在子组件增量更新时丢失

定位：[node.rs:2519](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/node.rs:2519)、[node.rs:956](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/node.rs:956)、[lifecycle.rs:295](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/lifecycle.rs:295)。关联 [issue #65](https://github.com/eddix/gpui-rhai/issues/65)。

最小场景只需要一个正式计数器组件：组件返回带 `.shared_layout("item")` 的 keyed Text，外层用 `motion_group("g", [Counter(...)])` 包裹。首次挂载成功；点击计数器，更新其自身 state，第一次增量 rerender 就失败。

本轮三组对照：

| 子组件声明 | 点击 | 新状态是否呈现 |
| --- | --- | --- |
| 不加 shared layout | 成功 | count=1 |
| 子组件内 `.shared_layout("g", "item")` | 成功 | count=1 |
| 子组件内 `.shared_layout("item")`，group 从外层继承 | **失败** | 仍停在 count=0 |

错误为：`invalid shared layout declaration: shared layout group and id must be declared together`。Automation dispatch 返回 Err，`ScriptViewHandle::last_error` 也有同样的原因。因此这是事务被拒绝，不是单纯没有播放动画。

**根因：**`apply_motion_group` 直接递归修改 `node.attributes`，没有进入 ADR 0019 的 presentation mutation / snapshot 交接机制。原始组件输出只有 shared_layout_id；外层后加的 shared_layout_group 没有被记录为可重放的外部呈现。组件自己变脏时，`render_dirty` 仅替换该组件的新输出，不会重跑外层 `motion_group`，随后 shared-layout 校验发现 group 丢失，整个点击事务回滚。

复现不需要 Layer、NativeSignal 或复杂 FloatWindow，定位已经收敛到 group 继承与增量更新。它与 #65 的 shared-layout 症状吻合，但在没有完整接入代码的情况下，不能断言它是该 issue 所有失败场景的唯一原因。

**建议修复：**将 group 成员关系保留为可随 retained tree / component recipe 正确解析的声明上下文。可以在计划解析时继承 group，也可以让相应外部呈现进入现有 replay 协议，但不能只更改当前树的临时 attributes。应保留增量更新能力，避免为了重新获得 group 而强制每次重跑根脚本。

**回归条件：**外层 group + 子组件独立 state 更新、receiver/node-prop replay、nested group、虚拟项后续实现、热重载。确认 group 身份稳定、点击正常、旧快照不污染新组配置。

**临时规避：**在组件自身 render 中使用双参数 `.shared_layout(group, id)`，本轮该对照组通过。group/id 仍需满足当前唯一性约束。

### #65 还需要区分的两个场景

- issue 片段把 hole 和 float shell 同时设为相同 shared-layout identity。ADR 0020 当前约定每个 group/domain 只有一个真实节点持有该 identity，出场源由快照表示；两个同时存活的真实节点重复 identity 会被拒绝。占位 hole 不应与当前真实目标重复声明同一 identity。这个约束与 I01 的“合法声明在更新后丢失 group”是不同问题。
- keyed receiver wrapper 对 caller-owned node prop 添加 `enter_motion(opacity)` 或 `enter_motion(translate_y)`，本轮两个最小原生场景均能正常点击并更新子组件。尚未复现 issue 所述另一种 enter-motion 故障，不能把所有 enter-motion 或跨 portal 组合都归因于 I01。

仅在某种树的文本投影里看到 error-boundary fallback，也不足以证明该 fallback 实际被绘制；应同时查看 presented geometry、dispatch 的 Err 和 last_error。

## I02 · P2：SVG 通道补偿只覆盖 currentColor，固定颜色仍会红蓝交换

定位：[asset.rs:871](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:871)、[renderer.rs:2824](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/renderer.rs:2824)。关联 [issue #58](https://github.com/eddix/gpui-rhai/issues/58) 的边界覆盖。

目前通过把替换 currentColor 的字符串写成 `#BBGGRR`，抵消 GPUI 0.2.2 的 SVG decoder 未执行 RGBA→BGRA 的问题。这个办法修复了单色 currentColor，却没有处理 SVG 中原有的固定 fill/stroke、渐变色等内容。

独立 GPUI decoder 测试：

| 输入 | 最终像素字节 | 结果 |
| --- | --- | --- |
| currentColor，指定 RGB `#1234ab` | `[171,52,18,255]` | 正确 BGRA |
| 固定 `fill="#ff0000"`，同一 tint 路径 | `[255,0,0,255]` | 错误；红色应为 `[0,0,255,255]` |

因此多色 SVG 可以同时出现“currentColor 部分正确、固定颜色部分错误”。没有 currentColor 的彩色 SVG 也不会被这层字符串补偿修正。此结论来自真实像素字节与 pinned GPUI 的格式约定，不是根据 tint 字符串猜测显示结果。

**建议修复：**在完整 SVG 的解码/像素边界统一处理通道顺序，或采用经过 source-verified 的其他完整适配路径；currentColor 的语义解析和 GPUI 像素格式兼容应分开。上游修复后能够整体移除这层兼容，避免只对某类颜色反转或重复反转。

**回归条件：**currentColor、固定红/蓝、二者混用、局部渐变、透明像素；inline 和 asset-backed SVG 都要验证最终 RenderImage 字节。现有纯 currentColor 的 1×1 测试是必要控制组，但覆盖面不足。

## I03 · P2：SVG currentColor 丢弃语义色 alpha

定位：[asset.rs:877](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:877)。这是本轮额外确认的颜色语义缺口，可以和 I02 一起修，但应保留独立回归。

`tint_svg_current_color` 从 RGBA 中仅提取 RGB，然后输出六位十六进制颜色。低 8 位 alpha 被丢弃。接口、样式和缓存 key 都接受完整 RGBA，渲染结果却始终把这部分色彩变为不透明。

独立输入 `Rgba8::from_rgba_hex(0x1234ab00)`，SVG 为 `fill="currentColor"`：

- 期望 alpha=0；
- 实际解码像素为 **[171,52,18,255]**，完全不透明。

这会让继承半透明/透明文字前景的图标与相邻文字表现不同。该缺口在旧的 RGB-only tint 中也存在，属于新发现的既有问题，并非将它错误归为 #63 新引入的回归。

**建议修复：**保存 currentColor 的完整 RGBA 语义，并与 SVG 自身 opacity/fill-opacity/stroke-opacity 正确组合，不能用整个节点的 opacity 草率替代：那会连 SVG 的固定颜色部分一起改变。

**回归条件：**alpha=0、0.5、1，以及它们与 SVG opacity 的组合；最终像素和相邻文本应具有一致的透明度语义。

## I04 · P2：with_script_view 没有保留独立 Host 的 frame boundary

定位：[host_slot.rs:101](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/host_slot.rs:101)、[app.rs:3653](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/app.rs:3653)。关联 [issue #59](https://github.com/eddix/gpui-rhai/issues/59)。

`HostSlotRegistry::with_script_view` 只返回 `view.flex_item()`。对于同一原生窗口内、属于不同 ScriptViewHost 的 resident view，外壳的 Host.container 不会使 resident 自己的 Host 进入 active frame。后者 render 时进入错误状态：

`embedded ScriptView must be rendered inside ScriptViewHost::container`

三组独立原生对照：

| 放置方式 | resident.last_error |
| --- | --- |
| 外壳与 resident 共用 Host，使用 with_script_view | None |
| 不同 Host，直接使用 with_script_view | **上述错误** |
| 不同 Host，with_slot 工厂显式包 resident 自己的 Host.container | None |

这不是把一个 view 放进了别的原生窗口：测试中的 outer/resident 始终在同一 GPUI Window。公开 helper 声明支持 independently mounted script view，但没有说明或校验“必须共用 Host”的前提。现有新增产品测试只使用 native div 工厂，未覆盖这个 helper 的主要组合场景。

**建议修复：**明确 helper 的 Host/frame 归属协议。若支持不同 Host，应保留 resident 的 container 边界，并回归各自的 overlay、focus、输入和生命周期；若有意只支持同 Host，则必须公开并验证该前提，并提供跨 Host 的明确用法。无需赋予外层 Rhai 对子视图的 lifecycle 权限。

**已验证的临时路径：**使用 `with_slot` 工厂返回 `resident_host.container(resident_view.flex_item()?)`，错误转换为工厂的 String 错误；完整可编译写法在测试源码中。

## 已通过的近期修复与对照

| Issue / 场景 | 判断 |
| --- | --- |
| [#56](https://github.com/eddix/gpui-rhai/issues/56)：静态祖先前景色继承 | renderer 正确传递 ambient color，node-local 覆盖和 asset ancestor 测试通过；不把 alpha 缺口算作该继承链失效 |
| [#57](https://github.com/eddix/gpui-rhai/issues/57)：IconButton selected | selected ghost 的 root/icon accent 与 pressed 属性测试通过；source 中 disabled/loading 前景优先级明确 |
| #58：不透明 currentColor 单色 SVG | 原始场景通过；固定颜色和 alpha 见 I02/I03 |
| #59：native 工厂布局、输入隔离 | native click 保留，外层 Rhai capture 不触发；slot 外点击控制组能触发 capture。flex slot 实际填满 **300×210**；跨 Host helper 见 I04 |
| [#61](https://github.com/eddix/gpui-rhai/issues/61)：默认 Combobox/Select 自适应宽度 | 产品原生测试通过：resize 后 trigger 随剩余空间变化、panel 匹配实际 trigger、固定 Select 保持 140 px；自定义 trigger 继续走原有策略 |
| 之前 Motion F01–F04 | 正向验收程序重跑通过，没有因本轮发现而重新打开那些已修复的具体缺陷 |

## #60：当前最小场景未复现，不能直接判为已解决

[Issue #60](https://github.com/eddix/gpui-rhai/issues/60) 描述 fill_height 的 outer bounds 正确但数据行不实现。本轮实际运行两个父级布局：

| 父级高度 | Table 模式 | 原生 list viewport | 数据实现数 |
| --- | --- | --- | --- |
| 固定 500 px，含 30 px header | height=430 | 430 px | 15/15 |
| 固定 500 px，含 30 px header | fill_height=true | 438 px | 15/15 |
| relative(1.0)，含 30 px header | height=430 | 430 px | 15/15 |
| relative(1.0)，含 30 px header | fill_height=true | 1018 px | 15/15 |

Accessibility 中 role=row 的计数为 16，包含 1 个表头行；数据数取 native virtual metrics 的 item_count/realized_count=15，未把表头当成数据成功。测试也断言不能只剩表头。

因此暂时没有证据把 #60 定位为最新版本的一般 fill_height 故障；也不能仅凭这些简例宣布下游问题消失。本轮已请求真实 Table 调用及完整父布局/Host 容器，尚未取得接入项目复现。这个待确认项不混入四个已证实缺陷。

## 建议实施顺序与验收方式

1. 先修 I01 的 group 上下文与增量交接；它会拒绝正常交互事务，是当前最应优先处理的问题。
2. 修 I04 的 Host frame 归属，并覆盖真正的 ScriptViewHandle 组合。保留已验证有效的 native 内容事件隔离和 flex 行为。
3. 将 I02/I03 作为一次 SVG 适配器修复处理，分别保留固定色、透明度的独立像素断言。
4. #60 按完整接入布局继续定位；不要在缺少失败用例时改动已经通过的 Table 高度路径。

持续迭代应把验证范围从“某属性首次渲染正确”扩展到“属性跨组件更新仍正确”、从“native div 可以放进去”扩展到“真实 ScriptView 的 Host 生命周期边界仍正确”、从“替换出的 hex 正确”扩展到“完整图片最终像素正确”。

HostSlot 的拖动性能动机尚无本轮专项 release 基准。后续应同时记录大 resident view 拖动时的 Rhai 调用、Rust render/节点转换次数和帧时间，不能把“没有每帧 Rhai 调用”直接等同于完成 120 Hz 性能验收。该项是验证建议，没有缺乏证据地另记成性能 bug。
