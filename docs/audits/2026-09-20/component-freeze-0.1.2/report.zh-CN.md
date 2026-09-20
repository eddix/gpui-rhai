# gpui-rhai 0.1.2 组件基础层冻结审计

日期：2026-09-20

结论：现有 51 个官方 Rhai 源码组件构成 0.1.2 的完整基础层。本轮不再
临时扩张目录，而是冻结身份、状态所有权、事件、尺寸、样式部件与可访问性
契约，完成发布级工具链、示例、文档和验证闭环。0.1.3 的主要产品目标转向
通用动效系统。

## 1. 审计范围

- `registry/components` 的 51 个官方组件与 15 个内置主题；
- Runtime API 1、正式组件 schema、CLI 安装/更新/metadata/snippet 路径；
- Theme Studio 与 Component Gallery 的全目录 specimen；
- examples、User Guide、组件规格、发布清单和视觉/原生交互基线；
- shadcn/ui 与 Longbridge gpui-component 的当前公开组件目录及分层方式。

本轮没有把“另一个库存在同名组件”直接解释为缺陷。shadcn 的 Drawer 与
gpui-rhai Sheet、Separator 与 Divider、Field 与 FormField、Dropdown Menu
与 Menu 已经是语义等价关系；大量 Web/Chat 专用组件也不属于原生桌面基础层。

参考资料：

- [shadcn/ui Components](https://ui.shadcn.com/docs/components)
- [shadcn/ui Sidebar](https://ui.shadcn.com/docs/components/base/sidebar)
- [shadcn/ui official registry policy](https://ui.shadcn.com/docs/official)
- [Longbridge gpui-component catalog](https://github.com/longbridge/gpui-component/blob/main/skills/gpui-component/SKILL.md)
- [gpui-component architecture](https://github.com/longbridge/gpui-component/blob/main/docs/ARCHITECTURE.md)
- [gpui-kit layering](https://github.com/longbridge/gpui-kit)

## 2. 冻结边界

0.1.2 冻结以下组件基础契约：

1. 组件 ID、导出名和依赖图；
2. 业务值、open/query/selection 等受控状态的所有权；
3. schema 校验后的语义事件名称与 payload；
4. `xs`、`sm`、`md`、`lg` 尺寸词汇及 Omarchy 方正视觉语言；
5. 声明过的 `parts` 与组件样式表覆盖边界；
6. Runtime API 1、Array/NativeCollection 行为一致性、无
   `gpui-component`/GPUIX 依赖；
7. Theme Studio 和 Component Gallery 对同一正式组件源码的覆盖。

“冻结”不表示永远不增加源码组件，而是以后新增 Breadcrumb、持久 Sidebar、
Tree、Resizable 等组合时，不再复制基础 primitive、焦点系统、Overlay、滚动、
事件或状态机。代码编辑器和终端模拟器继续作为独立平台能力处理。

## 3. 本轮发现与修复

### 3.1 显式可访问名称

旧契约允许 Input/Textarea 用 placeholder 作为名称，也允许 Combobox、Select、
DatePicker、Pagination、Menu、ContextMenu、Popover、Tooltip 和 Progress 产生
空名称或借用提示文本。这会让视觉提示、字段标签和辅助技术名称互相污染。

0.1.2 对上述组件统一要求 `label: string`。内部组合继续显式转发该名称：
Combobox 的搜索 Input、Select 的 Combobox、Pagination 的 page-size Select 和
ContextMenu 的 Menu 都不能靠默认值掩盖遗漏。CLI snippet、Studio、Gallery、
examples、测试和 User Guide 已同步迁移。

### 3.2 装饰与范围语义

- Icon 有非空 label 时为命名 image；省略 label 时固定为 presentation，不再
  暴露空名称图片。
- Divider 保留 separator 角色并暴露方向，不再写入空 label。
- Progress 要求 label，暴露 `value_min = 0`、`value_max = max`，并用一个
  24–4096px 的 width 契约共同驱动轨道、确定/不确定指示器和 Rust 动画边界，
  不再把 200px 分散写死在行为中。
- Toast 使用可由应用本地化的 `dismiss_label`，但不把 LocaleManager 变成组件
  可渲染的前置条件。

### 3.3 组件职责

IconButton 继续独占正方形纯图标动作目标；Button 的 `prefix`/`suffix` 独占
图标与文字混排；普通 Icon 不承担按钮宽度与命中目标。Table 的拖拽列宽和
双击 auto-fit 仍是 Rust 高频路径，Rhai 只接收一次可持久化结果。

## 4. 暂不纳入 0.1.2

- Breadcrumb：当前可由 Text/Divider/Icon/row 透明组合，尚无足够独立行为。
- Persistent Sidebar：是应用布局而不是 Sheet 变体，不在发布收尾期引入。
- Tree/Resizable/RangeSlider：可作为后续正式 registry 增量，但不是现有 51 个
  组件可发布性的阻塞项。
- Editor/Terminal：需要各自的文档模型、输入法/PTY/进程/安全边界，不伪装成
  普通组件补目录。
- 通用 enter/exit/layout/shared-layout、spring/keyframe 和衍生 property source：
  作为 0.1.3 动效主线统一设计，不在组件里增加一次性动画捷径。

## 5. 验收条件

发布提交必须同时满足：

- 全工作区 all-target/all-feature tests、严格 Clippy 和 rustdoc；
- 独立 native-keyboard 与 performance 结构测试；
- 21 个 example 清单、release build/smoke、产物泄漏和 38 个 PNG 基线审计；
- 三个 crate 的 `cargo package`/publish dry-run；
- Theme Studio/Gallery 的 51 组件、15 主题、类别、compact/regular、RTL、
  reduced-motion 与关键 overlay 状态；
- 组件 schema 防回归测试：显式 label、装饰 Icon、Progress 范围；
- 受保护主干 PR、crates.io 依赖顺序发布、精确 `v0.1.2` tag 与 GitHub release。

## 6. 本地发布证据

2026-09-20 的候选提交通过：

- `cargo test --workspace --all-targets --all-features`：350 个 core 单元测试、
  21 个 formal-component 测试、49 个 official-registry 测试、24 个 CLI
  测试、registry snapshot 及全部 example 测试通过；
- workspace 全 targets/features 严格 Clippy 与 `RUSTDOCFLAGS=-D warnings`
  rustdoc；
- 独立 native-keyboard 53/53、严格测试 Clippy；
- performance 结构测试 2/2 通过，两个受控机器绝对耗时基准按设计 ignored；
- 21 个 example manifest、38 个 PNG、默认 release 产物泄漏审计通过；
- 默认 release 的 21 个真实 event-loop examples、Theme Studio 与 DataTable
  selected/loading/empty/grouped 状态 smoke 全部通过；
- `gpui-rhai` 包含 71 个文件并在隔离 package 目录重编译通过；registry
  包含 96 个文件。CLI 的完整 package 验证必须等待精确依赖
  `gpui-rhai = 0.1.2` 进入 crates.io，这是发布顺序约束而非候选缺陷；
- fresh 默认深色 Component Gallery 目视抽查通过；原生测试完成类别与主题
  热切换、表单/overlay/Command/Table 等真实交互验证。

受保护主干提交、GitHub CI、三个 crate、tag 与 release URL 在发布完成后补录。
