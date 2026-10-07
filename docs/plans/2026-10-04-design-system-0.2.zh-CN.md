# gpui-rhai 0.2.0 设计系统重做实施计划

日期：2026-10-04。分支：`feat/design-system-0.2`（自 `v0.1.8` / `d87ebb9`）。状态：五步实施完成，待维护者评审；未推送、未发版。

本计划来自 2026-10-04 与维护者的设计 grilling 会话。共识的正式内容写在
[`docs/design/`](../design/principles.md)，本文件只跟踪执行顺序、完成证据和遗留事项。
细节决策记入 [`docs/design/decisions.md`](../design/decisions.md)。

## 授权边界

- 一口气完成下列五步，步骤之间不停下评审。
- 按主题攒批提交到本分支；不推送、不合并到 `main`、不发版、不打 tag。
- 只在三种情况停下问维护者：某条共识被证明做不到、必须改方向；破坏性或对外操作；需要维护者本人操作（权限等）。
- pre-1.0 有意破坏性升级：版本 0.2.0，`runtime_api` 2→3，不保留历史兼容别名或双轨实现。
- 不修改 `oh-my-byted`、`disktree-rhai` 等外部仓库；disktree-rhai 只用 `cargo --config` 临时指向本分支做冒烟。

## 五步

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | `docs/design/` 四份规范 + 决策日志 + Gallery 线框；延迟构造 ADR | 完成（c232500） |
| 2 | 运行时地基：开放 token 与字阶、token 基础层、组件声明 token、声明式环境值（原生解析）与原生继承 disabled、颜色派生与读取、Rust 派生迁到 L0、action 快捷键查询、组合审计、CLI profile | 完成（3224767、64108b0） |
| 3 | L0 `tokens.rhai` + 内置主题只写颜色 + 新默认主题；全部 62 个组件的组合契约与新视觉 | 完成（见进度记录） |
| 4 | L2：`layouts/`（Stack/Inline/Toolbar/Region）与 `patterns/`（Section/DescriptionList/Stat/FormLayout/InlineState/DataView/ListDetail/AppShell） | 完成（见进度记录） |
| 5 | 用 AppShell + L2 重做 Gallery（规格页 + 场景页）；自带设计参照示例进 CI；审计零告警；纯键盘场景测试；重拍基线；文档、CHANGELOG、迁移说明 | 完成（见进度记录） |

## 每批验收命令

与 CI 一致：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --manifest-path tests/performance/Cargo.toml --locked
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked
bash scripts/audit-visual-baselines.sh
```

视觉改动另外截图自查；Linux 只能在推送后由 CI 验证（本机为 macOS）。

## 进度记录

（按批次追加：日期、提交、内容、证据。）

### 2026-10-04 第 3 步：62 个组件按 0.2 契约重做

- 内容：全部 62 个组件升到 0.2.0，声明 `tokens` / `environment`；尺寸与密度全部走
  `metrics.*`；标记系统（Button / Tag facet / Badge 方灯 / Kbd 键帽）；字段统一 2px 框；
  列表行语法（`metrics.row`、`metrics.inset`、指示条）；Table 无外框、标签语气表头、数字列；
  Tabs 轨道 + 浮起拇指；ToggleGroup 漫游焦点；覆盖层统一为浮起色块 + 发丝线。
- 为此补的运行时能力：`group_focus` / `focus_within` 样式、带焦点样式节点的 tab stop 修复、
  覆盖层横轴 `align`、fill-height 虚拟列表延后首次 reveal、`ScriptApplication` 致命错误写 stderr、
  滑块拇指按锚点居中并解析焦点态。决策见 `docs/design/decisions.md` D8–D19。
- 截图自查（本机 macOS，默认深浅两套主题）：markers、fields、lists 三张规格图，紧凑档高度
  量过（md 28、sm 24、xs 20）。
- 证据：`cargo fmt --all --check` 通过；`cargo clippy --workspace --all-targets --all-features
  -- -D warnings` 无输出；workspace 667 passed / 0 failed；native-keyboard 213 passed / 0 failed；
  performance 2 passed / 0 failed；`scripts/audit-visual-baselines.sh` 通过（69 张，基线待第 5 步重拍）。

### 2026-10-04 第 4 步：L2 layouts 与 patterns

- 内容：`layouts/` 4 个（Stack、Inline、Toolbar、Region）与 `patterns/` 8 个（Section、
  DescriptionList、Stat + `stats()`、FormLayout、InlineState、DataView、ListDetail、AppShell），
  登记进 `BUNDLED_LAYOUT_SOURCES_BY_ID` / `BUNDLED_PATTERN_SOURCES_BY_ID`，registry lint 覆盖到 L2。
- AppShell 区域键盘模型：F6 / Shift+F6 在侧栏、主区、检查器之间移动焦点；区域自身持焦点时
  显示 2px 墨色框，Tab 进入区域内第一个控件。
- 运行时：键处理名支持修饰键（`shift+f6`、`cmd+k`，无修饰名仍对任意修饰生效）；
  `audit_allow([...])` 显式豁免；审计规则按真实组合校正：同轴才比较嵌套间距、
  `justify_between` 的 gap 视为下限、标题引领的内容向外层看齐、控件与标记按自身边缘对齐、
  字体检查认可声明的回退链、原生输入框参与同行字号检查。
- 组合修正（审计发现的真问题）：区域内的 section 之间用 `section` 间距；DataView 让表格
  `bleed` 到区域两侧，行文字与标题同一条边。
- 证据：`tests/native-keyboard/tests/l2_patterns.rs` 两项：完整 AppShell 组合在
  productivity 规则下零发现（测试平台无系统字体，字体规则交给 Gallery 在真实平台上验）；
  F6 依次框住三个区域、Shift+F6 返回上一个。fmt / clippy（all-features）无输出；
  workspace 667 / 0，native-keyboard 215 / 0，performance 2 / 0。

### 2026-10-04 第 5 步：Gallery 验收应用、BYOD 示例、基线与文档

- 提交：53caeca（组件与运行时修正）、416b8c9（Gallery）、7339995（BYOD 示例）、
  34c0db1（离屏基线）及本文档提交。
- Gallery：`registry/gallery/` 下的 Rhai 应用，基于 AppShell + L2；83 页 / 11 组
  （Foundations 4、Markers 8、Fields 14、Lists 9、Overlays 6、Containers 11、Display 7、
  Interaction 7、Layouts and patterns 12、Scenes 4、Effects 1，合计 83）；四个场景
  （运维、数据浏览、表单、设置）、命令面板、密度/明暗/语言切换、源码检查器、状态栏实时审计数。
  Host（`gpui-rhai-cli` 的 `acceptance` 模块）只提供 Cmd+K 绑定、页面源码文档、审计计数。
  旧 Rust 外壳 `gallery_app` 删除；`--story` 改为无外壳的独立 story 窗口。
- 审计与视觉复查发现并修复的问题（决策 D20–D33）：RTL 下 `justify_start/end` 不镜像（运行时）；
  TitleBar 两行标题在紧凑档被裁；0.2 去掉 Alert 图标后 `info`/`warning` 无人声明、画不出来；
  Slider 数值漂到容器最右；Table 状态列文字与徽标重复；ghost/outline 按钮禁用时多出底色；
  stacked DescriptionList 无分组；弹层触发按钮与 ToggleGroup 被拉满整行。
- 排查记录：命令面板“打不进字”的根因是 Gallery 入口脚本的 `fn index_of(values, key)`
  劫持了 Command 组件里的 `s.index_of(c, pos)`（Rhai 脚本函数可按方法调用且优先于内置函数），
  渲染失败后事务回滚、`last_error` 又被下一次成功渲染清空，表现为静默。已改名并写入
  composition.md、迁移说明和 AgentNotes。
- BYOD：`examples/byod_treemap` 自带调色板、无 token 基础层、无官方组件；进 verification
  manifest，CI 构建、release smoke 启动。disktree-rhai 在临时副本中经 `cargo --config` 冒烟：
  需要三处改动（runtime_api 2→3 是唯一由 0.2 引起的），45 个测试通过、自有皮肤渲染正常；
  未修改其仓库。oh-my-byted 只读扫描，迁移要点写入 `docs/releases/0.2.0.md`，未修改其仓库。
- 基线：本机运行着平铺窗口管理器 rift，会把新窗口改成 852×1067，屏幕截图不可控；改用
  gpui-pre 的 `VisualTestAppContext` 离屏窗口 + `Window::render_to_image` 读回 GPU 纹理。
  19 张 2560×1720（4 页 × 两档密度 × 明暗 + 1 张阿拉伯语 RTL + 2 张简体中文），替换旧外壳的
  31 张；总数 57 = 示例 38 + Gallery 19。**示例的 38 张仍是 0.1.x 外观，待单独重拍。**
- 文档：`docs/gallery.md` 重写；`docs/design/` 补 atoms/composition/decisions/wireframes；
  `registry-design-system.md` 改为指向 `docs/design/`；theming、authoring、locale-and-rtl、
  visual-testing 更新；CHANGELOG 0.2.0（Unreleased）；`docs/releases/0.2.0.md` 含迁移说明与
  disktree / omb 实地报告。crate 版本号仍为 0.1.8，按发版流程再改。
- 证据：`cargo fmt --all --check` 通过（native-keyboard、performance 同）；
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` 退出码 0，
  native-keyboard 与 performance 的 clippy 退出码 0；workspace 672 passed / 0 failed
  （第 4 步 667 + BYOD 2 + acceptance 单测 2 + 资产声明测试 1）；native-keyboard 219 passed /
  0 failed（215 + gallery_acceptance 5 − 删除的旧外壳测试 1）；performance 2 / 0；
  `scripts/audit-visual-baselines.sh` 通过（57 张）；`verify-target-manifest.py` 21 个示例；
  release 版 `gallery --list` 含所需 page/story，`--page scene.operations --density compact`
  与 `--story apps/operations --case large` 各运行 3 秒退出码 0、无报错、启动目录无写入。

### 2026-10-05 0.2.0 发版前：示例重写、38 张示例基线、键盘与 RTL 修正

- 维护者决定：这一轮不叫 0.1.9，已知问题处理完后以 0.2.0 发布。
- 示例：settings_panel、dashboard_layout、form_showcase、data_table、embedded_views 用
  0.2 的 Region / FormLayout / DataView / Section / Stat / DescriptionList 重写，不再有 0.1
  风格的字面几何；每个示例导出 `view(...)` 构造函数和 `WINDOW`，`main` 只读环境变量。
  新增门禁 `tests/native-keyboard/tests/example_audit.rs`：示例在各基线状态下 productivity
  审计零发现。
- 38 张示例基线用离屏采集重拍（`scripts/capture-macos-example-baselines.sh`），用例矩阵不变；
  焦点用例用脚本化按键，normal 动效用手动时钟固定时刻。
- 重写中发现并修复：
  - 键盘：Select、Combobox 各 3 个 tab stop，Popover、Tooltip、DatePicker、ToggleGroup 各 2 个，
    关闭的 Dialog 1 个（应为 0）。根因是 overlay 触发器包装层、带按键处理的容器、tooltip 面板
    被当成 tab stop。修复后新增普查测试 `tab_stops.rs`，19 个交互组件全部符合（D34）。
  - RTL：拉伸列里定宽子元素停在左边（D35）；审计在 RTL 下比较左边缘导致误报，居中单元格
    无起始边（D36）。
  - 组件：FormLayout 控件不再被拉满、帮助文字 unit 间距，Textarea 计数器 xxs（D37）；DataView
    页脚分组间距与两端分布，Table 徽标重复单元格值时替代文字，并按行给徽标实例键（虚拟行里
    重复实例路径），标识符值与过长选项截断（D38）。
- 证据：fmt 通过；workspace / native-keyboard / performance 的 clippy（-D warnings）退出码 0；
  workspace 672 passed / 0 failed；native-keyboard 227 passed / 0 failed；performance 2 / 0；
  基线审计 57 张通过；Gallery 审计门禁与键盘场景通过。

### 2026-10-05 性能度量与优化

- 工具：`tests/native-keyboard/src/bin/gallery_profile.rs`（release、离屏真实 Metal 渲染），
  按交互统计 Rhai 耗时、`Window::draw` CPU 帧耗时、审计耗时的 p50/p95；另有逐页帧耗时、
  任意 Rhai 视图探针、供 `sample` 采样的循环模式。
- 发现与修复（数字均为本机 release 实测）：
  1. Gallery Host 每帧跑一次组合审计，每次约 103 ms，几乎全是枚举系统字体。改为只在提交新
     渲染后审计（新增 `ScriptViewHandle::committed_revision`），字体名缓存、关掉字体规则时不枚举；
     审计降到约 0.1 ms，且悬停/滚动帧不再触发（D41）。
  2. 每帧布局 20–52 ms：被列拉伸、没有自身宽度的子元素先按内容宽度测量、再按拉伸宽度重排，
     逐层叠加。改为渲染成确定的 `width: 100%`：帧耗时约 2 ms；57 张基线与 415 次全页渲染
     逐像素一致（D40）。
  3. Rhai 渲染约一半时间在深拷贝 `UiNode` 子树。`UiNode` 改为 `Rc` 写时复制句柄：页面切换
     15→7.4 ms，120 行数据场景 22→13 ms，过滤框每次按键 10.6→5.1 ms，明暗切换 21→14 ms，
     渲染结果逐像素一致（D42）。
- 剩余：一次交互 5–14 ms 中约 70% 是 Rhai 解释执行（Gallery 根视图每次变化都整体重算导航项、
  命令面板项等），已在一到两帧之内；进一步优化需要应用层缓存静态数据或运行时级的子树缓存。
- 证据：fmt 通过；三个 workspace 的 clippy 退出码 0；workspace 672 / 0，native-keyboard
  227 / 0，performance 2 / 0。

### 2026-10-05 交互问题与运行时诊断

- 维护者试用 Gallery 报告三个问题，逐个用离屏真实渲染和指针模拟复现：
  1. Resizable 拖完报错、一闪就消失。根因：`resize` 事件发出 `{x,y,width,height,handle}`，
     Gallery 原样存成下一帧的 `rect`，而 `rect` 的 schema 不认 `handle`，下一帧渲染失败、事务
     回滚（矩形弹回），紧接着下一次成功事务清掉了 `last_failure`。workbench 的 interaction_lab
     有同样问题。修复：payload 只发 `{x,y,width,height}`，与 `move`/`transform_change`/`rotate`
     一致——受控事件的 payload 就是下一次的受控值（D44，写入组件编写指南）。
  2. 报错看不见是运行时语义问题：任何成功事务都会清错误。改为失败一直保留，直到成功热重载、
     横幅上的 Dismiss 按钮或新 API `ScriptViewHandle::clear_error`；自动化命令改为比较失败计数，
     只报告本命令的失败，不再清掉开发者没看到的错误（D43）。
  3. PanZoom 本身正常（两次拖动累积、状态和画面一致），但 demo 的网格比视口小，平移后露边，
     看上去像"圆被挪走"。Rotatable 只有右上角 32×32 的手柄接收指针，点臂没反应。Gallery
     改为：PanZoom 画布大于视口、显示变换读数和 Reset view；Rotatable 不传手柄，区域内任意
     位置拖动旋转；页面说明写清用途和键盘操作（D45）。
- 运行时诊断剩余项：`gpui-rhai check` 新增 `builtin-shadow` 警告（D46）。实测 Rhai 1.26 的解析：
  方法调用 `x.f(a, b)` 找脚本 `fn f(a, b)`（接收者不计参数），直接调用 `f(a, b)` 找同参数个数
  的脚本函数；规则据此区分两种劫持。扫描结果：Gallery 的 `fn set_locale(ctx, value)`（劫持
  直接调用）、两个 story 的 `fn index_of`、data_table 示例的 `fn filter(ctx)`、diff_viewer 示例
  的 `fn split(ctx, payload)`，均已改名；Gallery、全部 story、Theme Studio、示例应用由测试守住。
- 回归测试：Gallery 三个页面的指针测试（撤掉 Resizable 修复时该测试失败，报错正是用户看到的
  `$.rect.handle: unknown field`）；错误横幅 Dismiss 与 `clear_error`；lint 单测与 check 警告。
- 证据：fmt 通过；workspace 与 native-keyboard clippy（-D warnings）退出码 0；
  `cargo test --workspace --all-features --all-targets` 676 passed / 0 failed（此前 672，新增 4 个）；
  native-keyboard 230 / 0（此前 227，新增 3 个）；performance 2 / 0；57 张基线重新离屏渲染后与
  仓库逐像素一致（gallery 19、dashboard_layout 14、form_showcase 9、settings_panel 9、
  data_table 5、embedded_views 1）。

### 2026-10-05 标题栏、主题切换与圆角风格

- 主题切换：新增 `ctx.theme_variants()`；Gallery 标题栏用一个主题 Select 列出全部 15 个内置主题，
  `--theme` 接受任何内置主题 slug。原来的明暗按钮改成命令面板里的"切换明暗"：同族有另一种明暗
  才切（只有 Default 与 Catppuccin 两族同时有明暗）。（D47）
- 自研标题栏：新增节点 API `.window_drag_area()`，按在节点空白处移动窗口（macOS/Linux 的平台拖动），
  双击执行系统标题栏动作；被可聚焦控件接走的按下（GPUI 获得焦点时会 `prevent_default`）不移动窗口。
  默认无效，宿主需对该视图打开 `ScriptViewConfig::window_drag_areas` / `ScriptApplication::window_drag_areas`，
  保留"嵌入视图不能把内容变成窗口控制区"的既有约束。TitleBar / AppShell 增加 `window_drag`。
  Gallery 在 macOS 上隐藏系统标题栏（透明 titlebar、`app_owns_titlebar_drag`），红绿灯在 (12, 11)，
  标题栏起始留 72pt。用 Swift 发送真实 CGEvent 验证：拖标题栏空白处窗口移动（rift 平铺管理器
  随后把窗口复位），从控件上起拖窗口不动。紧凑密度下红绿灯比文字低约 1pt。（D48）
- 圆角风格：`corners` 环境轴（square 默认 / subtle / round），圆角角色随之取值，新增 `radius.xs`。
  round 下控件与标记为胶囊、方形控件为圆、面板 8px；Checkbox 保持带圆角的方框（否则与 Radio
  混淆），状态灯保持方形，Slider 推子与 Progress 保持矩形，表格/列表/树的选中行保持直角。
  新增逻辑圆角 `radius_start` / `radius_end`（RTL 镜像），ToggleGroup、ButtonGroup 只圆外侧两端，
  Tag 分面段承担起始圆角；竖排 ToggleGroup/ButtonGroup/Tabs 用面板圆角。Gallery 标题栏可切换。（D49）
- 排查：标题栏改动后 `form_scene` 在调试构建的测试线程（2MB 栈）上栈溢出，16MB 时通过，属于递归
  渲染栈帧累积而非死循环。把 `populate_with_interactions` 中挂交互的部分拆到 `#[inline(never)]` 的
  `wrap_interactions`，递归前出栈；该场景所需栈从 >2MB 降到 1.5–1.66MB。
- 证据见本节提交说明与回复。

### 2026-10-07 PR #111 收尾共识（grill 结论）

审查（#111 review，R1–R5）与 13 个 open issue 全部在 #111 内处理，按主题分批提交、每批验证后推送；全部完成后发 0.2.0。

- 发布门槛：去掉 120Hz 物理验证，改为 release 构建 `gallery_profile` 的 `Window::draw` p95 ≤ 8.3ms（Rhai 交互耗时只记录）。真机检查：键盘、焦点、剪贴板、多窗口、标题栏拖动与双击用 CGEvent 自动化取证；IME 预编辑与 VoiceOver 由维护者手动。
- R1 stretch 快速路径只在可证明等价时生效（无横向 margin/auto margin 等），父级 stretch 判断改读实际生效样式；R2 token 热更新按候选整体校验后原子提交，失败保留上一版；R3 Table 键盘两种数据源同一逻辑，Native 从原生投影取相邻/首末 key；R4 三个 phase 共用 qualified 优先、plain 回退的匹配；R5 拒绝不完整 by_env 表并报缺失组合。审查探针修复后全部通过。
- #112 审计进入 error_boundary、Layer 与虚拟列表已实现行；#113 吸顶分组头裁剪到视口；#114 三处误报；#116 shortcut 用平台图例和 Kbd；#117 迁移指南 1–4。
- #115 Toolbar `fill` 槽占起始组与结束组之间的剩余宽度（有最小宽度），DataView 同步 `fill`/`size` 并透传 `inset`。
- #118 Table 分组头用标签字阶；能持有焦点的容器统一 2px 墨色框；AppShell 写清边界不扩展。Region 新增 `scroll`（默认关）与 `external_title`。
- #110 虚拟列表每行一个路径段 `Item[<行 key>]`（不兼容）；#109 overlay 身份为"组件实例路径 + key"，`parent_overlay` 解析最近祖先，`overlay_placement(view_id, key)` 保留且歧义时报错并新增按实例路径查询，shared layout 同样按实例隔离（不兼容）。
- #95 (a) 按 0.2 模型重定义验收（等宽字体由 token base 的 code 角色与宿主覆盖决定，调色板不改字体）；(b) Table 列 `typography: "code"`，两种数据源；(c) span `.typography(role)`（字体族与字重）与 `.background(color)`。
- #89 Table `on_context_request` → `#{ key, column, anchor, source }`，右键先选中该行，支持 Shift+F10 与菜单键，菜单由调用方用 Menu `anchor` 弹出。
- #91 `call_with(&InvocationContext, …)` 默认转发 `call`；上下文含来源（UserInput、Automation、Timer、TaskCompletion、Subscription、Effect、Lifecycle）、view_id 与组件路径；异步来源带最初发起来源。
- #83 SplitPane 与 Resizable 支持装饰性手柄节点（画在面板之上，伸出部分可开始拖动，键盘与无障碍留在原生手柄），新增通用 `.signal_style(state, #{…})`，以及 `line` 与 `line_inset`。
