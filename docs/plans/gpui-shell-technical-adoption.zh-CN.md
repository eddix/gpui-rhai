# GPUI Shell 技术机制吸收实施计划

日期：2026-10-17。面向负责实现的开发 agent。

目标是在现有 gpui-rhai 架构上减少重复执行、统一 API 契约，并提前发现脚本接入错误。优先交付性能观测、API 描述统一和宿主调用预检；原生子树缓存、结构模板化先做有边界的原型，达到正确性与收益门槛后再进入产品。

## 范围与源码基线

本计划只吸收技术机制，不安排 gpui-shell 产品验收、生态对比、语言迁移或引入其运行时。保留现有 Rhai、状态事务、组件依赖追踪、原生信号和生命周期模型。无需重做已存在的事务、代际隔离、异步归属和 suspend/resume。

| 对象 | 基线 |
|---|---|
| 实现参考 | gpui-rhai `9a1b7797a2385170649f51d3858867ad2e22e391`，当前 workspace 版本 0.2.2 |
| 锁定依赖 | gpui-pre/core/platform 0.3.7，Rhai 1.26.0；生产执行后端仍是 AST interpreter |
| 借鉴来源 | gpui-shell `5b5566e9acf44208b67339f79cb4902e692be9de`，本计划引用固定提交中的实现 |
| 执行方式 | 在实际实施分支核对差异；已有等价能力直接复用，不因本计划较早而重复建设 |

当前代码比最初讨论已有进展：

- `ExecutionTiming`、`ComponentReuse`、`ScriptViewPerformanceSnapshot`、虚拟集合指标和 reconcile 结果已经存在。
- `HostFunction`、`[host]` 声明和 `lint_known_calls_with_host` 已支持宿主函数名称/参数数量检查。本计划补充的是 capability 方法语义及同源契约，不是再做一套 known-call lint。
- 原生滚轮区域已支持缓存帧后的事件登记重放；`cached_native_body_replays_current_event_regions_after_unrelated_sibling_updates` 是现成回归。它不代表完整 ScriptView 缓存已经实现。
- `type_name` 已修复泛型返回类型被截成 `Dynamic>` 的问题。API 描述统一解决的是长期维护问题，不应把这个旧缺陷重新列为未修。
- `UiNode` 已采用写时复制，普通 clone 是引用计数操作。模板收益不能建立在“当前每次传节点都深拷贝整棵树”的旧假设上。

实现入口：[性能文档](../performance.md)、[架构](../architecture.md)、[组件增量 ADR](../adr/0013-retained-tree-and-incremental-components.md)、[能力调用](../capabilities.md)、[运行时契约测试](../runtime-contract-tests.md)。

## 实施顺序与交付边界

| 顺序 | 工作项 | 交付要求 |
|---|---|---|
| 1 | T1 统一性能观测 | 直接产品化，提供非破坏式读取、明确计时口径和确定性测试 |
| 2 | T2 统一原生 API 描述 | 先选代表性 API 迁移验证，再分批覆盖内置 API，消除平行维护 |
| 3 | T3 宿主 capability 调用预检 | 复用现有 lint 和 capability schema，支持明确的检查覆盖状态 |
| 4 | T4 原生子树缓存 | 受控原型；有稳定收益且交互契约不回退，才扩展适用范围 |
| 5 | T5 固定结构模板 | 根据 T1 的结构重复数据选热点；没有必要收益时保留普通路径 |

T1 与 T2 可独立提交。T3 复用 T2 的契约导出表达，但不应被“所有 API 全部迁完”阻塞。T4、T5 各自比较基线与候选，不能把两项同时打开后只报告一个总收益。

每个工作项交付独立、可审查的变更和相应维护文档。允许必要的破坏性 API 调整，但必须同步迁移仓库内调用、示例与生成物，并写升级说明；不为本轮重构新增永久兼容转接层。本计划不自动增加现有发布的阻断项，发布仍以所属版本的范围和 release checklist 为准。

## T1 统一性能观测

### 要解决的问题

现在可以观察 Rhai 执行、组件复用、虚拟行和最终帧耗时，但一次交互跨越这些环节时，难以统一归因。`ScriptViewPerformanceSnapshot` 的明细读取会排空记录，也不适合多个观察者独立取样。

借鉴 Shell 的[累计计数与快照差值](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/metrics.rs)，在现有诊断体系内增加统一观测；不要建立独立的第二套 profiler。

### 实现要求

1. 为挂载 View 提供轻量累计计数的只读快照。快照包含实例身份和观测 epoch；求差时识别重挂载或重置，不能把不同实例的计数相减。现有排空明细可按需要保留或迁移，多个读取者不得互相吃掉数据。
2. 区分根/普通组件执行、虚拟行执行、组件复用、宿主原生调用、reconcile、原生元素构建、缓存命中/未命中。次数、操作数和耗时分开表达。
3. 记录失效原因的有界分类：state/store、props、locale、geometry、原生数据源、热重载等。一次更新可以有多个原因，不应强行指定一个导致统计失真。默认只记类别和计数，不记录业务 payload。
4. 记录结构重复与结构改变。指纹只包含节点种类、拓扑、稳定身份及结构性操作，排除文本/颜色等已定义动态值。仅在候选树已经被遍历的阶段计算；不为统计增加每帧全树扫描。指纹用于分析，不是缓存有效性的证明。
5. 复用 Gallery/Inspector 展示取样差值。计数器读取不得触发 Rhai、刷新 View 或强制创建新帧。直方图/明细采用固定上限；默认不落盘，仅在明确请求导出时生成一个结果文件。

### 计时语义

- 保持现有 operations/round_operations 的含义和预算检查。嵌套调用的父子耗时不能简单相加；明确 inclusive/exclusive 或 span 父子关系。
- 失败的执行确实消耗了资源，应计入 attempts/failed；树和状态回滚不应把已发生的工作从指标里抹掉。
- 宿主函数耗时是脚本调用跨度的子集，拆出来归因，不再次加入总耗时。高频 Style/builder 调用不得默认逐次读时钟；细粒度计时应通过诊断开关或抽样开启，并测量自身开销。
- `Window::draw` 是完整 CPU 帧路径的测量边界，不等于 GPU 完成或显示器实际呈现时间。若增加交互延迟，明确测到的是“输入到包含相应 committed revision 的帧完成”。
- 不对累计最大值或累计 p95 做相减。区间分位数来自可求差的有界桶或明确的采样窗口，并说明近似精度。

### 验收

两个观察者独立读取并求差得到一致结果；连续读取不产生工作；失败执行可见；重挂载不能串计数。稳定场景重复绘制时脚本执行计数不增长。单组件状态变化只归因到实际受影响组件；虚拟行工作不会混入普通组件计数。统计本身的开关开销有对照记录。

入口：[engine.rs](../../crates/gpui-rhai/src/engine.rs)、[app.rs](../../crates/gpui-rhai/src/app.rs)、[retained.rs](../../crates/gpui-rhai/src/retained.rs)、[virtual_list.rs](../../crates/gpui-rhai/src/virtual_list.rs)、[gallery_profile.rs](../../tests/native-keyboard/src/bin/gallery_profile.rs)。

## T2 统一原生 API 描述

### 要解决的问题

组件 schema 与 PrimitiveDescriptor 已能提供结构化契约；普通原生函数的注册实现、`ScriptFnDoc` 和返回类型推导仍需连接多份信息。Shell 的[组件注册描述](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/component_registry.rs)值得借鉴：明确描述构造/方法/参数，在注册完成后形成经过校验的目录。

### 实现要求

1. 为内置原生函数设计最小统一声明：脚本名称与 namespace、receiver、重载参数、脚本返回类型、参数名、说明、feature 条件，以及注册实现。可用小范围宏或 builder，选择维护成本更低的一种，不建设通用代码生成框架。
2. 脚本类型使用结构化表示，覆盖 Array、Map、optional、custom type、dynamic、getter/setter 和操作符。fallible Rust 返回值暴露的是成功类型；类型语法最终由目标消费者格式决定。
3. 同一声明驱动注册与工具目录，目录供 known-call lint、`.d.rhai`、reference 和 skills 使用。注册后与引擎真实 metadata 交叉核对，防止声明与注册 closure 的类型/arity 脱节。
4. 先迁移一组 Style 方法、返回 Map/Array 的函数、一个重载、一个自定义类型和 feature 条件项。通过语义验证后分批覆盖内置 API，移除对应的平行手写表。试点之外的现有目录继续工作，不能出现两份同名声明各自权威。
5. 宿主自定义函数仍须可注册和发现。缺少丰富文档时允许可靠的 metadata 降级表达，不得因未列入官方目录就判 API 不存在，也不能猜测不存在的类型。
6. 复用现有 ComponentSchema、ValueSchema、PrimitiveDescriptor。目录冻结可用于一个准备完成的版本；热重载构建新候选目录后原子替换，不能把“冻结”解释成禁止合法热重载。

### 验收

从实际引擎注册结果验证名称、重载、receiver、参数数量与返回类型；至少覆盖嵌套泛型、optional、Map/Array、自定义类型及 feature 开关。生成的定义必须被目标定义语法的消费者/解析器接受，不能只检查字符串存在。

若没有可直接复用的解析器，明确选定一个有版本的消费验证工具，保存最小可重复检查；不要拿普通 Rhai 脚本 parser 错当 `.d.rhai` parser。生成 reference/skills 后，直接在干净 CLI 项目运行其示例，包含事件触发后的路径。同步测试继续保留，但“文件等于生成器输出”不是语义验收。

入口：[script_docs](../../crates/gpui-rhai/src/script_docs/mod.rs)、[script_lint.rs](../../crates/gpui-rhai/src/script_lint.rs)、[primitive.rs](../../crates/gpui-rhai/src/primitive.rs)、[reference.rs](../../crates/gpui-rhai-cli/src/reference.rs)、[skills.rs](../../crates/gpui-rhai-cli/src/skills.rs)。

## T3 宿主 capability 调用预检

### 要解决的问题

现有 `HostFunction` 能识别 `name/arity`、`module::name/arity` 与宿主 primitive。仍需进一步识别这样的调用内部的语义名称：

```rhai
ctx.call_capability("app.files", "raed", input)
```

外层方法和 arity 正确，不代表 capability/method 存在。借鉴 Shell 的[宿主模块提前解析与调用时转发](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/engine/quickjs/host_modules.rs)，把可证明的错误提前暴露，保留运行时授权边界。

### 实现要求

1. 从现有 CapabilityDescriptor / CapabilityMethod 及同步/异步/订阅注册类别导出无执行能力的契约：ID、版本、同步/task/subscription 入口及输入输出 schema。统一表达代码版本/Runtime API 与契约格式版本；指纹采用规范化的语义字段，不因说明文字调整就改变语义身份。
2. 导出只读取声明，不执行能力 handler、任务、订阅或业务服务初始化。不导出密钥、业务数据、绝对工作区路径或可调用句柄。
3. 将契约接入已有 known-call lint。对静态可见的字面量 ID/method 检查存在性、调用类别及与 app manifest 的对应关系；确定的未执行分支也应检查。只分析可证明的输入，不执行脚本做常量求值。
4. CLI 没有宿主契约时，明确标出哪些宿主调用未被验证；不要假装全通过，也不要把合法的未知 Host API 当成必然错误。动态名称同样保留运行时校验，报告覆盖范围。
5. 单独的契约文件仅是开发工具输入。真实宿主加载时对照实际注册目录/版本，发现过期契约时给出明确诊断；不能据文件内容授予能力。调用时仍校验授权、有效版本、生命周期及回调归属，不能保存绕过注册表的原始 handler。
6. 先增强现有 API 的检查能力，不为模仿 ES module 语法增加新语言层。默认保持静态分析为只读行为。

### 验收

覆盖拼错 method、未知 capability、错误入口种类、未执行分支、动态名称、缺少契约、过期契约和合法 Host 扩展。用调用即失败的测试 handler 证明检查过程没有执行它。准备后发生的撤销/卸载/代际变化仍在运行时生效，静态检查通过不构成授权。

入口：[capability.rs](../../crates/gpui-rhai/src/capability.rs)、[script_lint.rs](../../crates/gpui-rhai/src/script_lint.rs)、[AppManifest](../../crates/gpui-rhai/src/capability.rs#L647)、[CLI 检查流程](../../crates/gpui-rhai-cli/src/lib.rs)、[宿主扩展文档](../capabilities.md)。

## T4 原生子树缓存原型

### 机会与现有约束

Shell 对窗口内容使用 GPUI 的[缓存视图路径](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/root.rs#L848)。我们已缓存组件的 UiNode 结果，但渲染器仍从 retained 数据构建短生命周期的原生元素。缓存要减少的是这一段工作，不能仅以“Rhai operations 为零”证明有效。

锁定的 gpui-pre 0.3.7 已有 `Entity::cached` / `AnyView::cached`，内部复用还受 bounds、content mask、text style、dirty views 和 refreshing 等因素控制。使用其正式缓存路径，不跨帧保存、克隆或重复消费任意 `AnyElement`，不因此升级 GPUI。

### 实施步骤

1. 用 T1 选择元素构建成本明显、输入稳定的只读说明面板作为首个对象，提供可对照的缓存开关。先以单个 View/明确子树为边界，不默认开启整窗缓存。
2. 列出该边界的全部失效源：提交版本、theme/env、locale/RTL、geometry/bounds/clip、祖先文字样式、原生数据、信号/动效、焦点/选择、错误表面、热重载和生命周期。首版无法正确支持的情况走现有路径，不能忽略。
3. 分开稳定绘制数据与必须更新的交互/几何登记。复用已有 native wheel capture 重放，不建立另一套 wheel 所有权；对 pointer routes、presented、drop targets、geometry hitboxes、虚拟实现请求和辅助功能逐项说明缓存命中时的有效性来源。
4. 当前 `WindowInteractionCoordinator::begin_frame` 仍清理部分登记，`prepare_host_render` 维护几何与焦点。缓存方案必须解释这些步骤如何继续执行或安全复用，不能在 root 随手加 `.cached()` 后仅凭截图宣布成功。
5. 缓存归属使用当前 View、组件/节点身份和 generation/lifecycle，不得跨不同 Host 域或卸载后复用。释放应跟随现有 owner，不能增加无界全局缓存。

### 验收与退出条件

稳定内容经历无关兄弟更新时，脚本和原生元素构建计数都不应因该更新增长；相关输入改变时正确失效。布局、文字、主题与绘制结果等价。测试缓存命中后的滚轮、裁剪遮挡、焦点、卸载/重挂载，并验证已有缓存滚轮回归仍通过。

扩大到交互子树前，再覆盖拖放、pointer capture、overlay、虚拟行、signal、动效和辅助功能，不能用静态面板的验收替代。性能必须在我们的同机基线与候选上证明；收益不稳定或登记维护成本抵消收益时，关闭实验并保留观测结论，不将实验 API 作为正式承诺。

入口：[app.rs](../../crates/gpui-rhai/src/app.rs)、[renderer.rs](../../crates/gpui-rhai/src/renderer.rs)、[interaction.rs](../../crates/gpui-rhai/src/interaction.rs)、[geometry.rs](../../crates/gpui-rhai/src/geometry.rs)、[现有缓存滚轮测试](../../tests/native-keyboard/tests/native/collections/scrollbars.rs)。

## T5 固定结构与动态值槽位原型

### 机会与边界

Shell 的[内部 template](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/engine/quickjs/template.rs)将固定结构与参数槽位分开；测试通过 `globalThis.__template` 使用它。其[实例化](https://github.com/longbridge/gpui-kit/blob/5b5566e9acf44208b67339f79cb4902e692be9de/crates/shell/src/spec.rs)仍复制结构节点，因此主要减少脚本描述和语言边界调用，不代表消除分配、reconcile 或布局。

我们已有三类能力要先利用：输入不变时的组件复用、UiNode 写时复制、信号支持的原生热路径。模板只针对“组件确实需要执行、结构长期不变、描述成本显著”的剩余热点。没有这种热点就不必新增公共模板 API。

### 实施步骤

1. 根据 T1 选一个静态结构行或数据摘要块；使用同一内容的普通构建版本作为对照。排除已由原生信号有效解决的更新。
2. 设计显式、类型化槽位，首版只覆盖文本、颜色与确定类型的长度/样式值。保留符号 token 和 environment 的解析语义，不能把首次 theme/density 的结果固化。
3. 首版排除状态组件、effect、ref、signal、任意回调槽和可变子树。这些仍走现有普通节点组合。模板编译/记录发生一次，但每次填值必须归属于当前实例与代际，产生合法稳定 key，不复用另一实例的 NodeId。
4. 先验证全部参数，再把候选节点交给现有事务/reconcile。参数错误不能留下半棵模板、错误回调或悬挂资源；热重载、卸载使旧模板引用失效。
5. 不照搬依赖 JavaScript 哨兵对象的推断方式。不得宣称可自动把任意 Rhai 函数、条件分支、循环或闭包变成模板。可先在 Rust 侧实现结构/槽位原型，再决定是否需要脚本入口。
6. 不在每个数据项中预先展开整份共享选区或模型；模板不能重新引入 List 曾出现的嵌套数组膨胀。

### 验收与退出条件

同一结构的不同实例填值互不影响；参数数量/类型错误、token/env 切换、实例重排/删除、旧代际使用都有明确结果。普通路径与模板路径的树、样式、语义和身份一致。

分别测首次记录、稳定填值、混合模板/普通节点、内存与释放成本。固定结构热点的构建工作应明显减少，端到端收益超过相邻基线抖动，不能仅以 builder 调用减少判定成功。如果公共 API 的复杂度超过可重复收益，保留内部实验结论并结束此项，不交付长期双路径维护负担。

入口：[node.rs](../../crates/gpui-rhai/src/node.rs)、[style.rs](../../crates/gpui-rhai/src/style.rs)、[engine.rs](../../crates/gpui-rhai/src/engine.rs)、[retained.rs](../../crates/gpui-rhai/src/retained.rs)。

## 共同验证矩阵

| 场景 | 必须观察的行为 |
|---|---|
| 干净窗口与无关兄弟变化 | 不引入额外 Rhai 执行；缓存范围内不重复构建；指标读取无副作用 |
| 局部状态、props、store path 更新 | 只执行所需组件；结果与关闭优化时一致 |
| 数据只改值、重排、过滤、空集合 | 区分值与结构变化；身份稳定；不会扩大数据到平方规模 |
| 主题、density、RTL、窗口尺寸与裁剪 | 正确失效；继承值不固化；几何/命中不陈旧 |
| 滚轮、焦点、overlay、pointer capture | 原有所有权与传播契约保持，尤其缓存命中后 |
| 错误、失败 reload、成功 reload | 失败仍保留 last-good 状态；统计记实际失败工作；旧代际不能继续调用 |
| suspend、resume、dispose、同 key 重挂载 | 资源按原生命周期处理；缓存/模板不能跨实例泄漏 |
| Host 扩展与无宿主 CLI 检查 | 已知错误提前失败；未知范围明确报告；检查不执行 Host 服务 |
| 生成物与 skills | 两处分发副本同步；类型正确；安装后的示例可运行 |

测试加入现有产品测试入口，不复制整个历史审计目录。当前原生 workspace 使用 `autotests = false`，主入口为 `tests/native/main.rs`；新 native 用例应纳入相应模块，单独放一个 `.rs` 文件不会自动运行，也不应为每个探针增加新的大型 test binary。

性能实验采用同机 release、相同输入与 feature、相同观察范围，交替运行基线和候选，至少三组，每组建议 5 次 warmup、30 次有效样本。记录准确 SHA、工作区状态、工具链和硬件。比较分阶段计数、p50/p95/p99 与端到端耗时；每项明显回退都需解释，不能用平均收益掩盖尾延迟。共享 CI 守结构性断言，绝对时间沿用受控机器上的现有预算。

可复用的命令如下，受影响的定向用例先运行，再做完整验收：

```sh
cargo test --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all -- --check
cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native --locked -- --test-threads=1
cargo test --manifest-path tests/native-keyboard/Cargo.toml --features dev-reload --test token_reload --locked
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --tests --all-features --locked -- -D warnings
cargo run --release --manifest-path tests/native-keyboard/Cargo.toml --bin gallery_profile -- 20
bash scripts/benchmark.sh
```

最后两项是性能比较工具，不用来证明所有功能正确。`scripts/benchmark.sh` 使用 offline 模式，新机器先准备锁定依赖。不要修改 `RUST_MIN_STACK` 或放宽脚本预算来掩盖回归。

T2/T3 修改生成源后执行现有生成流程：

```sh
GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli
cargo test -p gpui-rhai-cli
```

第二次运行不带更新变量，确认生成内容已经稳定；再检查实际输出语义及干净项目消费，不止确认文件没有变化。

## 最终交付与收尾

每项报告实际完成的范围、契约测试、性能结果及保留限制。T4/T5 可以以“实验无稳定收益，不进入产品”完成，但必须留下可理解的测量结论；不能把未完成原型称为已优化。

长期规则分别并入 `docs/architecture.md`、相关 ADR、`docs/performance.md`、`docs/capabilities.md`、`docs/runtime-contract-tests.md` 与使用指南。API/reference/skills 由生成源更新。按 `docs/README.md` 的约定，实施完成后本阶段性计划留在交付分支或 git 历史，产品树只保留维护中的契约与必要的未完成事项。

工作区采用项目现有编译缓存；不新增重复仓库、巨型独立 target 或长期临时 worktree。可复现证据提交到约定的 GitHub 分支后，清理本轮临时脚本、日志和专用编译产物；不另建一份永久本地归档，也不删除其它任务的工作或共享依赖缓存。
