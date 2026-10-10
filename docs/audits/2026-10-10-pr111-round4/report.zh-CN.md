# PR #111 第四轮独立复审：新增控件、open issues 与两个 skills

审查日期：2026-10-10。审查 head：`95253c65d5f97a1e64ca51567255e6337ae02c96`；上一轮已审 head：`2c5e8a079304bb0c8910ddbd73eca772f49fd453`。

**结论：当前 head 尚不宜合入或发布。新增 2 个 P1、3 个 P2；另有已有 issue #131 未修。** 原先的 10 个独立回归探针全部通过，没有发现这些已验收问题回退。新问题主要出现在新增组件的状态变化和文档生成结果的实际消费环节。

当前 26 个 open issue 中，24 个在本轮核查范围内可以随 PR 关闭；#119 虽然已交付 List，但 U2 阻止完整验收；#131 尚无修复。这里只给关闭建议，没有操作 issue 状态。本材料仅在独立 review 分支保存，不要求把审计历史重新合入产品树。

## 一、需要修复的代码问题

### U1 / P1：TabBar 无法挂载空集合，关闭最后一个标签导致事务失败

位置：[tab_bar.rhai:203](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/registry/components/tab_bar.rhai#L203)、[reveal effect:366](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/registry/components/tab_bar.rhai#L366)。

`reveal` 无条件注册，start 无条件执行 `ctx.scroll_into_view("cursor-tab")`，但 ref 只绑定到与 cursor 匹配的标签。`tabs: [], value: ""` 无匹配节点，mount 返回错误。更常见的路径是用户关闭最后一个文档：调用方在 `on_close` 中清空 tabs/value，随后的 effect 报错，正常关闭事务不能提交。

独立探针的实际错误：`component /View[tabs]/TabBar[files] has no mounted element ref cursor-tab`。这不是要求兼容旧 API，而是新组件应支持的空状态。

建议：明确空列表和无有效 cursor 时的行为；没有实际目标时不要注册或执行 reveal。另检验删除当前项、从空集合增加首项、全部禁用这些状态转换。不得通过吞掉全部 element-ref 错误来隐藏真正的错误引用。

验收探针：`empty_tab_bar_mounts_without_error`、`closing_last_tab_commits_empty_state`。

### U2 / P1：List 为所有行展开整个选区，500 行仅选 20 行就渲染失败

位置：[list.rhai:298–306](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/registry/components/list.rhai#L298)。关联：[#119](https://github.com/eddix/gpui-rhai/issues/119)。

`render_List` 在虚拟化之前，为每一行调用 `next_selection(mode, selected, item.key)`，把“点击该行后的完整选区”放进该行数据。因此数据规模从行数加选区大小，变成两者相乘；不可见行也承担同样的数组构造和复制。

独立原生探针结果：

| 输入 | 结果 |
|---|---|
| 500 行、multiple、0 行选中 | 通过 |
| 同样 500 行、20 行选中 | mount 失败：`Size of array/BLOB too large` |
| 同样 500 行、500 行选中 | 同样失败 |

这是 Rhai 1.26 对嵌套值递归累计数组大小的限制，已核对锁定源码 `eval/data_check.rs` / `packages/array_basic.rs`。失败时 operations 分别为 156088、116492，都低于一百万默认执行预算；提高 operation limit 不会修复它。调用方输入的两个数组本身远小于各自的 10000 项 schema 上限。

建议：保持虚拟集合输入的体积与源行数据大致线性；每行只携带 key、selected 等必要状态，点击时计算下一次选区，或只在实际实现的行上构造所需载荷。成员判断也宜复用一个索引。可以顺带检查 Array-backed Table 的相似预计算，但本轮没有把它列为已独立复现的新缺陷。

验收探针：`list_500_rows_with_20_selected_still_renders`、`list_500_selected_rows_do_not_expand_into_oversized_arrays`；正对照 `list_500_unselected_rows_control`。修复后再补从少选到多选的事件路径，不能仅验证初始无选区。

### U3 / P2：TabBar 顺序变化但选择值不变时，不会重新显示当前标签

位置：[tab_bar.rhai:366](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/registry/components/tab_bar.rhai#L366)。

reveal 的依赖只有 `[props.value, cursor]`。调用方响应重排、恢复文档顺序或在当前标签前插入项目时，当前 key 可以完全不变，但对应的几何位置已经变化。effect 不会重启。

独立探针使用 12 个标签、360px 宽的栏：初始 `t1` 可见；仅反转 tabs，保持选中 `t1`，其范围变为 `x=1299, width=139`，仍留在 `[0, 360]` 视口之外。现有重排测试只记录 reorder 回调，未把新的 tabs 顺序写回，因而没有覆盖这个闭环。

建议：把顺序变化纳入 reveal 的有效条件，并验证真实受控重排后的布局；同时保留用户主动滚动浏览其它标签的能力，避免每次 scroll/frame 都强制跳回当前项。

验收探针：`reorder_keeps_selected_tab_in_view`。

### U4 / P2：新定义生成器把 Array / Map 返回类型破坏成 `Dynamic>`

位置：[script_docs/mod.rs:343–350](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/crates/gpui-rhai/src/script_docs/mod.rs#L343)。

`type_name` 解开 `Result<...>` 后对完整泛型类型使用 `rsplit("::")`，取到了内层 `rhai::types::dynamic::Dynamic>` 的尾段。`Vec<Dynamic>` 的特判无法匹配实际 metadata 中的全限定名，Map 也存在同样问题。

实际 `RuntimeEngine::definition_source()` 包含：

```text
fn date_info(date: String) -> Dynamic>;
fn date_month_grid(date: String, first_weekday: String) -> Dynamic>;
fn actions(this: UiContext) -> Dynamic>;
```

共有 8 个签名出现这个错误，完整列表见 [malformed-api-signatures.json](evidence/malformed-api-signatures.json)。影响 `gpui-rhai metadata` 输出和技能内的 script API reference；这已经是语法破损的定义，不只是文档措辞。现有测试证明“每个签名都有文字说明”，却不验证返回类型正确。

建议：识别容器外层类型及泛型结构，或复用可靠的 Rhai 类型映射；对引擎真实 metadata 的 Array、Map、fallible return 建立语义断言，并验证生成的 `.d.rhai` 可以被目标语言服务器消费。修正生成器后统一重生成 reference、两个 skill 分发副本和 bundle。

验收探针：`generated_definitions_use_script_collection_types`。

### U5 / P2：skill 的四个“完整可运行” recipes 实际都无法原样通过 check

位置：[skills.rs:201–213](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/crates/gpui-rhai-cli/src/skills.rs#L201)。

`example_view` 只提取 Rust 示例的 `const MAIN`，没有执行示例宿主之后的 `.replace(...)`。因此 skill 复制的是用于截图/场景生成的模板，不是示例实际运行的脚本。

用 CLI 同一套 Project API 创建干净项目、安装所需 modules 和 skills，再从发布 bundle 原样取每段 Rhai 写入 `ui/main.rhai`，结果如下：

| 片段 | `Project::check()` |
|---|---|
| SKILL.md 的计数 Button | 通过，无 warning，首帧验证通过（正对照） |
| Settings panel | locale 默认值 `__VISUAL_LOCALE__` 不属于允许值 |
| Form with validation | `__VISUAL_ACCEPTED__` 导致解析失败 |
| Data view with a table | `__VISUAL_SELECTED__` 导致解析失败 |
| Dashboard | init 设置的 `__VISUAL_LOCALE__` 未加载 |

建议：共享示例真正使用的源码构造逻辑，给 recipe 选择明确、可运行的参数，并带上必要的初始化前提；不要让 agent 自行猜测占位符类型和值。测试应直接消费生成后、安装后的 recipes，而不只是检查它们是否与同一生成器一致。还应覆盖 Form 提交/确认/Toast 等不在首帧执行的分支。

验收探针：`installed_skill_recipes_pass_check_as_written`。四个 recipe 都需要修，不只有含裸变量的两段。

## 二、26 个 open issue 的处置

完整原始描述和最新讨论快照见 [issue-evidence.json](evidence/issue-evidence.json)，最终清单见 [final-open-issues.json](evidence/final-open-issues.json)。PR 的 `Fixes` 当前只列出前 25 项，不包含 #131。

表中的“可关闭”指所报问题的已验证范围，不表示所有平台的人机验证已经完成。

| Issue | 本轮结论 | 核查依据 |
|---|---|---|
| #83 自定义 resize grip | 可随 PR 关闭 | control_visuals；历史遮挡/裁剪探针通过。上一轮仅作建议的 OS 光标观察不冒充已实测 |
| #89 Table context request | 可随 PR 关闭 | table_context；Array / NativeCollection 路径及键盘锚点 |
| #91 调用来源 | 可随 PR 关闭 | invocation_origin、window_command_origin；task completion 后排队 action 的历史探针通过 |
| #95 等宽字阶/列字体/span | 按 0.2 已沟通的 token-base/Host 字体模型关闭 | code_typography；Linux span 字族 fallback 仍是声明过的验证限制 |
| #109 overlay 实例身份 | 可随 PR 关闭 | overlay_identity、shared_layout_scope；重复实例及嵌套父级 |
| #110 虚拟行实例路径冲突 | 可随 PR 关闭 | virtual_item_paths；相邻 retained / 新实现行 |
| #112 error_boundary 内审计 | 可随 PR 关闭 | audit_regressions；边界内容和已实现虚拟行 |
| #113 sticky 分组头越界 | 可随 PR 关闭 | table_sticky_header；裁剪及列头命中 |
| #114 composition audit 误报 | 可随 PR 关闭 | audit_regressions；本轮也检查 SplitPane 和 absolute frame 的新增豁免 |
| #115 Toolbar 剩余宽度 | 可随 PR 关闭 | region_toolbar；fill 槽、宽窄布局、DataView 透传 |
| #116 快捷键图例 | 可随 PR 关闭 | shortcut_legend；Button / Command / 已格式化文字 |
| #117 迁移说明遗漏 | 可随 PR 关闭 | release 0.2.0 与 embedding 覆盖 app_owns_titlebar_drag、Variable、主题分层、SplitPane wrapper、Region scroll；Region 历史裁剪探针通过 |
| #118 三条设计建议 | 可随 PR 关闭 | 分组标签字阶、panel_focus_frame、composition 文档的 AppShell 范围 |
| #119 List 和 DataView bleed | **尚不能完整验收** | API、常规交互及 bleed 已实现；U2 的 500 行多选场景失败 |
| #120 CommandDialog | 可随 PR 关闭 | command_dialog_options；隐藏标题、Escape hook、width、内层 parts |
| #121 Region scrollbar | 可随 PR 关闭 | region_scroll_scrollbar；原生拖动和 ScrollArea 对照 |
| #122 stale / refreshing detail | 可随 PR 关闭 | inline_state_stale_detail |
| #123 窄容器文字换行 | 可随 PR 关闭 | long_text_wrap，含 stale / refreshing |
| #124 Toolbar 内 Tabs 溢出 | 可随 PR 关闭 | toolbar_tabs_overflow；收缩、滚动及空 end 组 |
| #125 Section 标题被压窄 | 可随 PR 关闭 | section_actions_squeeze；窄宽场景 |
| #126 FormLayout 内部误报 | 可随 PR 关闭 | form_layout_audit；横向 submit 与纵向 description |
| #127 disclosure 动态高度 | 可随 PR 关闭 | collapsible_content_height_density；整数对照、Length、内容测量、关闭后展开 |
| #128 TabBar menu 触发器高度 | 原问题可关闭 | tab_bar_overflow_trigger、embedded/control tests；不掩盖 U1/U3 的独立缺陷 |
| #129 Table header 图层底色 | 可随 PR 关闭 | table_header_on_raised_surface；表头位于独立滚动区外，移除填色合理 |
| #130 首个 TabBar 双边线 | 原问题可关闭 | tab_bar_component 的首项边线及 corners 检查；使用逻辑 border_start |
| #131 Chart 图层底色 | **未修复，建议纳入本 PR** | issue 原探针在当前 head 仍失败，见下文 |

### #131 的独立复现

[Chart wrapper](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/registry/charts/chart.rhai#L211) 和 [native chart theme](https://github.com/eddix/gpui-rhai/blob/95253c65d5f97a1e64ca51567255e6337ae02c96/crates/gpui-rhai/src/chart/primitive.rs#L2790) 仍分别填充 `surface`。

原 issue 探针得到同样两个色块：`[(600, 320), (596, 316)]`（2x 设备像素）。因此只改 wrapper 的 part style 不能解决内部 plot 的底色。

建议明确交互图表的底色继承/显式覆盖契约，同时保持 PNG 导出的背景策略独立；检查 tooltip、Geo/Cartesian/pie 以及 raised layer。完成后补上 `Fixes #131` 和相应回归。复现文件：[issue131.rs](issue131.rs)。本项是已存在的 issue，不计入上面五个新发现。

## 三、两个 skills 的专项结论

**结构、分发、目录覆盖通过；内容和可执行性尚未通过。**

已核实：

- 两个 `SKILL.md` 的 frontmatter、命名与格式校验通过。
- 105 个 Markdown 文件出现在 registry 的实际 package 文件列表中；两处分发副本的同步测试通过。
- 167 个本地文件引用和 5 个本地 anchor 检查通过。
- 91 个官方 module 的 schema/reference 覆盖测试，以及原生函数/primitive 的说明覆盖测试通过。
- Rhai named callback、curry、模块约束、生命周期与受控值的核心指导与实现总体一致；入口按 API 与设计分成两个 skill 的方向合理。
- 真正安装后的入口示例通过；四个 recipes 全失败（U5）；生成 API 类型存在 8 处错误（U4）。因此不能用“生成文件最新”和“doc 字段齐全”作为 skill 正确性的充分证据。

另外建议在本轮文档收尾处理三个问题：

1. **限定规则的适用层。** 通用 `gpui-rhai/SKILL.md` 要求任何 visible change 都先遵循设计 skill，并把 `metrics.* / space.*` 写成绝对规则；但 design principles 明确允许 BYOD、任意 token vocabulary 和无 profile 的中立 runtime。先识别项目所选层与 profile；官方设计规则用于选择该体系的项目，不应把 BYOD 强制迁入 productivity。
2. **允许核验宿主扩展。** “API 只有在这份 reference 中出现才存在”应限定为该版本内置/官方 API。实际 Host 扩展和应用自定义组件应以已加载的注册实现/schema 为依据；还应先比对项目锁定版本与 skill 版本。这比同时禁止幻觉和合法扩展更准确。
3. **修正文档中的诊断与预算表述。** `user-guide.md` 仍称 `last_diagnostic` 会在成功时清除，而当前 `last_failure` 持续到成功 reload / `clear_error`，embedding 和迁移文档已是新说法。Rhai limits 表还应写明默认一百万 operations、宿主配置入口，以及嵌套容器大小递归累计的规则；U2 表明这会直接影响使用决策。修维护文档后再生成 skills，避免直接改副本。

## 四、验证与发布状态

| 检查 | 本轮实际结果 |
|---|---|
| Workspace all targets / all features / locked / offline | **695 通过** |
| 默认原生 GPUI 测试，单线程 | **331 通过** |
| 原有两轮独立探针，源文件 hash 不变 | **10/10 通过** |
| 本轮独立探针（含 #131） | **2 个正对照通过、8 个断言失败**；对应上面 5 个新缺陷和 #131 |
| 严格 Clippy、fmt、diff whitespace | 通过 |
| skill frontmatter / 本地引用 / package 清单 | 通过，细目见前文 |
| 已提交的 57 对视觉基线 RGB 比较 | 43 对有差异，14 对 Dashboard 不变；正负对照正常。这是存量截图比较，**不是新截图验收** |
| 远端同一 head CI | **失败**：Weston 在 5 秒等待内尚未创建 Wayland socket；X11 smoke 已通过，Wayland 应用尚未启动 |

细节和命令见 [reproduce.md](reproduce.md)、[verification.json](evidence/verification.json)、[probes.log](evidence/probes.log)、[historical-probes.log](evidence/historical-probes.log)。上述计数分别对应不同套件，不合并声称为互不重复的用例总数。

[CI 日志](https://github.com/eddix/gpui-rhai/actions/runs/37952936385/job/113896232526) 显示 Weston 的 EGL/llvmpipe 初始化接近等待截止时间；更像启动等待余量不足，目前没有证据把它归为 gpui-rhai 产品故障。应给出有界的合理启动等待、保留进程存活与错误诊断，然后在最终 head 重新验证，不能跳过 Wayland 或把本次状态写成 CI 全绿。

本轮没有重新进行真实窗口截图、IME preedit、VoiceOver 和 release `gallery_profile`。最终修复 head 仍需执行既定发布清单；物理 120Hz 检查已被项目替换为 `Window::draw` p95 ≤ 8.3ms，这里不恢复旧门槛。三个 crate 的版本与生成依赖尚沿用 0.1.8，是已知的发包前版本提升步骤，不作为本报告缺陷。

## 五、给实施 agent 的收敛建议

不需要为这几个问题扩大核心 runtime 重构。建议按三个边界补齐保障：

- **组件状态变化**：在回调中真实更新 controlled props，验证 1→0、0→1、重排、删除/禁用当前项；不止断言事件发出。
- **数据规模**：虚拟化之前的数据展开也要有复杂度和嵌套值大小测试，覆盖多选；绘制行数少不代表脚本预处理成本低。
- **生成物消费**：直接运行安装后的 recipes，检查真实 metadata 的类型并消费 `.d.rhai`。生成器与输出互相相等，只能防漂移，不能证明两者都正确。

先修 U1–U5 与 #131，再刷新 skills/reference，运行这些独立探针及受影响回归；最后在同一最终 head 完成 CI、视觉、性能和发布清单。#119 的验收与 U2 绑定，不要先用现有三行 fixture 宣称它已经完整关闭。
