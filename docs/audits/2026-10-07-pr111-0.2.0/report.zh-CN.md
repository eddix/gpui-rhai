# PR #111：0.2.0 发布标准审查

日期：2026-10-07。审查对象：[PR #111](https://github.com/eddix/gpui-rhai/pull/111)，精确 HEAD **`c28849a139e34a2ed25fc8e27b63633383dcd48d`**，基线为已发布 0.1.8 的 `d87ebb9f781ecaaa53e265d39b9cacc7ded07f25`。变更共 355 个文件，+20,052 / −7,127 行。主工作区未切分支，审查在隔离检出中进行。

## 发布结论

**当前不符合 0.2.0 发布标准，建议 Request changes。确认 3 个 P1、2 个 P2。** 这不是因版本号尚未修改，也不是把有意 breaking change 当作缺陷；五个问题均有实际行为反例。

设计目标和完成的主体内容足以构成 0.2.0：中立 runtime 与设计 token 分层、组件与组合规范、独立尺寸/密度/圆角轴、Rhai 编写的验收 Gallery、可选择的 composition audit，方向正确。当前差距是新机制尚未在所有支持入口和数据源上遵守同一契约。应保留这条路线，定向补齐，不需要再推翻设计系统。

本轮亲自执行完整 workspace **677/677**、native **235/235**，均通过；精确 HEAD 的远端 CI 已成功。额外五项独立回归 **0/5，通过正对照后分别触发预期失败**。因此“已有测试全绿”和“当前存在发布缺陷”同时成立。

## 发现与修复要求

### R1 / P1：stretch 性能优化改变带外边距节点的布局语义

位置：[renderer.rs:3931](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/crates/gpui-rhai/src/renderer.rs#L3931)，调用入口约 2196 行、父级 stretch 提示约 2555 行。

`definite_stretch` 将纵向 stretch 容器中无显式宽度的子节点统一设成 `width: 100%`，没有扣除横向 margin。伸展后的 border box 与容器等宽并不是 stretch 在这种条件下的语义。

独立原生几何对照：父级宽 300px；子级左右各 20px margin，高 30px，无 width。两种写法表达相同的伸展意图：

| 子节点写法 | 实际 x | 实际 width | 预期 width |
| --- | ---: | ---: | ---: |
| 显式 `.self_stretch()` 正对照 | 20 | 260 | 260 |
| 默认继承 stretch | 20 | **300** | 260 |

后者右边到达 320px，已经越出父级，且无法保留右侧 margin。它影响通用 runtime 和自带设计的应用，不局限于官方组件或 Gallery。57 张现有布局的像素一致不能证明这个全局优化保持所有布局语义。

**建议：**仅在能证明等价时采用 definite-width 快速路径；带 margin、auto margin 等不能保证等价的布局保留正常 stretch。父布局判断应来自实际生效的样式，避免优化依据与呈现样式分离。不要要求使用者改掉有效 margin 或到处加显式宽度来补偿运行时错误。

**验收：**本反例、LTR/RTL、带 padding/border、min/max、auto margin、responsive 方向/对齐代表组合；随后重测 Gallery 帧耗时，确认正确性修复没有无意撤销可保留的优化。

### R2 / P1：token 热更新绕过组件要求校验，破坏 last-good

位置：[app.rs:6232](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/crates/gpui-rhai/src/app.rs#L6232)；prepare 使用的 `validate_component_tokens` 位于约 2715 行。

正式组件声明依赖 `metrics.unit`，用该 token 控制高度。通过 FileScriptView 的真实 PollWatcher 与 GPUI 调度执行：

1. 初始 token 为 40px，节点高度 40px。
2. 合法修改到 50px，节点变成 50px，证明 watcher、时钟与重绘驱动有效。
3. 从 `tokens.rhai` 删除该必需 token，热更新后的节点变成 **26px**，`last_error = None`。
4. 对完全相同的磁盘文件重新 `prepare()`，明确拒绝：`lacks tokens required by components/probe: ["metrics.unit"]`。

`reload_theme` 重建主题后直接发布到 runtime，没有执行 prepare 的组件要求校验；还在全部候选处理成功前写入 factory 的 base。于是运行中的主题可以进入冷启动会拒绝的状态，界面静默变形，而新的错误持久化机制也无错误可展示。

**建议：**token base、palette/variants 和有效 component requirements 组成一个候选；复用准备阶段的校验后再一起提交 runtime 与 factory。失败保留上次主题及 layers，并报告错误。同步检查脚本热更新新增 token requirements 的入口，不能只在首次 prepare 检查。

**验收：**合法 token 更新；删除 required token；坏 palette/表达式；失败后纠正；新组件要求与新 token 同批修改。至少增加一项启用 `gpui-rhai/dev-reload` 的 mounted-view 测试。当前默认 native suite 没有启用该 feature，235 项通过不能证明这条路径。

### R3 / P1：Table 键盘契约只在 Array 数据源实现

位置：[table.rhai:698](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/registry/components/table.rhai#L698)，以及 `with_row_keys`。

[设计规范 Table 段](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/docs/design/atoms.md#table)承诺：启用 selection mode 后有键盘入口，Up/Down/Home/End 移动选择并 reveal，Enter 激活。实现却只在 `!native_data` 时构造导航键与 reveal target；NativeCollection 得到空的 `row_keys`，因此没有安装相同的按键行为。

本轮给 Array 和 NativeCollection 相同的三行数据、相同选择和 callback，实际聚焦后发送 Down：

| 数据源 | 初始选择 | 按 Down 后 |
| --- | --- | --- |
| Array 正对照 | r0 | r1 |
| NativeCollection | r0 | **r0** |

大数据正是 NativeCollection 的适用场景。不能用 Array Gallery 场景和 Array tab-stop census 代替生产数据源的键盘验收。

**建议：**导航使用两种数据源的同一逻辑投影，Native 路径从原生投影获得相邻/首末 key；不要为了键盘每次在 Rhai 枚举或复制整份大集合。

**验收：**Array/Native 的 Down/Up/Home/End/Enter、选择与 reveal；筛选、分页、排序、折叠组、源替换的代表组合；大集合下保持有界 Rhai 工作量。修复承诺的同一能力即可，不要求加入新的多选快捷键功能。

### R4 / P2：带修饰键的 Capture handler 永远匹配不到

位置：[renderer.rs:2452](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/crates/gpui-rhai/src/renderer.rs#L2452)。

在同一可聚焦节点注册：

```rhai
.on_capture("key:shift+f6", Fn("cap"))
.on("key:shift+f6", Fn("target"))
```

两者都返回继续传播。实际 Shift+F6 的 trace 为 **T**，期望 **CT**。Target 能收到证明按键、修饰键和焦点驱动有效。

原因很直接：Capture 用无修饰的 `logical_keyboard_key` 查表；Target/Bubble 才组装 `canonical_key_name`，先匹配 qualified 名，再回退到普通 key。新增语法被统一接受，分发逻辑却只有一半升级。

**建议：**三个 phase 共用同一个事件键选择函数，保持“qualified 优先、普通 key 回退”的现有约定；不改变已被 Host action 消费的 chord 不进入 raw key 阶段这一 GPUI 行为。

**验收：**Capture/Target/Bubble × qualified/plain fallback，补 Stop/StopImmediate 与 disabled 控制；不以 Automation-only 调用代替真实 native key dispatch。

### R5 / P2：不完整的 by_env 表被判为合法且满足 required token

位置：[theme.rs:512](https://github.com/eddix/gpui-rhai/blob/c28849a139e34a2ed25fc8e27b63633383dcd48d/crates/gpui-rhai/src/theme.rs#L512)，`ThemeTokens::provides` 及 `EnvTable::resolve`。

声明 density 有 comfortable/compact 两个值，默认 compact；`metrics.row` 却只定义 comfortable 分支。当前加载、validate 及 `require("metrics.row")` 全部成功，但默认环境下解析该 token 返回 **None**。完整表的正对照正确返回 28px。

`validate_variable` 只检查已存在的条目是否合法，不检查被允许的环境取值是否都有解析结果；`provides` 又仅以 token 名存在判断要求已满足。因此消费者会在默认环境或切换密度/圆角后丢失样式，无法依赖准备阶段的保证。

**建议：**明确环境表的完整性策略。推荐拒绝缺失可达分支的表并给出 token 与缺失组合；若选择稀疏表，则必须有明确、经过校验的 fallback，不能在已通过 required-token 校验后静默返回 None。多轴检查应有界，不需要运行时逐帧枚举组合。

**验收：**缺默认分支、缺非默认分支、空表、多轴缺组合、完整表正对照；在 component preparation 和 theme reload 上得到一致结果。

## 为什么全绿还存在这些问题

这些不是需要再加几百张同类截图的问题，而是覆盖方向不够：

| 新变化 | 当前绿色证据擅长证明 | 本轮补出的空缺 |
| --- | --- | --- |
| stretch 优化 | Gallery 既有布局像素不变、帧耗时下降 | 通用布局的等价条件，尤其 margin |
| token base / declarations | 首次准备与合法官方主题 | reload 与 prepare 一致；可达环境值全部可解析 |
| Table 键盘 | Array 场景与单一 tab-stop census | NativeCollection 后端一致性 |
| 修饰键语法 | AppShell 的 F6/Shift+F6 Target 行为 | Capture/Target/Bubble 使用同一匹配规则 |

建议把这四行维护成小的契约矩阵。不要把“Gallery 是验收应用”理解成“Gallery 覆盖了底层 runtime 的所有合法使用方式”。BYOD 的中立运行时也必须保持通用布局语义。

## 功能、API、架构与 UI 判断

- **分层合理。** token base 与 palette 分离、环境值在原生呈现阶段解析、L2 模式复用 L1 组件，都能支撑持续迭代。ADR 0023 明确暂不支持逻辑级环境，避免给 eagerly constructed slots 一个不可靠的 `ctx.env()`；这是合理取舍，不列缺陷。
- **Gallery 进步实质明显。** 它现在使用产品自己的 Rhai AppShell/L2，覆盖组件规格、组合和四个工作场景，能暴露原 Rust 外壳会遮住的问题。这比“示例数量多”更有价值。
- **设计语言已成体系。** 本轮查看的 Button compact/light、Operations RTL/dark、Form compact/dark 三张现有基线，标记形态、标签字阶、输入井、内容边缘与层级有统一表达，没有发现需要推翻视觉方向的问题。未逐项人工操作全部 83 页，也未重新生成全部 GPU 图像。
- **API 仍需跨入口一致。** 本轮主要问题是新能力在旧机制各入口接入不完整，而非缺少更多抽象。修复应复用现有 token validator、事务提交、事件路由和数据投影；不应通过并行实现、兼容别名或 Gallery 专用修补绕过。
- **性能优化应附适用条件。** UiNode COW 的方向合理，完整既有测试本轮通过；stretch 的 R1 则说明“所有现有截图相同”不足以证明通用语义等价。Rhai 渲染、CPU draw 耗时与物理帧率需继续分别表述。

## 发布门槛核对

| 项目 | 状态 / 判断 |
| --- | --- |
| 精确 HEAD workspace | **本轮独立 677/677 PASS**；all-targets/all-features、locked/offline |
| 精确 HEAD native | **本轮独立 235/235 PASS**；默认线程栈、单测试线程 |
| 本轮新增契约探针 | **五项失败，均已确认**；见 R1–R5，不能被上两行豁免 |
| 精确 HEAD CI | **SUCCESS**，含 MSRV/stable、Clippy、默认 feature、性能结构、native、rustdoc、package、release build、Linux X11/Wayland smoke 和 artifact audit。[run](https://github.com/eddix/gpui-rhai/actions/runs/37571320156) |
| 视觉基线文件 | **本轮检查 57 张文件 PASS**；目标清单 21 examples PASS；这不是本轮重新 GPU render/逐像素比对的结果 |
| macOS 完整 release smoke | PR 明确本机未执行；Linux CI smoke 不能代替 macOS Metal/真实窗口启动。修复冻结后补最终候选结果 |
| OS 输入、辅助功能、窗口行为 | 仍需该版本的真实验收证据：IME preedit/clipboard/focus/VoiceOver、custom title bar 拖动与 double-click 等适用项。235 项 TestAppContext 不能替代 |
| 120Hz | 不从 CPU 帧耗时或 60Hz/offscreen 结果推断通过；按现有发布政策补证或由维护者对本版另作明确决策 |
| 版本与发行准备 | 三 crate 仍 0.1.8，是 PR 已声明的后续步骤；发布前须一致升级到 0.2.0、更新依赖/生成项目并重做 package/干净消费者验证 |
| Windows 拖窗 | 按声明支持范围处理。若 0.2.0 不承诺 Windows，不应把该已披露限制擅自扩大成这一轮移植任务；若承诺则必须补实现和测试 |

特别注意：[0.1.8 release notes](https://github.com/eddix/gpui-rhai/blob/d87ebb9f781ecaaa53e265d39b9cacc7ded07f25/docs/releases/0.1.8.md)明确写明 OS manual/physical120Hz 豁免 **仅限该次发布**，且不放宽未来政策。因此既不能将它们写成 0.1.8 已通过，也不能自动继承为 0.2.0 已获豁免。这是政策与证据缺口，与五个代码 finding 分开记录。

轻微的 compact 标题栏按钮约 1pt 偏移，可作为 UI 打磨项；单独不构成本报告的发布否决理由。预期的 Runtime API 2→3、组件 props/defaults 调整和删除旧 Rust Gallery 外壳也不计兼容性 bug。

## 下一轮实施与验收顺序

1. **先修 R1–R3，再补 R4/R5。** 每项把本轮反例变为正式断言回归；保留正对照，不能删 margin、改成 Array-only 场景或绕过 reload 来获得绿色结果。
2. **补小矩阵。** layout 等价、token 完整性/事务、两种 Table 数据源、各按键 phase；corner styles 取 square/subtle/round 与 RTL joined-control 的代表组合即可，不要求无差别穷举 83×15 个页面/主题。
3. **冻结产品后统一验证。** workspace/native、`dev-reload` mounted 测试、默认 feature、Clippy、MSRV/stable、release smoke、artifact/package；按修改影响更新 GPU 基线和有依据的性能对照。
4. **完成发行准备。** 保持本次有意 breaking change，无历史别名；同步三 crate/CLI 生成依赖版本，验证 API3 fresh init/embed 和旧项目 update 冲突路径。依赖实际发布后的 crates.io clean install 仍是后续发行阶段步骤，不能提前冒称通过。
5. **下一轮独立审查先验 R1–R5 与剩余门槛。** 没有新失败证据时，不再扩为另一轮设计系统重构。

## 证据、重现与边界

- [完整 workspace 输出](evidence/workspace.log)、[完整 native 输出](evidence/native.log)。
- [最终独立探针源码](probes.rs)、[最终确认日志](evidence/probes-confirmed.log)、[可重复运行器](run-probes.py)。
- [CI 原始 JSON](evidence/ci.json)、[PR 元数据](evidence/pr-111.json)、[审查基线与计数](evidence/baseline.json)。
- [视觉文件审计](evidence/visual-audit.log)、[目标清单检查](evidence/target-manifest.log)、[机器可读结论](verification.json)。

在包含目标提交/修复的检出上运行：

```sh
python3 /path/to/this/audit/run-probes.py --repo /path/to/gpui-rhai \
  --target-dir /path/to/existing/native-target
```

运行器只临时新增自己的 native test 文件并在结束时删除；产品源码不变。明确开启 `gpui-rhai/dev-reload`，原 HEAD 预期五项均失败。初期记录 `probes.log` / `probes-2.log` / `probes-3.log` 含尚未正确驱动几何、尚未启用 dev-reload 的夹具结果；`probes-dev-reload.log` 还含 dispose 后测试 Host unwrap 的清理错误。它们保留为探索记录，**不作为产品缺陷数量或最终失败原因**。最终日志中每一失败均落在相应产品行为断言，合法 reload、显式 stretch、完整 env 表、Target key 和 Array Table 正对照有效。

本轮只新增审计材料与临时探针，没有修改产品、正式测试、历史审计或外部 dogfooding 项目，没有提交 GitHub 评论或执行合入/发布/tag。审查判断覆盖上述范围，不以五个反例宣称已穷尽所有合法输入。
