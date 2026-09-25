# gpui-pre 升级实施计划

日期：2026-09-25。状态：**实施中；自动化适配通过，发布门槛仍有阻塞项**。

基线：gpui-rhai `0.1.5`，commit `120ef2239079ee5e2205c05a9b982768b096a19a`。
本文件的编写没有修改产品依赖、工具链或运行时代码，也不代表升级已经通过验证。

## 1. 决策与交付目标

将当前官方发行的 `gpui = 0.2.2` 升级到 **gpui-pre 发布的、可追溯到 Zed commit 的较新上游 GPUI**，直接使用 core 与 platform crate 家族。保持源码中的依赖使用名 `gpui`、`gpui_platform`，将发行渠道差异集中在 Cargo 声明与必要的平台入口适配中。

本次必须同时交付三个结果：

1. 现有 Runtime、组件、CLI、Host 嵌入方式在新 GPUI 上正常工作，功能、主题、生命周期与性能不出现未解释的回归。
2. 接通新 GPUI 已公开、而本项目此前受限的原生 accessibility 语义与操作接口，使升级产生可验证的用户收益。
3. 建立完整版本锁定、跨平台验证和发行验收依据，使将来切回官方发行家族成为一次可控的依赖升级。

本轮按 `0.1.6` 组织，保持 `RUNTIME_API_VERSION = 2`。Rust Host 的 GPUI package identity、MSRV、平台入口与必要的公开 Rust API 允许破坏性迁移；现有 Rhai 脚本、官方组件与主题原则上必须保持源码兼容。若实施证明 Rhai 契约必须改变，应暂停并单独决策，不能把变化隐藏在依赖升级中。

`0.1.6` 是单一主题的基础设施版本：范围仅包括 GPUI 家族升级、原生 accessibility、因此必需的运行时／组件／Host／CLI 适配和迁移中发现的阻塞缺陷。不顺便开放动态字体下载、系统设置监听、Kit/Base 能力或其他无关产品功能。

这不是迁移到 Kit/Base 的组件与应用行为体系。**本次不引入 gpui-kit、gpui-base、gpui-component、gpui-ce、JS 桥接或内置 3D 引擎。** Rhai、RetainedUiTree、主题、Motion、Chart Runtime 和资源权限仍由 gpui-rhai 管理；有需要的 Rust Host 可以继续自行集成 3D。

允许为此次升级调整公开 Rust API，不保留旧、新 GPUI 双后端，也不增加历史兼容层。目标是维护一套清晰的当前实现。Rhai 的现有组件与 Runtime API 2 契约原则上不变；若发现确实需要改变这些契约，必须单独列明变化及理由，不能借升级隐式改变。

## 2. 调研依据与实际收益

`gpui-pre` 是社区维护者重打包的上游快照，**不是 Zed 官方发布的独立产品**。crate 的 Zed authors/repository 元数据说明源码来源，不说明发布者身份。其发布流程会报告对 Kit 的构建／测试结果，但并不保证每次快照只有全部测试通过才发布；本项目需要自己的验收。

截至 2026-09-25 复查，已发布候选仍为 `gpui-pre 0.3.6` / `gpui-pre-platform 0.3.6`，官方 `gpui` 仍为 `0.2.2`。0.3.6 记录的 Zed 源码 commit 为 `bcf6582ce3500df93a8a39366640173e6786cea6`，快照日期为 2026-09-21。其元数据中的 `zed-version = "0.2.2"` 是上游 manifest 的版本字段，不能据此误判为与旧 crates.io 包相同的源码。[pre 版本信息](https://crates.io/crates/gpui-pre/0.3.6)、[platform 版本信息](https://crates.io/crates/gpui-pre-platform/0.3.6)、[官方发行信息](https://crates.io/crates/gpui)。

| 已核实的上游变化 | 对本项目的作用 | 实施时的判断 |
| --- | --- | --- |
| 公开 role、ARIA 属性、元素语义写入、synthetic children 与 AX action 接口 | 将内部 AccessibilityTree 投射到系统原生语义树 | 需要本项目实现适配；升级依赖本身不会自动完成 |
| cached View 中 Input 的 paint-range 崩溃修复 | 提高输入控件在 Host 缓存视图中的可靠性 | 增加跨帧、缓存失效与输入焦点的原生回归验证 |
| macOS fallback font 保留粗细／斜体 | 改善中文与拉丁字符混排的一致性 | 在实际字体缺字／fallback 场景验证，不能只测全覆盖字体 |
| macOS accessibility adapter 引用环修复 | 关闭窗口时释放 native view、Metal 等资源 | 上游修复与本项目 teardown 两层均需验证 |
| TestWindow scale-factor 模拟改进 | 能真实覆盖 1x/2x 与 resize 的 DPI 行为 | 用新测试入口验证布局、命中、IME bounds 与像素对齐 |
| 动态字体安装及 generation/cache invalidation | 支持 Host 安装字体后的正确重排 | 不新增脚本自动下载字体或网络权限 |
| Window 可见性、帧调度与平台分包 | 更明确的平台选择与调度接入 | 不把窗口隐藏直接等同于 ScriptView suspend |
| App reduced-motion flag | 有助于协调底层动画 | 不等于已自动读取 OS 偏好；保留 Host 的策略权威 |

对应上游修复包括 [#50665](https://github.com/zed-industries/zed/pull/50665)、[#56771](https://github.com/zed-industries/zed/pull/56771)、[#63719](https://github.com/zed-industries/zed/pull/63719)、[#63498](https://github.com/zed-industries/zed/pull/63498)、[#64143](https://github.com/zed-industries/zed/pull/64143)。这些收益来自较新的 Zed GPUI 源码，不要求先使用 Kit/Base。平台分包、更新 WGPU 等事实也不直接构成“性能会提升”的证据。

已完成的独立 probe 证明：直接依赖 pre core/platform、调用 `gpui_platform::application()` 并构造带原生语义属性的元素可以编译，依赖树不包含 Kit/Base/Component。但这只是 macOS 小工程的编译验证，不是产品迁移、窗口运行或系统无障碍验收。完整调研与证据见[上游核心与 Kit 收益拆分](../research/2026-09-24-gpui-backend-selection/upstream-core-vs-kit.zh-CN.md)。

## 3. 候选版本、工具链与发行限制

### 3.1 版本选择

以 **`=0.3.6` 为适配起点**；最终交付必须锁定一个明确的、通过下述门槛的已发布 pre 家族版本，不能使用浮动版本、Git `main` 或“最新即可”。

开始实施和冻结候选时各检查一次 crates.io：如果存在更新的完整家族，并包含本计划要求的修复，应核对源码、工具链与 feature 差异后，将全家族一次性提升到该版本，并在验收报告记录替换理由。直接依赖与传递依赖应对应同一 Zed snapshot，不能混装多个 pre 版本或旧官方 core。

如果冻结候选前 Zed 官方已经发布具备同等 API、平台分包和关键修复的完整 crate 家族，应把官方包与 pre 放在同一门槛下重新评估并优先采用验证通过的官方发行。目标是较新、可追溯且通过本项目验收的上游 GPUI，不是长期绑定 `gpui-pre` 品牌。

### 3.2 已知候选缺口：文本命中 panic

上游 [#64672](https://github.com/zed-industries/zed/pull/64672) 于 2026-09-23 合并，修复 wrapped-line 在零宽字符等边界命中时的 panic，commit 为 `e71963a599c64c213aca1c607e7418503a1e787d`。**0.3.6 的 09-21 快照没有包含它。** Linux/Web 的 cosmic-text 路径尤其需要关注，不能用 macOS 测试通过代替验证。

默认交付策略：使用包含此修复或等价修复的后续、已发布且版本对齐的 pre 快照。若暂时没有，可继续完成适配工作，但保留发行阻塞项。若拟证明本项目支持路径不可达，必须给出源码路径分析及目标平台回归探针，不能仅凭“目前没有复现”。若需要在项目内做等价防护，应验证所有可达调用路径，而不是只在某一个输入控件吞掉 panic。

不将本地未发布的 Git/path patch 当作可发布方案，不删除上游触发条件来让测试通过。复现材料至少覆盖含 `\u{200b}` 的窄行换行与行宽边缘命中，并与上游回归案例对应。

### 3.3 Rust 工具链

当前项目固定 Rust `1.94.0` / MSRV `1.94`，独立 probe 已在 `std::hint::cold_path` 处失败。该 API 自 Rust **1.95.0** 稳定，因此不能维持原 MSRV 声明。[Rust API 记录](https://doc.rust-lang.org/std/hint/fn.cold_path.html)

实施默认先用 **稳定版 Rust 1.95.0** 验证候选，拟将 MSRV 提升到 `1.95`。这只是有依据的起始候选，**尚未证明整个产品与所有依赖的最低要求就是 1.95**。若最终 pre 快照或传递依赖需要更高稳定版本，P0 阶段确定实际最低版本并统一更新，不反复临时打补丁。

必须同步维护 `rust-toolchain.toml`、workspace `rust-version`、CI、发布清单和用户安装说明。开发工具链使用明确版本，另外保留 latest-stable 检查。不得使用 `RUSTC_BOOTSTRAP`、修改依赖源码绕过编译器要求，或把此前在本机 nightly 上通过的 probe 写成“稳定版已通过”。

## 4. 依赖与平台边界

Cargo 中保留上游惯用的使用名。以下展示 **包身份与 pin 规则**；平台 features 还需按下表配置，不能把这个片段单独视为完整可运行配置：

```toml
[workspace.dependencies]
gpui = { package = "gpui-pre", version = "=0.3.6", default-features = false }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.6", default-features = false }
```

若 P0 选择了更新版本，两个 pin 及独立测试 workspace 同步替换。禁止在业务代码中散布 `gpui_pre` 品牌相关判断；不为这次升级另造一套覆盖全部 GPUI API 的抽象层。

| 目标／用途 | 配置要求 |
| --- | --- |
| macOS 产品 | 核实 core/platform 的 `font-kit` 启用关系，保证实际字形栅格化；单纯文本布局测试通过不够。独立 probe 使用了两者的 `font-kit` |
| macOS shader 构建 | 显式决策 `runtime_shaders`。probe 启用了它，但正式构建需验证启动、shader 编译、产物与工具要求；不把它误写成新增自定义 shader／3D API |
| Linux 产品 | 在 platform 层选择原生 backend；本次以 X11 + Wayland 为默认实施目标，分别验证。core 的同名 feature 不能代替 platform 的 backend 配置 |
| 原生／性能测试 | 保留独立 workspace，只在那里启用匹配版本的 `test-support`；需要平台测试构造器时同步启用 platform 测试 feature |
| Windows / Web / FreeBSD | 不在本轮扩大产品支持承诺；涉及它们的条件编译不能随意破坏，未实测部分按未验证记录 |

Linux 原生依赖与初始化要求以最终 `gpui-pre-linux` 源码为准，更新 CI 系统包清单。当前 CI 的三个 xcb/xkb 包不应被假定为已经足够。平台 features 按目标组织，避免 macOS 构建偶然统一的 feature 掩盖 Linux 缺失后端。

需要修改、核验的 manifest / lockfile 至少有三组：

- 根 `Cargo.toml` / `Cargo.lock`，以及 `crates/gpui-rhai/Cargo.toml` 的平台依赖。
- `tests/native-keyboard/Cargo.toml` / `Cargo.lock`。
- `tests/performance/Cargo.toml` / `Cargo.lock`。

CLI 当前主要经 `ScriptApplication` 启动，不应无理由再创建自己的平台封装。项目已经公开重导出 `gpui`，应保持其指向唯一的当前 core；如需向 Host 提供 platform 入口，可在同一公共入口重导出 `gpui_platform` 并形成一条明确用法。

**Host 必须与库使用同一个 package 身份和版本家族。** `gpui::App/Window/Entity` 在旧官方包与 pre 包之间不会因为使用名相同而变成同一种 Rust 类型。外部 Host 示例、定制 primitive、HostSlot 和扩展说明都必须验证这一点。

## 5. 实施顺序与阶段出口

### P0：冻结候选与取得可比较基线

1. 记录实际实施起点、已有工作区变更及验证状态；如果已偏离 `120ef223`，明确新增提交，避免覆盖其他会话工作。
2. 确定完整 pre 家族、Zed commit、crate checksum、稳定工具链、平台 features；核实第 3 节的已知修复状态。先用独立小工程验证 macOS 与 Linux 的依赖、启动／测试入口及工具链，不把产品同时改到半升级状态。
3. 在原 GPUI 上跑现有检查和关键产品场景，保存结果。性能 A/B 尽量让旧、新 GPUI 都使用候选稳定编译器，以免把编译器差异算成后端收益；如旧版本不能使用该编译器，明确比较限制。
4. 写明最终的 shader 编译政策与 Linux 后端选择。记录锁文件和依赖图，确认没有偷偷依赖本机 patch 或 Cargo 全局配置。

**出口：** 候选能在稳定 Rust 上构建、平台选择明确、旧版基线可复查；已知发行阻塞项单独保留。P0 不要求等待新快照才能开始其他可独立推进的适配。

### P1：迁移包家族、入口与公开 Rust 边界

1. 同步三个 workspace 的依赖与 lockfile，以及工具链、CI 配置。
2. 将真实平台的原始 `Application::new()` 入口迁移到 `gpui_platform::application()`。当前明确命中为 `src/app.rs`、`examples/phase0_probe.rs`、`examples/embedded_views.rs`、`examples/host_owned_tree.rs`。同时全库重新搜索，不把此清单当作永远完整。
3. 保留 gpui-rhai 自己的 `ScriptApplication::new(view)` 语义。其内部平台启动适配不应扩散为每个应用都要手动维护初始化顺序。
4. 按实际新源码适配 Context、Window、Task、Element、Canvas/Image、文本与平台事件接口。逐调用点确认返回值与所有权，不为消除编译错误机械增加 `unwrap`、`detach` 或吞掉 `Result`。
5. 验证 Host-owned tree、嵌入既有 GPUI 应用、custom primitive、HostSlot 和公开类型的组合编译与运行。

**出口：** 根 workspace、两个独立测试 workspace、全部 23 个示例目标及 CLI 能编译；依赖图不存在新旧 GPUI 混用，产品依赖图没有测试特性污染。后续若修改示例清单，以 verification manifest 的一致性检查为准。

### P2：渲染、文字与资源行为适配

优先核验 `renderer.rs`、`primitive.rs`、`style.rs`、`geometry.rs`、输入／文档元素、虚拟列表／Overlay、`asset.rs`、`inline_svg.rs`、`font.rs` 与 Chart/Canvas 的 Element 实现。

- 新版 layout、prepaint、paint、hit-test 的时序必须保持同一帧的几何与交互一致；禁止通过重复布局、额外整树重建来掩盖适配问题。
- 验证输入 focus、缓存 View、UTF-16 IME 范围、grapheme、selection、clipboard、换行与混合字体；包含 #64672 的实际触发条件。
- 验证 scroll offset、裁剪、捕获、overlay 定位与跨 DPI resize。不要把逻辑尺寸、物理像素、窗口 bounds 和 chart plot bounds 混用。
- 现有 SVG 适配处理旧 GPUI 的 RGBA/BGRA、预乘 alpha 路径。先对新 decoder 做小型颜色／透明度特征验证，再决定是否整段删除旧适配；不能凭“已经升级”直接删除，也不能在新正确路径再次交换通道。`currentColor`、内嵌／外部 SVG 与字体相关渲染均需覆盖。
- 不为减少 Cargo 重复项而顺便强制统一项目自身 `usvg/resvg` 版本；确认输入、安全限制与输出行为后再做有必要的升级。
- 动态字体安装如与现有 Host API 相交，验证字体 generation 更新能失效相关文本缓存；没有采用的新能力只记录，不扩大脚本权限。

**出口：** 实际产品窗口的文本、图形、交互与资源加载通过功能和视觉回归；已知 workaround 的保留／删除均有行为证据。

### P3：接通原生 accessibility

使用已存在的 RetainedUiTree / AccessibilityTree 作为语义权威，增加到 GPUI 公共 AX API 的投射。不要让原生树、自动化快照、组件 metadata 各自重新推导一套状态。

实现时应把语义权威收敛为同一提交边界中的 `CommittedSemanticFrame`（名称可按代码语境调整）：候选 retained tree 产生一次完整、不可变的帧语义投影；自动化快照与 GPUI 原生 AX 都消费这个投影；geometry、paint、hit-test、AX 与 presentation generation 对齐。不能用上一帧 `GeometryRegistry` 临时拼下一帧 AX，也不能让 renderer 再从原始 attributes 推导第二套角色、状态或 action。现有 `AccessibilityNode` 需要补足 selected、placeholder、集合／表格位置、action capabilities 等实际契约字段。

1. 建立逐属性映射表：role、name/description、disabled、value、checked/pressed/selected、required/invalid、关系与范围等。能使用公共 API 的直接映射；未支持属性准确列为平台缺口，不伪装已支持。不要假定所有属性都有 fluent helper，核验 Element 语义写入与 synthetic children 扩展点。
2. 沿用已有稳定身份、布局扁平化与已呈现节点规则；关系 ID 必须在目标窗口／视图正确解析。保持 keyed reorder 中的语义身份，同时用生命周期 generation 拒绝旧 action。不要以每帧新 ID 的方式规避失效问题。
3. 原生 activate、focus、set value 等操作进入已有的有序事件／状态提交流程，遵守 disabled、controlled state、限额与失效检查。没有相应操作能力的节点不对系统宣告该 action；避免系统 action 与鼠标／键盘处理造成双触发。
4. Input/Textarea 继续使用原生文本输入与选择模型，避免同一控件出现两个可编辑语义节点。Overlay、Dialog、菜单、Tabs、ToggleGroup 的语义与真实 focus boundary 一致。
5. 虚拟列表、Table、Chart 使用有界的语义投射。按产品当前暴露的信息与导航能力提供结构，不能为了“完整”在每帧实例化 100k 行／点，或承诺并不存在的屏幕阅读器数据探索能力。
6. 覆盖卸载、错误回滚、热重载、窗口关闭与暂停恢复；旧节点不能残留为仍可操作的系统对象。AX 的后台请求不得直接进入前台 Rhai/GPUI 状态。
7. 原生 AX action 只是新的输入来源，不是新的授权来源。它必须进入现有 semantic action／controlled-state 流程，不得绕过 disabled、read-only、Host capability、资源权限、callback 限额或 generation 检查，也不新增一套 Rhai 回调协议。
8. 原生 AX 只在系统请求时构建；内部语义树、测试查询和 agent 自动化继续始终可用。平台适配器的瞬时失败保留 last-good 可视 UI 与内部语义树并产生诊断；静态无效语义在 check／准备阶段拒绝。
9. 全部官方交互组件必须进入完成度清单，逐项标记“可感知且可操作”“仅可感知”或“纯装饰、不进入 AX”。用户自定义 Rhai 节点遵守相同 role／state／action 契约，不把原生 AX 变成官方组件特权。
10. Rust custom primitive 获得受生命周期、generation 和预算约束的原生语义扩展点。简单 primitive 可由外层节点描述；Chart、CodeViewer 等复杂 primitive 可以提供有界 synthetic children 和 action。
11. 无效 role／状态组合、重复 ID、宣告但没有实现的 action 在 `gpui-rhai check` 或准备阶段报错；平台暂时无法表达的合法语义保留在内部投影、输出诊断并安全降级，不崩溃也不虚假注册 action。
12. 可访问名称优先来自应用可见 label、description 和现有 locale 文案。Runtime 不生成固定英文或暴露内部 ID／组件名作为替代；没有可见文字的图标按钮等交互节点必须显式提供本地化名称。
13. EmbeddedScriptView、Host-owned tree 与 HostSlot 的语义节点组合进 Host 已有窗口 AX 树，保持单一窗口 root、焦点链和 action 路由，不创建第二棵并行平台树。

**出口：** 原生 AX tree 测试与实际系统操作均证明全部官方交互组件按清单可感知、可操作，而不只是内部 snapshot 正确。macOS VoiceOver 手工验证需记录操作和结果；Linux X11／Wayland 按同一产品边界分别实测并列明平台差距。未验证的平台不能写成已完成认证。

### P4：保持生命周期、调度与主题的一致性

- 新 Window visibility 只是平台状态。保持 ScriptView 的显式生命周期契约，不能自动把隐藏窗口变成 `Suspended`，也不能在恢复显示时漏掉已有 prepare/commit 或补偿流程。
- 复查 Task 所有者、取消路径、WeakEntity、订阅与异步结果 generation。窗口销毁、source 替换、热重载和失败回滚后不得有旧任务继续提交。上游 AX 引用环修复不能代替这层验证。
- Motion 继续使用现有 active-time 与集中策略；不能接入第二个互相竞争的时钟。若同步 GPUI 的应用级 reduced-motion flag，由 Host 在应用边界决策；多个 view 不可在 render 中反复互相覆盖全局值。自动读取 OS 偏好作为单独明确能力，不能因升级就宣称已经具备。
- Chart 保持 pending/presented、数据版本、类型化联动视口、独立 X/Y 逻辑窗口、Geo plot 几何与暂停恢复的统一模型。
- ThemeManager、主题 token、Style resolver 继续是视觉权威。不得为匹配新版截图或默认控件外观写死颜色、padding、gap、radius。验证直角／圆角、疏密、明暗主题切换、RTL 和运行中换主题。

**出口：** 无跨窗口残留、错误状态泄漏或后台持续调度；主题切换与实际控件样式一致，关键性能结构仍成立。

### P5：工具链、打包、文档与交付闭环

1. 更新 CI 与 release smoke，保留 Linux 自动化及本地 macOS 原生验收方式，不假定可以新增付费 macOS CI。检查 product、default、charts、all-features 及独立测试图中的 features。
2. 审查 `scripts/tool-wrappers/xcrun` 对 Metal 调试信息和模块缓存的处理，以及 `scripts/audit-release-artifacts.sh` 的路径例外。包拆分会改变构建产物路径；只保留经验证的窄例外，不放宽成忽略全部 target/build，不为了通过构建关闭产物审计。
   普通 macOS crates.io 用户不应再被要求手工运行 `xcodebuild -downloadComponent MetalToolchain`。P0/P5 必须用冷启动、运行期编译失败、包体、产物泄漏和干净机器构建证据决定 `runtime_shaders` 或等价发行策略，而不是把构建问题转移到用户环境。
3. 在全新外部 Cargo 项目验证 CLI init/dev/embed、生成代码、复制组件、Host snippet 与 source update。检查实际打包文件，而不只验证 workspace path dependency；CLI 当前不需要直接依赖 GPUI 的路径也应继续正常。
4. 更新当前文档中的依赖、MSRV、启动方式、平台配置、原生 AX 能力及剩余限制：README、USER_GUIDE、embedding、自定义 primitive、accessibility、assets、release checklist、第三方许可等。对 bidi-isolation 等旧限制逐项重新核实，不随版本号一起武断删除。
5. 历史 release notes、审计报告、probe 锁文件与基线元数据保持当时事实，不能全库替换 `0.2.2`。需要复跑历史探针时，在本轮验收目录迁移测试 harness，保留原版与适配说明。
6. 建议按下一次 `0.1.x` 发布组织交付，若版本未被占用可用 `0.1.6`；发布说明明确 Rust Host 依赖家族与 MSRV 的 break。没有 Rhai 契约变化时保留 Runtime API 2，不为了依赖升级虚增 Runtime 代号。
7. CLI 不自动重写用户 `Cargo.toml`。`gpui-rhai check` 应识别官方 `gpui 0.2.2`、混装 package 家族和不一致版本，给出精确且可复制的迁移声明；外部 Host 文档以 `gpui-rhai::gpui`／`gpui-rhai::gpui_platform` 为推荐类型入口。

**出口：** 可复现的 package/release 产物、外部 Host 验证和全部证据齐全，才给出“可发布”结论。具体发布操作按项目正常流程进行，编写本计划不等于执行发布。

## 6. 必须完成的验收

### 6.1 自动化基础检查

以下沿用当前仓库入口；在候选 lockfile 生成并冻结后执行，不能用忽略失败或删除断言替代适配：

```sh
cargo fmt --all -- --check
cargo fmt --manifest-path tests/native-keyboard/Cargo.toml --all -- --check
cargo fmt --manifest-path tests/performance/Cargo.toml --all -- --check
python3 scripts/verify-target-manifest.py
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo test -p gpui-rhai --lib --locked
cargo test -p gpui-rhai --lib --features charts --locked
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --all-targets --locked -- -D warnings
cargo clippy --manifest-path tests/performance/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked
cargo test --manifest-path tests/performance/Cargo.toml --locked
RUSTDOCFLAGS='-D warnings' cargo doc -p gpui-rhai --all-features --no-deps --locked
RUSTDOCFLAGS='-D warnings' cargo doc -p gpui-rhai-cli --lib --no-deps --locked
bash scripts/audit-visual-baselines.sh
cargo build --release -p gpui-rhai --examples --locked
cargo build --release -p gpui-rhai --example chart_gallery --features charts --locked
cargo build --release -p gpui-rhai-cli --locked
bash scripts/audit-release-artifacts.sh
```

此外执行最终 MSRV 和 latest stable 两套检查；按 release checklist 对 core、registry、CLI 执行 package 及外部安装验证。未发布的 workspace 联动版本导致 CLI 暂不能从 registry 完整验证时，沿用已有依赖顺序与候选验证流程，最终发行验证必须闭环，不能把 `--no-verify` 当作产品通过的证据。

Cargo graph 检查使用实际 target 的 `cargo metadata` 和带 features 的依赖树：确认唯一 GPUI 家族、无 Kit/Base/CE/Component、无产品 `test-support`，并检查 release 的调试／inspector 排除规则。仅检查 `cargo tree -d` 不足以证明语义上唯一的 GPUI。

### 6.2 行为与平台矩阵

| 范围 | 必须证明的结果 |
| --- | --- |
| 三种使用方式 | ScriptApplication 独立应用、嵌入现有 GPUI Host、Host-owned tree 均可运行；HostSlot 与 custom primitive 正常 |
| 平台 | macOS 真窗口启动／关闭／多窗口；Linux X11 与 Wayland 分别做真窗口输入、文本和渲染 smoke。headless 测试不替代 backend 运行 |
| 输入与文本 | CJK IME commit/cancel、UTF-16 与 grapheme、焦点迁移、缓存 View 输入、wrap 边缘、fallback bold/italic、复制选择无回归 |
| 原生 AX | 全部官方交互组件有完成度清单；Button、Checkbox/Toggle、Input/Textarea、Slider、Tabs、Dialog、列表／Table 的真实语义和可支持操作正确；自定义 Rhai 节点与 custom primitive 走同一契约；disabled 与 stale action 不执行 |
| DPI 与命中 | 1x/2x 切换和 resize 后的文本、指针、IME candidate bounds、Overlay、Canvas/Chart 命中一致 |
| 资源 | SVG/PNG 色序、半透明边缘、currentColor、字体安装、内嵌与外部资源一致；无新增未授权脚本 I/O |
| 生命周期 | 关闭／重开窗口、卸载、热重载失败、暂停／恢复失败及补偿、异步迟到结果不破坏 last-good UI |
| Motion / Chart | 重绑 retained target、domain teardown、候选预算、连续性；图表联动、不同尺寸 Geo、轴向逻辑窗口、导出与暂停恢复保持已修复行为 |
| 主题与 UI | Button/Badge 密度、Tabs 的轨道内边距、token 化圆角／间距、明暗切换、RTL、focus/disabled 状态与正式规格一致 |
| 发行 | release 无工作区绝对路径／调试接口泄漏；生成 Host、嵌入产物与打包后运行均通过 |
| Dogfooding | 至少一个现有 Rust GPUI Host 嵌入项目和一个主要由 Rhai 编写 UI 的真实应用完成迁移；必验矩阵由实际代码变化生成，覆盖启动／输入／主题／Motion／Chart／AX／多窗口与关闭 |

截图只来自本项目确定性测试场景，记录主题、字体、DPI、视口、状态与变化理由。不得加入网上找到的第三方参考截图，也不能用“底层升级必然不同”自动接受所有视觉差异。

### 6.3 性能与资源稳定性

使用现有 `scripts/benchmark.sh`、performance probe、Chart release smoke 与 verification manifest；显式 benchmark 当前被 ignored，必须实际运行对应入口并保存结果，不能将普通测试通过写成 benchmark 通过。

- 比较前后使用相同机器、release profile、编译器、features、数据、字体、窗口大小、DPI 和 warm/cold 条件。建议每个稳定场景至少预热 5 次、记录 30 个样本及分布；耗时场景可调整次数，但报告必须公开采样方法。
- 同时记录总耗时、输入到呈现延迟、p50/p95、分配／内存趋势与关键阶段指标。100k 数据更新不是“每帧必须在 8.33 ms 内完成”的同义词；120 Hz 动画遵循原有独立帧预算。
- 保持高频 native/chart/motion 路径不逐点、逐帧调用 Rhai，不退化为整树／全数据重建；保留结构性断言。
- 新增 AX 后测试大数据／虚拟化场景的有界性。重复开关窗口、替换数据和 reload 后，Entity、订阅、任务与 GPU/native 资源不能出现随循环次数持续增长的趋势。
- Table、虚拟列表和大型 Chart 使用“整体结构元数据＋有界语义窗口”。辅助技术导航可以按需 materialize 相邻内容，但不得一次创建全部 100k 行／点。Chart 暴露标题、系列、坐标范围、当前聚焦／选择和可导航的可见或采样数据，而不是原始数据全量节点。
- 沿用 `docs/performance.md` 的既有预算与功能基线；对重复出现、超出噪声的回归必须定位原因。不能只用平均值或换机器后的单次数字宣告提升，不能因升级临时放宽预算。

## 7. 发布门槛、回退与未来官方切换

### 7.1 何时可以标为完成

- 稳定工具链与三个 workspace 的完整依赖图可复现，最终版本没有未处理的已知可达 panic。
- P1–P5 的出口全部满足，自动化与声明支持平台的原生验收齐全。
- 新原生 AX 适配真实工作；未支持能力准确列明，不把内部树存在当作系统支持。
- 现有功能、主题 token、生命周期和性能无未处理的阻塞回归。
- 包、CLI、外部 Host 和 release 产物检查完整；报告包含最终 commit、命令、结果、证据路径及未验证范围。

若只完成依赖编译，应标为“适配中”；若因候选修复或平台环境缺失不能验收，应标为“待发行验证”并明确具体缺口，不能写成“全部通过”。

### 7.2 失败时如何回退

升级按可审查提交组织：候选与基线记录、依赖／平台适配、语义与行为适配、验证和文档。提交之间可以有实施依赖，最终合入对象必须完整通过。

如候选不能达到发布门槛，保留独立升级工作成果并继续使用原发布版本；必要时回退整组依赖、lockfile、工具链与适配提交。**不在产品里留下用 feature 切换旧／新 GPUI 的双实现。** 本轮不涉及用户数据格式迁移，不应引入额外的数据回退负担。

### 7.3 将来切回官方包

是否切回以实际发行事实决定，不以尚未核验的发布频率承诺决定。满足以下条件后再执行：

1. 官方已发布所需 core/platform/macros/平台实现等完整家族，能够从 registry 正常解析。
2. 对应源码拥有我们已使用的 API、平台行为和关键修复，不是名义版本较新但实际代码回退。
3. 对比官方与当前 pre 的源码、重打包差异、features、MSRV、构建资源；不是仅看版本号是否相同。
4. 更改 Cargo 中的 package 来源与精确版本，必要时调整少量平台入口；保持 `gpui` / `gpui_platform` 使用名。让外部 Host 同时对齐并重新跑本计划的相关门槛。

预期优势是**大部分已迁移到上游新 API 的代码可以继续使用，切换发行来源的改动相对集中**。不能保证未来只改一行：上游后续 API、包拆分或 feature 变化仍需评估，跨 package 的 Rust 类型也不能混用。

## 8. 给实施会话的交付清单

- 最终候选说明：pre 家族版本、Zed commit、checksum、稳定工具链／MSRV、目标 features、#64672 等修复状态。
- 完整代码和 Cargo 变更；没有未经说明的 API 语义变化、双后端或兼容包装。
- 原生 AX 映射与行为说明，既有 workaround 的保留／删除依据。
- 自动化、真实平台、外部 Host、package/release 和性能 A/B 报告；检查未运行与运行失败必须区分。
- 当前用户文档与发布说明，明确剩余限制与 Rust Host 升级方法。
- 最后给出“可以发布／仍有哪些具体阻塞项”的结论，不以测试数量替代验收结论。

可将下面这段连同本文件交给实施会话：

> 按本计划将 gpui-rhai 从官方 gpui 0.2.2 升级到直接使用 gpui-pre 家族，先冻结通过稳定 Rust 验证的候选。保持一套 Runtime、主题和生命周期模型，完成平台入口、原生 accessibility、Host/CLI 与测试适配。0.3.6 仅为起点，必须处理 #64672 的版本缺口。不要引入 Kit/Base/CE 或旧后端兼容层，不把单纯编译通过视为完成。最终提交代码、更新正式文档，提供可复查的行为、平台、打包与性能证据，并列明发行阻塞项。

实施完成后，将持久契约合并到 architecture、embedding、accessibility、assets、release 文档及必要 ADR；将实际验证材料保留在本轮审计／验收目录。按仓库现有文档政策删除这份临时实施计划及索引入口，由 Git 历史保留决策过程，不把过时的升级步骤长期当作组件或架构规格。
