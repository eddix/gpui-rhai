# 当前开放 PR / issue 分流评审

日期：2026-10-01。范围：当前开放的 5 个 PR、6 个 issue。评审期间另一会话正在修改产品工作区，因此代码判断固定到下列 Git 提交；这些未提交修复不视作已经验收。

**建议：#97 小改、#96 拆分后的小改、#99 由主开发补齐主题闭环，纳入 0.1.8；#14 结束当前主线集成申请，保留实验。#94 可作为已验收的第三轮整改阶段性合入，但第四轮 P1 继续阻断 0.1.8 发布。** 三个新 dogfooding PR 当前 head 都不建议原样合入。

用户给出的分类包含两种维度：“谁来改/如何合”与“哪个版本合”。下表分别给结论，避免把“作者小改”和“纳入0.1.8”错误地当作互斥选项。本轮只评审，未合并、关闭、评论、审批 PR 或修改 issue 状态。

## PR 决策表

| PR / 固定 head | 处理方式 | 0.1.8 | 当前合入条件 |
| --- | --- | --- | --- |
| [#94 第三轮整改](https://github.com/eddix/gpui-rhai/pull/94) `f6e936a5` | **第1类：可阶段性接受**；由主开发继续第四轮整改 | 必须包含 | 第三轮反例复验已过、CI绿色；此批准只覆盖阶段性改进，不能视为发布验收。第四轮仍有3P1/5P2 |
| [#97 标点按键](https://github.com/eddix/gpui-rhai/pull/97) `31df8387` | **第2类：作者小改** | **建议纳入** | 修两个输入契约缺口、补实际分发测试、fmt和完整CI通过 |
| [#96 Host运行预算](https://github.com/eddix/gpui-rhai/pull/96) `befbe926` | **第2+5类：先拆分，再由作者小改** | **仅预算配置部分纳入** | 移出全局解析深度调整；补真正超1M及继承测试，明确配额语义；fmt和CI通过 |
| [#99 读取resolved theme](https://github.com/eddix/gpui-rhai/pull/99) `3209039e` | **第4类：主开发接管有限范围的主题闭环**，保留作者实现与贡献 | **建议纳入** | 修初次系统外观同步；纠正effect承诺；避免完整theme复制；补scope/切换测试及Clippy |
| [#14 Grain/JIT实验](https://github.com/eddix/gpui-rhai/pull/14) `6427de47` | **第5类：建议关闭本次集成PR，保留实验分支和证据** | **不纳入** | 将来由真实release瓶颈和整应用收益重新触发设计，不安排主开发现在接手重写 |

#94 的处理与前一轮“仍不建议发布”结论一致：项目已经存在合入主干但未创建 tag/发布 crates.io 的阶段，第三轮已证实有效的改进可以先进入这一阶段。第四轮整改需要新的明确验收，不能因为先合了 #94 就把后续问题降级或取消发布门槛。

## #97：方向正确，修输入契约即可

需求来自键盘优先的真实应用：`?` 帮助、`/` 过滤、`[`/`]` 步进应当能注册。锁定的 GPUI 0.3.7 支持常见标点；原生聚焦节点的 literal `?` 分发已验证。不需要为这个 PR 重写跨平台键盘层。

当前有两个可独立修正的问题：

1. 新规则接受 `on("key:Escape")`，但 generic on 不做小写归一化，实际 `escape` 查找命不中；`on_key_value("Escape",...)` 则会转小写。同一意图的两条 API 成功注册后行为不同。
2. 文档声明“named key 或单个标点”，helper 实际接受任意可打印ASCII串，包括 `cmd-s`、`shift-/`、`??`。renderer不会把它们解析为chord，注册成功却没有对应分发。

作者应统一一个 canonical key 校验入口：明确 named key 与单个标点的语法，generic 大小写归一或拒绝策略保持一致，并补负例。实际焦点输入测试覆盖 `? / [ ] - =`，保留 Escape、disabled、Enter/Space click fallback 的现有行为。

modifier 与 `key_char` 是已有边界：当前节点 matcher 按 `keystroke.key` 匹配，不比较修饰键，也不按 `key_char` 做文本匹配。因此 Ctrl+/ 也可能命中 key:/；“no modifiers”应说明不解析/不限定修饰键，而不是暗示带修饰键不会触发。复杂shortcut仍使用Host action/keybinding。不要在此PR顺手改掉整个键盘语义。

结论：**作者小改即可，独立合入0.1.8**。详见 [#97报告](pr-97/report.zh-CN.md)。

## #96：批准 Host 配额能力，拆出默认解析策略

可配置 operation budget 是合理的嵌入能力：默认1M不变，Host明确提额，继续复用累计 OperationTracker，脚本无法自行修改。源码已覆盖 File/Embedded prepare、secondary window factory、reload candidate 的传播；没有必要先建立庞大的执行策略框架。

需拆开的另一件事是 `set_max_expr_depths(64,32) → (64,128)`。它改变所有应用的默认解析策略，和特定Host给自己的脚本提额不同。PR举出的函数深度约22尚未超过32，不能据此证明需要默认四倍上限；Rhai自身function depth默认还区分debug16/release32，原注释不完整。runtime operation limit也不约束parser复杂度。

预算部分最小修订：

- 同一份真正超过默认1M的工作：默认拒绝、提高后成功且输出一致、再降额拒绝。PR现有60k循环实测仅180,009 ops，提高到50M再次通过不能覆盖真实提额需求。
- 针对性证明非默认额度在prepare、多窗口、reload candidate中生效；nested/sibling仍共享一个execution总额，retained/delayed callback不会继承父执行的旧消耗。
- 文档明确额度作用于该View的各执行回合，不是仅放行某次layout；操作数不保证一帧内完成，slow threshold只是诊断。
- 明确当前 `0 → 1` 的归一化语义；builder默认描述不要向公开u64方法描述内部None参数；说明可信extension与builder的优先级。
- 修fmt并执行完整检查。

**0.1.8纳入收窄后的预算部分。** 表达式深度另开最小复现和策略评审；如果只是特定Host需要，更适合Host配置而非无条件改变默认。不要增加脚本自行提额的 `ctx.with_budget`。

详见 [#96报告与固定基线探针](pr-96/review.zh-CN.md)。

## #99：新API暴露了尚未闭合的主题环境，需要主开发接管

需求有价值：跟随系统主题的Canvas/图表派生颜色，必须能知道该window/component实际解析到什么主题。读resolved variant比读用户选择的System偏好更准确。

但本轮对 **PR固定head的独立源码快照** 运行原生probe，证实不只是Clippy问题：

| 场景 | 实际结果 |
| --- | --- |
| 系统Light窗口首次挂载，root render直接显示 `ctx.theme_variant().mode` | Host有效theme为Light，脚本仍为dark；推进原生事件循环后仍不收敛 |
| effect body中读取theme，deps保持 `()`，随后选择Light | Host变Light，effect仍只启动一次，状态停留 `1:dark` |
| render读取theme并显式传入effect deps，同样选择Light | 正对照成功，`1:dark → 2:light` |

初帧问题沿用旧appearance解析链：prepare/render时window appearance尚未登记，resolver回退Dark；首次登记Light使用“之前存在且不同”的判断，没有使旧读者失效。这不是说PR新增代码制造了原有Host生命周期问题，而是新API的“resolved且随系统变化”承诺在真实挂载时不能成立。

建议主开发继续原PR或保留原贡献追加修复，控制范围为：

1. 统一首次挂载、System变化、Host/Rhai选择、window/subtree覆盖的有效主题解析与读者失效；不能只在getter硬换另一个默认值。
2. 文档改成“render中读取，派生值放入effect deps”。不为使错误文档成立而改成全局自动追踪effect body依赖。
3. 避免把现有 `motion_tokens()` 变重：PR先clone整个ThemeVariant再取motion，而旧实现只cloneThemeMotion。使用借用resolver/read closure或轻量metadata读取，共享解析逻辑即可，无需第二套主题系统。
4. 加固定选择/System首次Light与Dark/切换/window与subtree优先级/无theme的契约测试；明确Rust和Rhai返回的元数据与错误/None策略。
5. 删除测试中多余mut，严格Clippy及完整CI通过。

**建议进入0.1.8，但应由主开发把主题上下文的闭环补齐后合入。** 这项主要是框架整合责任，不要求重写所有theme实现。详见 [#99报告](pr-99/report.zh-CN.md)。

## #14：结束当前集成申请，保留研究成果

这个PR值得保留的是JIT可行性和ABI/副作用/代码生命周期研究；当前没有充分理由扩大产品支持面。

- 历史数据的numeric收益确实很大，但同次adaptive host-heavy样本相对生产AST **慢约28.4%**。自适应降级回Grain，并不等于保护当前AST基线；尚无当前release整应用收益。
- published package的fork隔离已修正，但根 `[patch.crates-io]` 仍使默认workspace/CLI解析到fork；执行feature关闭不等于依赖来源隔离。
- 旧adapter的observer只记绝对operations、progress回调返回None；现行主线已经使用累计预算和progress终止。直接移植会碰到明确的运行时契约冲突。
- 当前callback、incarnation、虚拟化、rollback和hot reload契约已演进；旧测试数量不能为新runtime API背书。这里无需支持旧API兼容。

建议关闭当前“集成到主线”的PR目标，保留分支与固定SHA。未来若当前release应用证实脚本计算是主要瓶颈，在独立实验workspace中重新比较AST/Grain/JIT的启动、整帧P95、内存和长期行为，再决定后端扩展接口。**不让主开发现在为了消化旧PR而接手重写。**

CI没有启动是账单/额度问题，不是上述决策依据。详见 [#14报告及历史数据重新计算](pr-14/report.zh-CN.md)；百分比是旧微基准，不是本轮重测或当前应用性能预测。

## Issue 决策表

| Issue | 处理责任 | 排期与最小交付 |
| --- | --- | --- |
| [#93 状态map默认值](https://github.com/eddix/gpui-rhai/issues/93) | 作者可补文档/测试，主开发审decode诊断 | **0.1.8**：非空map/array递归编码示例、字段路径错误；不用同时接受两套语法 |
| [#98 图片内容刷新](https://github.com/eddix/gpui-rhai/issues/98) | 作者补Host前台桥示例；主开发负责后续资产版本模型 | **0.1.8补接入说明**；per-asset强一致失效API后续独立设计，不作为本轮新增前置条件 |
| [#95 主题mono/列字体](https://github.com/eddix/gpui-rhai/issues/95) | **主开发统一设计a+b**，dogfooding验收；span另拆 | **默认0.1.9**：theme role、Table两种数据源、测量和Studio round-trip一起做 |
| [#91 capability调用来源](https://github.com/eddix/gpui-rhai/issues/91) | **主开发独立运行时设计** | **0.1.9**：可信origin/归属/异步边界；不要以“最近有输入”代替来源，也不顺带做通用权限系统 |
| [#89 Table context request](https://github.com/eddix/gpui-rhai/issues/89) | 主开发先定事件契约，作者可实现并提供真实用例 | **0.1.9独立Table增强**：row/column/window anchor、两种数据源、键盘入口与选择语义 |
| [#83 SplitPane自定义grip](https://github.com/eddix/gpui-rhai/issues/83) | 主开发native交互/命中/绘制契约，作者提供设计验收 | **按原计划0.1.9**：装饰节点、溢出命中、paint顺序、状态信号及theme token，不只增加node插槽 |

### #98 的关键澄清

底层已有公开 `AssetRegistry::refresh_namespace`；可信同步CapabilityHandler不要求Send，可通过configure_runtime保存registry clone，在后台任务完成后的**前台**回调中使用。前台NativeHandler/Host GPUI路径可处理重绘。后台TaskWork只传AssetId/revision或纯数据回前台，这是正确线程边界，不应为了“后台能调用”把整个registry改成Arc<Mutex<_>>。

现有namespace刷新不是强一致的单资产更新协议：它可能同步重读整个namespace的缓存内容，pending-only的旧decode也没有完整revision淘汰语义。五项有界API特征检查确认了这些边界。Cadenza已有versioned AssetId绕行，本轮可以先官方化该模式和前台桥例子；后续统一做按资产revision发布、旧异步结果丢弃、last-good及所有共享View重绘。不要只暴露一个Rhai刷新函数就宣布解决所有缓存问题。

详见 [#98/#83/#89报告](issues-assets/report.zh-CN.md)。

### #95 为什么不交给应用方做几行字体补丁

不仅缺少font字段：新增code role目前还会被两个白名单拒绝，Theme Studio导出也必须保留配置；NativeCollection列配置经UiValue严格投影，不能直接塞Style对象。每列字体还必须与auto-fit测量一致。span的font override不自动拆run，底层逐run字号/行高能力也有边界。

应扩展现有TypographyToken/ResolvedTypography，复用平台mono策略并允许theme/Host覆盖；避免组件各自写死字体。a+b统一交主开发，c独立规格。详见 [#95报告](issues-typography/report.zh-CN.md)。

### #91 为什么需要主开发设计

ExecutionPhase::Event同时用于真实事件与异步交付，不能作为用户激活凭据。Automation、native deferred proposal、timer/task/subscription等需要一套可信来源与View归属；同步嵌套继承，异步资格重置。origin是事实，是否授权某capability是Host政策，二者不能只用一个bool混在一起。

不需要为了旧handler兼容保留call/call_with双轨。也不把目前缺少新能力判成现有权限漏洞。详见 [#91/#93说明](issues-runtime-policy.md)。

## CI事实与验证边界

| PR | 当前CI事实 | 不能由此推出什么 |
| --- | --- | --- |
| #94 | portable SUCCESS，head为f6e936a5 | 不代表第四轮新反例已关闭 |
| #96 | Format失败，engine新测试排版 | 后续测试没跑，不能声称只剩格式也不能声称产品测试失败 |
| #97 | Format失败，两处helper排版 | 同上，仍需完整CI |
| #99 | 严格Clippy失败，测试`unused_mut` | 删除mut不等于主题行为已经通过 |
| #14 | job未启动，账户额度/账单限制，steps为空 | 没有可用测试结果；不将红灯用作代码失败证据 |

完整PR/issue正文、diff、CI元数据和失败日志保存在 [evidence](evidence)。

独立验证采用不同层次并在各报告说明：#99直接运行固定PR snapshot；#96只刻画固定f6e936a5的预算/parser边界，未声称PR全量测试；#97提取固定PR校验函数，配合与基线相同的native key分发代码作特征检查；#98为相关源码hash与基线一致的公开API特征检查；#14/#95/#91为固定源码与既有证据评审。没有对全部PR做完整workspace/package/release门槛复跑。

## 建议的合入次序

1. 主开发将#94作为第三轮已验收改进合入，继续以独立整改提交关闭第四轮3P1/5P2，避免混入无关功能。
2. 作者分别收窄/修订#96、#97；#99由主开发保留作者提交并补主题闭环。保持独立PR，合入时针对最终主干重新运行检查，不用某个旧head的绿灯代替集成结果。
3. #93与#98的文档/示例使用小PR加入0.1.8。#98若保留为增强issue，不应因文档合入就声称per-asset新API已交付。
4. 合并后的同一最终commit执行项目既有完整release gates，然后才决定tag和发布；#95/#91/#89/#83不阻塞这一轮收口。
5. #14的处置另行执行，保留研究资产，不让它成为主线发布队列中长期含糊的“待合入功能”。本报告只建议，没有替用户执行关闭。
