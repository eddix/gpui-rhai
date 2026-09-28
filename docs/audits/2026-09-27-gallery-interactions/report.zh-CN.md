# Gallery 交互与 Acceptance Application 复审

日期：2026-09-27。审查基线：`502c9fe2`，gpui-pre `0.3.6`。

## 结论

**当前版本已有有价值的同源展示、数据 fixture 和生命周期基础，但还不能按“完整 Acceptance Application”验收通过。** 两个用户可感知的公共问题已经复现：单轴容器会转换另一轴的滚动输入；Input 获得真实输入焦点后没有绘制声明的 focus border。Workbench 的成功反馈与配置数据没有形成闭环，Gallery 外壳还存在键盘可达性与空间组织问题。

Resizable 是明确的公开能力缺口，单独作为新增工作处理，不能冒充已存在组件的 bug。复用旧示例本身没有问题；需要审查的是能否定位能力、完成任务，以及反馈是否与真实状态一致。

本次按 **2026-09-26 修订后的已确认计划** 审查，不把最初提案中的 UI zoom、Omarchy adapter 等已被明确移出范围的能力列为漏做。未修改产品代码。

## 1. [P1] 单轴滚动沿用 GPUI 的跨轴转换，造成横纵串轴

位置：`crates/gpui-rhai/src/renderer.rs:489` 的 `apply_scroll_behavior`；`crates/gpui-rhai-cli/src/gallery_app.rs:927` 的 preview viewport、`:1019` 的 story list；固定上游 `gpui-pre-0.3.6/src/elements/div.rs:3325`。

上游 `restrict_scroll_to_axis` 默认 false。只有 X 轴允许滚动且输入 `delta.x == 0` 时，它会把 `delta.y` 用作 X 位移；仅 Y 轴滚动时则可以把 X 输入转为 Y。项目的通用滚动包装没有设置该策略，Gallery 的原生容器也没有设置。因此这不是必须依靠用户偶然手势才能发现的问题。

原生探针结果：

| 容器 | 输入 | 当前结果 | 限制轴向的对照组 |
| --- | --- | --- | --- |
| 横向 | `(0, -45)` | `offset.x = -45` | X/Y 均为 0 |
| 纵向 | `(-45, 0)` | `offset.y = -45` | X/Y 均为 0 |
| 实际 Rhai 横向 overflow Box | `(0, -45)` | 子节点实际 X 从 0 变为 -45 | 本探针未改产品，只记录缺陷 |

这能解释“有时候混在一起”：取决于鼠标所在子区域、该处允许哪个方向滚动、实际溢出与事件传播。Gallery preview 是横向容器；源码／文档也有自己的横向滚动区域。并非每次纵向滚动都会遇到同样的容器。

**整改要求：** 定义普通滚动容器的轴向政策，并在 Runtime 和 Gallery 原生 shell 统一落实。一般滚动输入应沿原轴消费；Shift+wheel 的平台转换、二维画布和 Chart pan/zoom 使用显式策略。上游 flag 同时影响精确手势轴锁定，不能只按名字全库替换后宣布问题全部解决。

补真实输入测试：纯 X、纯 Y、轻微斜向、持续／结束手势、Shift+wheel、嵌套滚动、子容器到边界、内容恰好无溢出，以及 Table/CodeViewer/Chart 与父页面组合。断言各层实际 offset／geometry，保证不串轴、不重复消费、不把余下输入无条件吞掉。不要在某个 Gallery 页面手工把 X/Y 再交换一次。

边界：本轮证明了一条公共、确定性的原因；尚未覆盖用户使用设备的完整手势序列，不能声称所有滚动问题都已穷尽。

## 2. [P2] Input 已声明焦点边框，但原生输入焦点没有驱动外层样式

位置：`registry/components/input.rhai:126–140`、`crates/gpui-rhai/src/renderer.rs:2415` / `:4266`、`crates/gpui-rhai/src/text_input.rs:704`。

正式 Input 已有 `.focus(style().border_color(theme_color("focus_ring")))`；项目视觉规格也要求可见焦点和预留边框。这里不需要再发明一个新的 Omarchy 装饰效果，应兑现现有语义。

源码链路存在两处断开：

- `node_needs_interaction_wrapper` 不因单独存在 pseudo style 就一定建立包装；普通 Input 的 change/focus callbacks 属于 primitive props，不能视为外层普通节点的事件绑定。
- 真正接收输入的是 `TextInputEntity` 自己的 FocusHandle；renderer 的 `.focus(...)` 与外层包装关联，并不自动跟随内部 primitive 的输入焦点。即使 AX 或其他属性促成外层包装，也不能假定它与内部 Entity 拥有同一个焦点。

实测将普通 border 覆盖为红色、`focus_ring` 覆盖为绿色：原生点击、全选、输入 `focused and editable` 均成功，公开语义 value 也更新，但已呈现 frame 的有边框 quads 仍是 **4 个红色、0 个绿色**。focus style 只是存在于 source，未成为正确的绘制结果。

**整改要求：** 把 native primitive 的真实 focus identity 与外层样式投射统一，或提供明确的 native focus-within 机制；保持只有实际输入控件拥有可编辑焦点，不能靠增加第二个 Tab stop 解决。统一排查 Textarea、文档搜索输入等同类路径，不仅修 Input story。

验收应验证实际 painted border，覆盖鼠标与 Tab 聚焦、失焦、disabled、read-only、error、主题热切换与 Host `focus_ring` override。error 和 focus 的优先关系写入规格，不能为了强调焦点丢掉错误状态，也不能让边框变粗挤动布局。

## 3. [P1] Workbench 的“部署成功”不提交配置，核心业务流程没有闭环

位置：`crates/gpui-rhai-cli/src/gallery.rs:274–369`；`registry/stories/apps/operations.rhai:110–140`、`:289–306`。

`gallery.operations.deploy` 的输入 schema 只有一个 bool，表示是否失败；没有目标主机、release channel、候选配置或预期 revision。Host 线程只按序发送 `15 → 48 → 76 → 100` 或失败序列。Rhai 在收到 100 后切换阶段和 Toast，没有提交 Host 配置。

独立探针通过实际 Input 将 release channel 改为 `audit-canary`，确认部署，等到实际树出现 `Configuration verified and activated.`，再导航回 Configurations，读取原生 Document 参数：

| 文档 | 之前 | “成功”之后 |
| --- | --- | --- |
| `operations_config_left` | revision 1；edge-01 / stable | 完全不变 |
| `operations_config_right` | revision 1；edge-02 / candidate | 完全不变 |

两份文档都没有 `audit-canary`。这违反当前计划要求的“所有页面消费同一会话业务模型、确认后更新结果”。本地确定性 fixture 完全可以做真实的内存状态提交，不需要真实网络服务或持久化数据库。

同一个模型问题还体现在配置对照：选择的 host 只改变左右标题，正文永远来自固定的 `operations_config_left/right`。如果选择了其他主机，标题可以与文档内 host 不一致。

现有 normal/cancel/failure 测试主要判断成功／失败文字，没有证明提交修改了目标状态，或失败保留了那个状态；另一个配置文档测试是测试代码主动调用 replace API，不能代替部署流程确实调用了它的证据。

**整改要求：** 明确唯一的会话业务模型，使用类型化、带目标与 revision 的部署请求。确定性本地服务也要经历候选、确认、执行、提交／失败；提交后更新目标主机配置和相关派生视图，取消／失败不改 committed 状态。流式场景如宣称更新指标／事件，也应发布对应数据 revision，不能只更新一行 revision 提示。

验收必须比较真实数据：选非默认主机和非默认 channel；正常完成后该目标 revision/content 改变、其他主机不变；返回 Hosts/Config/Dashboard 看到同一结果；取消／失败前后 committed snapshot 相等；切换页面和重置时迟到结果不修改新会话。

## 4. [P1] Gallery 外壳的关键操作未进入键盘焦点链

位置：`crates/gpui-rhai-cli/src/gallery_app.rs` 的导航条目、category、case、Reset、viewport、Motion、theme 和 locale 构造；例如 `:808–818` 的 Reset。

这些原生 `div` 有 `role(Button)` 和 `on_click`，但没有 `focusable`、`tab_index` 或 `track_focus`。固定版本 GPUI 不因 role/on_click 自动让它进入 Tab 链。

使用与 shell 相同构造方式的原生对照探针：

- 只有 role/on_click：`focus_next` 后无焦点；完整 Enter down/up 不触发 callback。
- 加入显式 Tab stop：能聚焦，同样 Enter down/up 触发一次 callback。

这是固定 API 的构造级证据，结合 shell 源码确认缺口；本轮没有把探针当作完整 Gallery 窗口的端到端认证。

**整改要求：** shell 的操作具备键盘、焦点视觉和原生语义的完整契约，优先消费正式交互组件；如确有原生 shell 元素需要，也必须复用一致的交互政策。实际跑完整 Gallery 的 Tab/Shift-Tab/Enter/Space 路径，覆盖 Search → 选择 story/case → Reset → 主题／Motion／viewport，而不只测试 story 内部。

目前 Gallery 原生 suite 多数调用 `prepare(story)` 后放进简化 `GalleryHost`，实际 `GalleryApp` 的导航、固定布局和工具栏并未一起进入这些测试，因此该问题可以在全部 story 测试通过时仍然存在。

## 5. [P2] Explore 仍以集合页兜底，覆盖统计不足以证明“找得到、用得上”

位置：`registry/src/lib.rs:548–572`、`:925–941`；`tests/native-keyboard/tests/gallery.rs:148–213`。

当前 11 个 stories 有新增的 Input、Tabs、Table 等针对性内容，并不是除了 Workbench 全部没做。但 `components/catalog` 把所有 51 个 component IDs 直接声明在 `module_ids` 中，仍使用 `STUDIO_SOURCE` 大样本页。coverage 测试只做集合成员检查；一项总目录就能满足全部组件覆盖。

因此未有独立定位的 Select、Popover、Slider 等可以被搜索匹配到总目录，却仍需用户自行找 case 和页面位置。Motion 也仍以整页 catalog 为主。复用代码是正确的，但需要把导航落点、状态和用法从集合展示中明确拆出来；不要求每个组件新增一个独立文件。

全 cases 原生测试的最低条件是无错误、存在 semantic root、**至少一个**非零几何节点。这能证明挂载，不能证明目标控件在可见区、关键状态正确或操作可完成。它也没有对每个适用 case 执行至少一个交互，弱于当前确认计划。

**整改要求：** 为每个公开对象提供可直接到达的 story/case/anchor 和真实用法，搜索命中后定位目标；保留组合 specimen 作为横向比较。覆盖表分别记录“可发现、实际呈现、适用交互、预期结果”，测试解析真实声明并断言对应目标，不能只检查 module 名单和任意节点。

## 6. [P2] Workbench 被放进样本查看器，空间与层级未达到参考应用要求

已核对现有产品基线：`tests/visual/macos/gallery/operations.dashboard.default-dark.en.normal.png` 与 `components-catalog.forms.default-dark.en.normal.png`。这是仓库已有实机截图的审阅，不是本轮重新运行实机验收。

确认计划定义 Explore 和 Operations Workbench 两个一级区域，但实际 Workbench 是普通列表 story，外部导航、用途／模块／fixture 元信息、case 控制和常驻源码栏全部保留。内部又嵌入一套导航和 title bar。在 1180×820 基线中，外层 sidebar 230、source pane 360，再扣除内部 sidebar 230，业务主区明显拥挤；Workbench 标题已经截断，表格／图表的有效观察空间不足。

源码里这些面板不能调整，Auto 始终并列；切换固定 viewport 只是改变布局预设，不能替代使用者按当前任务调整空间。很多与调试有关的元信息常驻最显眼区域，也挤占实际任务。

**整改要求：** 提供真实一级模式切换，Workbench 默认以应用空间运行；源码与开发详情按需展开。Explore 的 preview/source 可折叠和调整比例，窄窗口有明确替代布局。旧 Component Catalog 去掉重复的内部 Gallery chrome，只复用其场景内容。

这一项与 Resizable 新能力有直接联系，但先把层级和可折叠行为做对，不是加几根拖动条就自动完成设计。

## 7. 能力缺口：新增通用 Resizable / SplitPane

公开 registry 没有通用可调整面板。当前已有 Table 列宽拖动、pointer capture、几何与 native hot-value 机制，但不等于用户可以直接组合的 SplitPane。

建议新增一组正式可组合组件（最终命名由实现统一）：Group、Panel、Handle。第一用例就是 Gallery 导航／预览／源码分割，随后用于文档比较和 Workbench。

第一版边界应包括：

- 水平／垂直分割、稳定 panel key、明确的尺寸单位或比例策略、初始与受控值。
- min/max、容器 resize 后的合法分配，以及约束之和超过可用空间时的确定性处理。
- 拖动期间 native preview；结束后一次受控提交，拒绝时恢复；不为每个 pointer move 重新执行整个 Rhai 树。
- pointer capture、取消、失焦／窗口关闭、布局代际改变后的清理；RTL 和嵌套分割。
- handle 的可见 hover/focus/drag、扩大但不遮挡内容的命中区、键盘步进与原生 separator 语义。
- theme spacing/radius/color、方向和 viewport 改变时保持一致。

暂不扩成 Dock、多窗口面板拖放或布局持久化框架。正式组件、公开底层机制、story 和原生断言一起交付，不能只在 Gallery 写一个不可复用的拖动实现。当前没有相关既定组件契约，故将它记为新增能力而不是发布回归。

## 8. 对“验收应用”的判断与整改顺序

应保留已有成果：单入口、真实运行 source、独立 stories、NativeCollection/TextDocument/ChartData fixture、有界订阅、受控状态以及暂停／重置的基础确实有价值。旧场景复用也符合“同源”的初衷。

未达到要求的关键是三个层面没有一起完成：

1. **公共机制**：轴向滚动与 native focus 样式未闭合。
2. **应用任务**：Workbench 的成功反馈没有对应的数据提交。
3. **验收方法**：对 story 的挂载测试被用来支持完整 shell／用户任务结论，覆盖颗粒度不够。

建议顺序：先修公共滚动与焦点；补 Gallery shell 键盘完整路径；让 Workbench 数据任务真正闭环；随后完成一级模式／源码布局与 Resizable；最后补对象级发现、状态和行为覆盖。测试矩阵随对应修复同步加入，不放到最后补截图。

这一轮不应以增加新大面板、再做一组静态截图或增加“模块已覆盖”数字作为完成标准。验收人员要在完整 Gallery 中用鼠标、触控板和键盘真正走完查组件、试交互、看源码、修改部署参数、确认／取消、返回核对数据的路径。

## 9. 独立验证与交付范围

本轮 5 个 characterization probes 均完成，日志见 [evidence/probes.log](evidence/probes.log)，源码见 [probes.rs](probes.rs)。它们的断言刻意记录当前缺陷，**5/5 通过表示复现成立，不表示产品验收通过**。修复时应把相应断言改为正确行为并纳入正式套件。

```sh
python3 docs/audits/2026-09-27-gallery-interactions/run-probes.py
```

runner 使用当前 independent native workspace 的 manifest／lockfile，在临时目录运行测试，复用其构建缓存；不修改产品或正式测试源码。复查未来版本时需记录实际 commit。普通输入、实际 Rhai 滚动和 Workbench 使用产品公开接口；轴向 flag 与 shell button 的对照组使用同一固定 GPUI 的原生构造。

本轮没有重跑整个 workspace、发布 smoke、Linux 真窗口或完整 macOS 手势矩阵；没有新增第三方图片、修改正式视觉规格、提交或发布任何修复。
