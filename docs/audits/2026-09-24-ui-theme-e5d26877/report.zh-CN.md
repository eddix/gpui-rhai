# 第九轮：图表整改验收、主题一致性与 UI 打磨审计

日期：2026-09-24。产品基线：`e5d268776151a950f81f3d7e68e4bb2feff2d973`。
包含 `e7a29258` 有效 viewport、`fbcb5247` Tabs/Table、`e5d26877` 虚拟行主题快照修复。

## 1. 结论

**第八轮的 18 个独立探针全部通过，三个 viewport 原复现关闭。现在可以把主要精力转向 UI 收口；完整的主题一致性目标仍未达到。**

本轮没有新增已复现的 P0/P1 机制问题。扩大审查后，发现 **5 组 P2 级主题／UI 问题或能力缺口**：

| 编号 | 证据 | 内容 |
|---|---|---|
| U01 | R | Tabs 读取了 inset token，但固定 track/slot 高度破坏四边距和大字体容纳 |
| U02 | R/S | native/virtual 主题快照是固定白名单，自定义 namespace 颜色无法完整传递 |
| U03 | S | 51 个组件中 35 个含固定间距，主题密度迁移尚未完成 |
| U04 | R/S | 6 个主题下，可点击的未选中 Tabs 文字与轨道对比不足 4.5:1 |
| U05 | S | Chart 字号和导出字体仍固定，未纳入统一 typography 契约 |

R 表示独立复现或可重复计算；S 表示源码确认；D 表示打磨建议；G 表示本轮未认证范围。U02 是通用主题支持缺口，不是宣称已修复的内建 `table.selection` 仍然失效。U03 的扫描命中也不等于每个 `px()` 都是 bug，后文明确区分结构几何。

本轮实机查看了 Default Light 的 Navigation & data、Default Dark 的 Navigation & data／Foundations／Forms，以及 Gruvbox 的 Navigation & data；另静态扫描全部 **51 个组件、15 个主题**。不是宣称已逐一完成 51×15 的人工视觉签收。

## 2. 先纠正规格与参考图问题

这部分确有我此前写规格时的责任：旧维护文档仍要求 Tabs “局部例外”使用 8px／5px 圆角、固定 3px inset，Button/Badge 也给出脱离 token 的 padding 表。它会把结构参考误写成外观像素规范，进而鼓励实现绕过主题。

本轮已修订维护文档，而不是另建一份临时需求：

- [registry-design-system.md](../../registry-design-system.md)：删除 Tabs 固定圆角例外及截图尺寸约束；要求矩形表面消费 radius role、spacing 使用共享尺度、容器容纳实际文字和 inset；将 Button/Badge 的核心约束改成密度层级；写明参考图不能入库。
- [visual-testing.md](../../visual-testing.md)：增加非默认 spacing/radius、较大 typography、原生／虚拟路径 token 一致性以及实际状态颜色组合检查。

文档区分“目标契约”与“当前实现缺口”，没有把未实现的自适应高度写成已完成。固定 1px 细边框、图标 viewBox、语义圆形、零位移、必要的最小命中区域等仍可作为明确的结构常量。

**图片状态：** 三张参考图在当前工作区已经不存在，维护文档中也没有它们的引用；但 Git HEAD 仍跟踪它们，删除仍是未提交变更。发布／合并本轮文档整理时必须包含这三项删除和旧临时规格删除，否则只看最新产品 commit 仍会拿到参考图。不需要重写 Git 历史，也不要误删真正的产品测试 baseline。

本次只修改上述文字规格并添加审计材料，未修改产品实现、未新增图片。实机截图仅在审查工具中查看，没有存入仓库。

## 3. U01 · P2：token 已引用，但 Tabs 的尺寸关系仍被常量锁死

位置：[tabs.rhai](../../../registry/components/tabs.rhai) 82–99、145–151。

**R1：** Host 合法覆盖 `spacing.xxs=8px`，使用原样正式 Tabs。实际原生几何：

```text
track height = 30
slot height  = 26
left inset   = 8
top inset    = 2
bottom inset = 2
```

**R2：** Host 将 body 改成 size=24、line_height=36，slot 实际高度仍为 **26**，无法容纳 36px line box 和 2px 边框。

这不是要求把默认 Tabs 做大。默认主题下 30/26/2 可以成立，但 theme inset 改变后，`height(30)` 与 `height(26)` 仍锁死了上下几何关系；单纯将 `.padding(px(2))` 换成 token 不等于支持主题。

**修复边界：** track 的实际高度由 slot 测量结果＋两侧 inset 决定；slot 以 typography、padding 和 border 求高度，最小高度只作为下界。图标独占项随行高保持方形。横向和纵向、content/equal 布局都必须保持这个关系。

**验收：** 默认值继续紧凑；改变 xxs、body line height、图标尺寸后四边距仍一致，文字不越界，首／尾选中和焦点边框不破坏留白。两项复现见 [native-probes.rs](native-probes.rs) 与 [输出](evidence/native-probes.log)。

## 4. U02 · P2：主题捕获只传固定名单，扩展 namespace 支持不完整

位置：[primitive.rs](../../../crates/gpui-rhai/src/primitive.rs) 172–243、312–319；[renderer.rs](../../../crates/gpui-rhai/src/renderer.rs) 1226–1234。

**R：** 使用合法 `ThemeTokenOverrides.namespaces` 定义 `brand.tint=#55aaccff`，通过正常 prepare/mount。注册原生 primitive，在正式 render 收到的 `PrimitiveTheme` 中调用 `color("brand.tint")`：

```text
expected = Some(#55aaccff)
native captured value = None
```

**S：** native 和 OwnedColorResolver 共用的白名单包含目前的 document/chart/table 内建颜色，但不会枚举合法的自定义颜色；virtual row 使用的 OwnedColorResolver 也存在相同截断。这里对 virtual 路径的结论来自源码，并未把原生 probe 说成实际虚拟行像素对比。

`e5d26877` 修复了内建 selection/table.selection/xxs 的漏传，正式测试和实机 Table 选中态都支持这个结论。不过，再增加一个名单内 token 不能证明所有合法 namespaced theme 值已经贯通。当前 native 文档保证的是 required semantic surface，通用 namespace 的边界需明确并补齐，不能将这个范围较窄的保证当成整个主题配置都可用。

**修复边界：** 为 native 与 virtual 渲染使用同一完整、不可变且可按主题代际复用的主题快照，或明确采集实际引用的 token。避免每新增一个语义颜色都要改 Rust 白名单；也避免在每个虚拟行复制整份主题。保持 foreground/native ownership，不把可变 ThemeManager 或 Rhai 值传到后台。

**验收：** 自定义 namespace 颜色在普通节点、原生 primitive、virtual row、overlay 中一致；切主题和局部主题后更新；非法 token 的诊断一致；内建 table.selection 的现有正例保持通过。

## 5. U03 · P2：间距体系尚未真正使用主题尺度

**S：全量扫描结果**（按源代码行，不是按 bug 个数）：

```text
组件数                              51
包含 literal padding/margin/gap 的组件 35
命中源代码行                         85
使用 theme_spacing 的组件             5
使用 theme_spacing 的源代码行         11
```

[完整逐文件、逐行清单](evidence/style-inventory.json) 可通过 [scan-styles.py](scan-styles.py) 重建。

优先迁移这些确实属于视觉密度的值：

| 族群 | 代表位置 | 当前问题 |
|---|---|---|
| Button / Badge / Tag | button 174–176；badge 62–63；tag 93 | 各自维护 3/4/6/8/10/12/14 等 padding/gap，Host spacing 改变时仍保持旧密度 |
| Tabs | 83、92–98、127、141、171 | 只有轨道 inset/gap 用 token，label padding、icon gap、panel padding、根间距仍固定 |
| Card / GroupBox / Popover / Dialog / Sheet | card 44–53；group_box 45–48；popover 93–94；dialog 98–108；sheet 61–64 | 同级容器分别使用 10/12/14/16px，主题无法统一面板节奏 |
| Input / Select 类 / Command / Menu | input 129；combobox 252、377、440；menu 143、173；command 165–178、293 | 触发器、菜单项、搜索行和浮层缺少共享内容边距 |
| Form / 状态组件 | form_field 76；label 73；status_bar 40、52；toast 105、117 | label-help-error、图标文字、行间距无法随主题调整 |

**不机械替换所有数字。** 下列常量有独立结构含义：1px hairline、透明占位边框、0 offset、图标固有坐标、Avatar/Radio/dot/slider thumb 的半径、ButtonGroup 内接边的 0 radius、InputGroup 内层无边框、虚拟化明确声明的 row height。`TitleBar` 平台安全 inset 也不是普通 spacing。清单是审查入口，必须按语义处理。

圆角方面，Button/Input/Card/Popover 等外表面和 Tabs 已使用 theme radius，是正向结果；重点复核 Skeleton 的默认 literal radius 与加载完成目标的关系，以及 Switch thumb、Progress/Slider track 等零圆角是否属于明确形状契约。不要把语义圆形强制改成 `radius.sm`，也不要让矩形组件以“设计例外”为由永久绕过主题。

**修复边界：** 建立少量跨组件的间距使用规则并映射到现有 xxs/xs/sm/md/lg：文字与图标、控件内边距、label/helper、面板内容、区块分隔。无需给每个控件增加必填主题 token。若现有尺度不能表达必要的共享角色，先定义语义及派生规则，再增加一致的解析能力；不要继续散落近似像素值。

**验收：** 使用一套明显不同但合法的 Host spacing/radius 偏好同时挂载上述族群。它们应共同改变密度／圆角，布局关系仍成立；不能只在默认的全零 radii 和相同 spacing 的 15 个主题上证明“主题化”。

## 6. U04 · P2：未选中 Tabs 的可读性在 6 个主题中不足

位置：[tabs.rhai](../../../registry/components/tabs.rhai) 85–87、145–146，以及对应 theme colors。

Enabled 但未选中的 Tab 使用 `text_muted`，实际背景是 `surface_hover`。按源 token 的不透明 sRGB 颜色计算：

| 主题 | 对比度 |
|---|---:|
| Default Light | 4.17:1 |
| Catppuccin Latte | 4.31:1 |
| Catppuccin Mocha | 4.10:1 |
| Everforest | 4.21:1 |
| Gruvbox | 3.17:1 |
| Nord | 4.04:1 |

[全 15 主题数据](evidence/tabs-contrast.json)。实机 Gruvbox 的 Navigation 区域也观察到未选中标题明显弱于选中项；数字依据 token 计算，不是用截图取色猜测。

这里采用普通文字 4.5:1 的可读性目标；“未选中”仍可操作，不能按 disabled 例外处理。[W3C SC 1.4.3 说明](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) 支持这个区分。此处是设计验收目标，不是宣布整个原生应用已完成 WCAG 认证。

**修复边界：** 为实际表面选择合适前景；可调整配对角色、派生 tabs 前景或校正 theme 值。保留次级层级，但不要通过降低可读性制造层级。勿仅检查 text_muted 对 page surface 的比值，忽略 raised/hover/selection 表面。

**验收：** 所有官方主题的 enabled tabs、placeholder、辅助文字在其实际背景上符合目标；disabled、纯装饰和品牌图形单独处理；修改颜色后仍能分辨 selected、focus、hover。

## 7. U05 · P2：Chart 的文字尚未并入 typography 体系

位置：[chart/primitive.rs](../../../crates/gpui-rhai/src/chart/primitive.rs) 1754、1825；[chart/export.rs](../../../crates/gpui-rhai/src/chart/export.rs) 299。

**S：** 原生 chart label 和 tooltip 使用固定 `text_size(px(12))`；SVG label 使用固定 `font-size="12"` 和 `font-family="system-ui, sans-serif"`。因此 Host 改 typography size/line-height 时，图表文字不会按同一语义角色缩放；导出还固定了字体族选择。颜色正确换主题不等于整个图表完成主题化。

本轮没有把 12px 本身判成错误，也没有声称已验证所有字体都裁剪。问题是 typography 输入没有进入 label 的呈现／导出契约。此次修复 Start/Center/End label anchoring 是进步，应保留。

**修复边界：** 把 label 的语义角色和 resolved typography 纳入 prepared/presented scene 或统一呈现样式；原生与 SVG/PNG 消费同一 size/line box/font policy。标题、轴注释、legend/tooltip 按少量角色分层；根据实际文本度量处理边距和拥挤，而非再加固定宽度修正。导出 font availability 要有明确 fallback。

**验收：** 大字体、已注册的等宽/CJK fallback、light/dark/RTL、长标签和导出分别检查；普通字号下不增加无意义留白，字体变化后 title/tick/tooltip 不互相遮挡。

## 8. 面向 Omarchy 风格的打磨建议（D）

实际 Gallery 的直角、细分隔线、矩形 selection 与紧凑表单已经构成一致基础。建议保留，不增加通用 SaaS 式圆卡片、玻璃模糊、霓虹发光或持续动画。

1. **用排版和间距建立现代感。** 同类面板只保留一致的内容边距、标题到正文的间隔、label/value 对齐线。密度可以紧凑，但辅助说明不应成为模糊的小字。按钮、输入、Select 的文字基线与图标中心一起验收。
2. **主题控制皮肤，组件控制语义。** Button 是行动、Badge 是只读状态、Tag 是可移除元数据；Tabs 是一个轨道上的单一选择，ToggleGroup 是独立 pressed 状态。用密度、空间和语义区分，不靠某个永久圆角或配色区分。
3. **一个主 accent，状态色有实际含义。** 普通信息保持 text/surface 层级，选中与焦点分别表达；不把所有框线、标题和数值都染成 accent。Table 的新不透明选中背景在三种实机主题中稳定，方向正确。
4. **Omarchy 观感通过可配置字体呈现。** 可给 Gallery/应用提供注册好的 JetBrains Mono 加合适 CJK fallback 的验收 profile；库继续尊重 Host font policy。不要把字体文件或字体名硬塞进所有组件，也不要把像素字体用于正文。复古未来主义来自终端式信息秩序、稳定数字、锐利几何和克制反馈。
5. **焦点必须跟得上内容。** 已知的 offscreen tab 自动 reveal、大字体自适应、RTL/vertical keyboard、焦点与 selection 同时可见，应作为体验收口项目；不能以外观好看替代可操作性。自动 reveal 是此前已记录的缺口，不算本轮新发现。
6. **建立主题变化矩阵，不只存漂亮截图。** 除 light/dark/community 外，加入 spacing 变化、非零 radius、字体放大、CJK、局部主题、virtual rows、overlay、Normal/Reduced/None。固定少量有代表性的同屏 specimen，用几何／颜色／交互断言辅助实机检查。

本轮使用本地 `omarchy-style` 设计指导；项目是受其启发的组件库，不是 Omarchy 品牌页面。不需要添加 wordmark、强制全局快捷键或锁定黑绿配色。

## 9. 功能、安全、性能和验证范围

| 检查 | 结果 |
|---|---|
| 第八轮原探针 | **18/18**，原三个问题全部通过 |
| Workspace 全目标、全 feature、locked/offline | **564 通过** |
| 原生独立包 | **100 通过** |
| Performance 结构套件 | **4 通过，3 ignored** |
| fmt、workspace/native 严格 Clippy | 通过 |
| 当前 commit 完整 release smoke | 通过，含 Chart/Component Gallery、全部 examples、Theme Studio 与 Table 状态 |
| 新 UI 原生探针 | **3 失败**，归为 U01 两项、U02 一项 |
| 全量 style/contrast 扫描 | 51 组件、15 主题，原始清单已保存 |
| 实机 Gallery | 本文列明的类别和三主题已检查；全矩阵未签收 |

[上轮复验](evidence/previous-native.log)、[workspace](evidence/workspace.log)、[native](evidence/native.log)、[结构性能](evidence/performance.log)、[新探针](evidence/native-probes.log)。
完整启动检查见 [release smoke](evidence/release-smoke.log)，它不替代前述实机视觉与交互检查。

安全抽查包含默认 RestrictedModuleResolver、progress 累积操作预算、typed viewport 的数值有限性检查、原生／脚本权限边界。没有把 `set_max_operations(0)` 脱离其 progress adapter 误报为无限制执行。未发现本轮新增、可复现的越权或崩溃路径；这不是对整个项目的安全无缺陷保证，trusted Rust extensions 仍属于 Host 边界。

100k release 基线，本 commit、5 次预热、30 次采样：resize p50/p95 **27.992/31.479 ms**，128 行 sliding update **85.935/104.827 ms**，streaming Rhai operations **0**。Prepare 155.929 ms、首帧 133.554 ms 是单次启动值。见 [日志](evidence/chart-release-30.log)。与整改说明的另一组数字不同；没有同期受控 A/B，不据此认定性能回归，也不宣称更快。改主题快照时应保留虚拟化有界成本和增量更新性质。

**G：** 原生 macOS AX 在本轮工具中仅暴露窗口控制；这与 [已记录的 GPUI 0.2.2 平台语义桥接限制](../../accessibility.md#pinned-gpui-limitation) 一致。内部 AccessibilityTree 测试通过不等于 VoiceOver 全面通过。本轮没有新增一个“平台 AX bug”，也没有声称完成全平台辅助技术、IME 或全字体矩阵认证。

## 10. 实施顺序与完成条件

1. **先修可观察问题 U01/U02/U04**：几何容纳、主题值传递和文字对比。三个失败原生断言转绿，配色扫描及实际状态检查通过。
2. **按族群完成 U03/U05**：先基础动作与输入，再容器／浮层／数据，最后图表 typography 和导出。保留合法结构常量；不要全仓库无差别替换 px。
3. **以变主题证明一致性**：同一挂载树在非默认间距、非零圆角、字体变大后仍满足密度关系、命中区域、文字可见和状态保存。内建主题值碰巧相同不能作为证明。
4. **交付时一起提交维护文档与已删除参考图**；后续需求只描述语义角色、状态、几何关系和验收条件，数字示例明确标注所属主题。

功能整改验收与 UI 打磨可以分开记录：本轮确认前者通过；后者按上述有限工作包收口，不需要重新启动无边界的核心架构改造。
