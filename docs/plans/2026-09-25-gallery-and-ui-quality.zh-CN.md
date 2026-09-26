# Acceptance Application、Gallery 与示例体系实施计划

日期：2026-09-26。状态：**已确认，实施中**。

实施基线：`4ee713a4`，基于 gpui-pre `0.3.6` 候选分支的堆叠开发线。目标版本暂定 `0.1.7`；本计划不改变 `0.1.6` 的 gpui-pre、平台适配和原生可访问性发布边界，也不能绕过该版本尚未完成的上游与实机门槛。

## 1. 目标与非目标

本轮同时解决两个问题：公开能力散落在 23 个 Cargo examples 中；项目缺少一个展示推荐 Rust Host/Rhai View 架构的完整参考应用。

正式交付是 `gpui-rhai gallery`，包含两个一级区域：

- **Explore**：官方 components、Motion、Chart 的 story explorer。
- **Operations Workbench**：一个连贯、可完成任务的本地运维参考应用。

本轮不实施 Omarchy 系统主题适配、自定义 UI zoom、第三方 story 插件、浏览器/WASM Gallery、在线编辑器、CodeEditor、Terminal、SSH、真实命令执行、网络后端或持久化数据库。Omarchy 适配未来作为独立 `gpui-rhai-omarchy` crate；UI zoom 未来单独写 ADR 和 Runtime 工作包。Windows 不作为本轮实机阻塞项。

## 2. Explore 与 Workbench

Explore 提供搜索、分类、具体对象、cases、真实预览、用途、观察目标、当前运行源码、必要 Host 接入和正式文档。搜索或分类过滤不能重置当前 story；选中对象、键盘 cursor 与 focus 分离。

每个 story 包含稳定 ID、用途、关键词、公开 module IDs、运行 source、cases、fixture、feature/platform 条件、测试要求和文档链接。公开能力必须有 story、测试或书面豁免，覆盖从 registry 枚举，不能写死组件数量。

Operations Workbench 包含：

- Dashboard：状态、趋势图、告警和事件。
- Hosts：搜索、排序、选择、分页、分组和富状态单元格。
- Configurations：CodeViewer、DiffViewer、主机配置对照和部署参数。
- Deployments：状态机、进度、时间线和 Motion。
- Settings：主题、语言和 Motion preference。
- 全局 Command、Dialog、Sheet、Toast 和键盘导航。

核心任务是：选择主机 → 查看指标 → 选择对照主机并查看配置差异 → 修改部署参数 → 启动模拟部署 → 确认或取消 → 查看真实进度与状态更新。所有页面消费同一会话业务模型，取消不得修改数据；不能用“clicked”日志冒充结果。

## 3. 所有权、数据与生命周期

- Rust Host 负责窗口、生命周期、确定性 fixture、`NativeCollection`、`NativeChartData`、粗粒度 capability 和原生扩展。
- 导航、页面组合、业务状态、主题选择和绝大多数交互由 Rhai 编写。
- Workbench 只使用公开 Runtime、registry components 和 Host API，不使用 crate-private hook、仓库相对路径或 Gallery 特供构造器。
- Acceptance Application 暴露的问题修正式组件或 Runtime；禁止 Gallery 私有 wrapper、颜色、固定尺寸或事件补丁。

Rust fixture 提供版本化的主机、指标、配置、部署和事件，内置 normal、loading、empty、partial failure、streaming update 和 large dataset。默认不联网、不访问真实配置、不执行命令，也不创建 100k 数据或持续高频流。

Workbench 在页面切换时保留状态。Explore story 独立拥有状态，最近实例按预算挂起；释放后从初值开始。“Reset story”只重置当前 story。主题、语言、窗口尺寸和 Motion preference 不清空业务状态。关闭进程后不承诺恢复。挂起、重置和关闭必须停止 producer、task、subscription、timer 和 Motion，并拒绝迟到 generation。

## 4. 同源 story 架构

```text
registry：正式组件、主题、语言、story source 与 metadata
                              ↓
gpui-rhai-cli：Gallery shell、Operations fixture 与装配
                              ↓
gpui-rhai Runtime：公开状态、主题、输入、布局与生命周期

独立原生测试 → 复用同一 story/fixture → mount / draw / input
```

story source/metadata 放入 `registry/stories/` 并随包分发；Rust fixture 位于 CLI Gallery 模块。页面代码直接来自实际运行文件或验证过的 source range。完整 Rhai source 包含 import、state schema、回调和 view；需要 Rust 时另列最小 Host 接入。cases 改 state/props，不反复拼接和编译 source。

Theme Studio 只复用精选设计验收 stories，不加载大数据、流式 Chart、完整 Workbench或压力场景。第一版不提供第三方 story 注册。

## 5. CLI 与 examples

```sh
gpui-rhai gallery
gpui-rhai gallery --list
gpui-rhai gallery --story components/tabs
gpui-rhai gallery --story apps/operations --case config-diff
gpui-rhai gallery --theme default-dark --locale zh-CN
```

`--list` 不创建窗口；未知 ID 返回错误。启动不能读写当前目录，也不要求网络。安装版可查看内嵌源码；只有仓库模式可以打开本地文件。

`examples/README.md` 是唯一索引。顶层 `examples/` 只保留面向用户、值得复制的教程和接入示例；性能、native smoke、历史 probe 和 fixtures 进入分类子目录并由显式 Cargo targets/验证清单运行。

现有 Component/Motion/Chart Gallery 先迁为 stories；统一入口、文档、视觉基线、release smoke 和测试迁移后删除重复目标，不保留 shim。hello/embedding/extension/multi-window 教程保留；性能基线和仍有价值的 native smoke 保留；被正式测试完全覆盖的 probe 删除或重命名。

## 6. Dogfooding issue 映射

| Issue | 正式修复位置 | Acceptance 覆盖 |
| --- | --- | --- |
| #68 Host theme overrides | 复验现有通用 override 后关闭 | Settings + 主题代表 story |
| #70 HostSlot 阻断 IME | HostSlot/键盘传播与原生输入测试 | Host 嵌入表单；macOS 中文 IME |
| #71 Table 富单元格 | Table cell/adornment API 与虚拟化 | Hosts 状态/部署标记列 |
| #73 Chart wheel 吞滚动 | Chart interaction policy | 可滚动 Dashboard 的 Cartesian/Geo Chart |
| #74 Chart 错误不可观测 | primitive/view diagnostics 与 invalid semantics | broken-spec case 和 CI 门禁 |
| #75 空 Chart title 占位 | Chart spec/adapters | 无标题/显式标题对照 story |
| #76 Chart 相对高度不呈现 | Chart frame/layout 提交 | Dashboard 响应式卡片 resize |
| #77 Tabs root 收缩 | 正式 Tabs root layout | content Tabs 内 fill-height Table/Chart |

用户可观察问题进入 story/Workbench；竞态、资源回收和性能问题进入原生测试。Issue 仅在正式修复、自动回归及适用 acceptance case 通过后完成，不能把每个历史 bug 变成永久导航项。

## 7. 验收矩阵

- 所有 stories：默认环境实际 prepare、mount、draw，并执行至少一个适用交互。
- 所有内置主题：共享设计验收 stories。
- Workbench：Default Light/Dark 完整走通 normal/cancel/failure 流程。
- 关键 stories：中英文长文本、Arabic RTL、compact/regular/wide。
- Motion：normal/reduced/none。
- 高风险专项：Input/Textarea IME、HostSlot、Table、Overlay、Chart、Code/Diff、键盘和 AX。
- macOS：键鼠、中文 IME、Overlay、VoiceOver/AX、resize 和视觉。
- Linux：X11、Wayland、输入、滚动、Overlay 和基础 AT-SPI。

自动门禁包括 fmt、严格 Clippy、workspace/all-feature/default/charts、独立测试 workspaces、rustdoc、package、release smoke 和 artifact audit。新增 CLI Gallery 与代表 story/case 进入 smoke。不能以 headless draw、“窗口没崩”或延长 sleep 代替实机结论。

## 8. 工作包与出口

| 工作包 | 产物 | 出口 |
| --- | --- | --- |
| A | story schema、registry source/metadata、coverage/package 检查 | 稳定 ID、引用和首批垂直 stories 可运行 |
| B | CLI Gallery、Explore 导航/搜索/cases/源码/reset | 新用户无需读 Rust 源码即可发现和操作能力 |
| C | Rust fixtures、Rhai Workbench pages、部署流程 | normal/cancel/failure 跨页面任务通过 |
| D | #68/#70/#71/#73–#77 正式修复与回归 | 无 Gallery 特供 workaround；issues 达成关闭条件 |
| E | examples 分层、Theme Studio 复用、文档和旧入口清理 | 所有旧目标有去向，无重复权威示例 |
| F | 自动化、实机、视觉、生命周期、性能和包验收 | 第 7 节完成，明确记录通过/失败/未运行 |

A–F 是内部顺序，不是可发布半成品。先用 Button/Badge、Tabs、Input、Overlay、Table、Motion、Chart 和一个 Workbench vertical slice 验证架构，再扩展完整覆盖。

持久文档最终归入 `docs/gallery.md`、`examples/README.md`、`USER_GUIDE.md`、正式设计规格和 release/visual/performance 文档。本文件是临时任务书，完成后移除入口，由 Git 历史保留。

完成不以页面数、测试数或截图数判断。只有当用户可通过 `gpui-rhai gallery` 找到能力、运行真实源码、完成 Operations Workbench 跨页面任务，并且同源自动化与平台实机验收通过，Acceptance Application 才算完成。
