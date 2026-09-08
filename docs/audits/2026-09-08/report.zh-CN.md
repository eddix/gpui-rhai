# gpui-rhai 全面工程审计

审计基线：`d4a7901b26fa91311f3228d768037e1d52531e16`，2026-09-08。面向维护者与后续实施模型。

**结论：保留现有架构方向，优先修复执行预算、事务完整性与资源生命周期，再推进性能和功能扩展。当前测试全部通过，但不足以证明关键契约完整。**

本次没有修改产品代码。交付包括本报告、[实施任务书](implementation.zh-CN.md)、[探针运行说明](reproduce.md)、[可执行探针](probes.rs)和[原始证据](evidence/metadata.json)。中文报告是审计意见，不替代仓库已有的英文规范。

**第二轮已补充 [第一性原理复审](first-principles.zh-CN.md)**：新增7组已复现问题，重点是窗口/组件/程序的身份域、timer状态语义和流背压。撤回A07后，两轮合计有效发现23项（P1: 10，P2: 13）。实施前应同时阅读两轮证据，优先处理B01的多窗口作用域问题。

## 1. 范围、证据与判断边界

审计时工作区干净，仓库有 342 个跟踪文件、93 个 Rust 文件、约 83,649 行 Rust；约 27,880 行属于测试，计数包括内联测试与独立测试工作区。另有 50 个官方源码组件、19 份 ADR、三个主工作区 crate。

本次建立了整个文件、模块与验证流程地图，深入检查执行入口、组件调用、状态事务、生命周期、热重载、异步交付、源码安装更新、值转换、性能关键路径，并抽查渲染、输入、文档、无障碍和 registry 契约。**没有声称逐行证明全部 8.4 万行代码正确。**

证据等级：

- **R：实测复现。** 使用当前项目公开 API 或对应边界的最小调用序列；探针及输出一并交付。
- **S：代码路径已确认。** 明确指出触发条件和缺失的处理；尚未为完整平台路径新增端到端测试。
- **G：工程治理缺口。** 表示缺少可执行的验证或契约，不等同于已发生生产故障。

优先级在本报告中按迭代顺序定义：P1 为发布或扩展前应修复的可靠性问题，P2 为下一轮质量建设项。未确认需要按 P0 处理的远程攻击或不可恢复用户数据事故。脚本是应用自有代码，敌对脚本的进程级沙箱不是项目目标；下述资源与错误问题主要影响开发反馈、宿主可用性和契约可信度。

### 本次执行结果

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 通过 | [fmt.log](evidence/fmt.log) |
| 全工作区、全目标、全特性测试，locked/offline | 409 passed | [workspace-tests.log](evidence/workspace-tests.log) |
| 独立原生 GPUI 工作区，locked/offline | 49 passed | [native-tests.log](evidence/native-tests.log) |
| 全目标、全特性严格 Clippy | 通过 | [clippy.log](evidence/clippy.log) |
| core 全特性与 CLI lib 严格 rustdoc | 通过 | [rustdoc.log](evidence/rustdoc.log) |
| core `cargo package --locked --offline` | 包含 70 个文件，重新编译验证通过 | [package.log](evidence/package.log) |
| PNG 基线资源检查 | 38 个文件的头部、尺寸、数量通过 | [visual-assets.log](evidence/visual-assets.log) |
| release Table / Document 基准 | 2 passed；Table 3 warmup、10 samples | [Table JSON](evidence/performance.json)、[Document JSON](evidence/document-performance.json) |
| 新增故障探针与 release 微基准 | 见逐项发现 | [probes.log](evidence/probes.log)、[release-micro.log](evidence/release-micro.log) |

运行环境为 Rust 1.94.0、aarch64 macOS 26.6.2；GPUI 精确锁定 0.2.2，Rhai 精确锁定 1.26.0。基础 Rhai features 包括 `internals`、`metadata`、`serde`，不启用 `sync`；全特性检查另启用 `grain`。实际依赖源码已核验，未把技能文件里的历史描述当作现状。

未重新执行：真实桌面截图/IME/屏幕阅读器认证、全部 release GUI 启动 smoke、托管 Linux 或 Windows CI、registry/CLI 干净打包安装、最新漏洞数据库及许可证全矩阵、120 Hz 完整帧认证。基准期间机器负载未严格控制，硬件元数据返回 `unknown`；数值只用于本次观察，不据此声称相对历史版本发生了性能回退。


维护者补充的明确约束：**0.1.1 的破坏性变更符合当前快速迭代设计，不要求历史兼容，不引入旧 API alias、兼容层或旧版本支持矩阵。原 A07 已撤回，不计入缺陷；保留编号空位，以免已有审计引用错位。** 本报告不建议因此更改发布编号或 runtime API 代号，也不把旧源码在新运行时中失败视为 bug。

## 2. 值得保留的设计

1. **`UiNode` 与 GPUI 对象隔离成立。** Rhai 构造声明，Rust 持有 native entity、布局与绘制机制。HostCallback 是显式宿主扩展，文档没有把它伪装成可序列化纯数据。
2. **依赖与资源所有权已经明确建模。** 正式组件调用配方、字段/路径读依赖、effect activation、timer、ref、signal、generation，是后续增量执行的正确基础。最近的 node-prop ownership 修复尤其值得保留，并已有多种失败和复用用例。
3. **数据规模与视图规模已有分离。** NativeCollection、NativeTextDocument 和虚拟投影减少了 Rhai 大对象传输；真实 GPUI 测试与性能测试独立工作区避免将 `test-support` 混入发布图。
4. **项目有实际验证资产。** 409+49 项测试、源码组件契约、明确记录平台限制的文档、带原始样本的性能输出，远好于仅依赖“能启动”。需要提升的是验证的故障模型和覆盖入口，而非追求测试数量。
5. **暂缓 Grain 的判断正确。** 当前生产仍由 AST 解释器执行，Grain 只做特性化测试。现阶段主要瓶颈不应归因于“解释器不够快”。

## 3. 发现总览

| ID | 优先级 | 证据 | 发现 |
| --- | --- | --- | --- |
| A01 | P1 | R | 嵌套执行预算漏计、延迟回调继承旧预算 |
| A02 | P1 | R | 普通脚本 const Map 赋值触发未隔离的 Rhai panic |
| A03 | P1 | R/S | 热重载预加载失败绕过恢复，破坏旧组件注册表 |
| A04 | P1 | S | 外层事务不包含已提交的 lifecycle 树，组合提交可能分裂 |
| A05 | P1 | S | 异步整批取出后统一回滚，失败会丢失独立消息 |
| A06 | P1 | R | 已取消异步资源无法被当前快照恢复 |
| A08 | P1 | R/S | 组件挂载二次增长，热路径过度复制全量状态 |
| A09 | P2 | R | `RuntimeEngine::new()` 默认允许文件系统 import |
| A10 | P2 | R/S | 后台 task panic 留下永久活动项，异步资源缺少整体限额 |
| A11 | P2 | R | CLI 多文件计划失败后留下部分写入 |
| A12 | P2 | S | 更新计划没有协调新增依赖、资产与本地清单 |
| A13 | P2 | R | 自制 import 词法分析不接受合法 Rhai 源码 |
| A14 | P2 | R | `UiValue` 接受非有限浮点，破坏相等与序列化契约 |
| A15 | P2 | S | 原始事件追踪绕过 sensitive 字段策略 |
| A16 | P2 | S | automation 报告派发成功，但隐藏执行失败 |
| A17 | P2 | G | CI、基线和发布证据缺少统一且可追溯的覆盖清单 |

第二轮新增项的完整触发条件、代码位置、整改和验收标准见 [B系列详细报告](first-principles.zh-CN.md)。所有B系列条目均有公开API实测证据：

| ID | 优先级 | 发现 |
| --- | --- | --- |
| B01 | P1 | standalone多窗口的NodeId、geometry和capture相互干扰 |
| B02 | P1 | 相同key重挂载后，旧callback/signal误写新实例 |
| B03 | P2 | 独立候选共享generation，旧callback被误接受 |
| B04 | P2 | timer丢失暂停原因，签名遗漏callback curry |
| B05 | P2 | 重复asset provider注册被拒绝，却已替换旧provider |
| B06 | P2 | Store schema与Host signal入口绕过值域校验 |
| B07 | P1 | receiver adapter遇到背压后断流并丢失消息 |

### A01 — 执行预算的计量单位没有覆盖嵌套调用

位置：`engine.rs:500–507, 1448–1466, 1681–1705, 2022`；`invocation.rs:20–30, 60–70`。上游：Rhai `types/fn_ptr.rs:440–458`、`func/native.rs:208–215`、`eval/data_check.rs:184–205`。

`on_progress` 把当前 evaluator 的绝对计数覆盖到一个原子变量。`FnPtr::call_within_context` 克隆 global runtime 后执行嵌套函数；返回父调用后，父计数继续从较小值增长。因此记录最终绝对值既不是嵌套总量，也无法提供跨 evaluator 的一次调用总预算。

复现一：4 个正式组件各循环 200,000 次，总计 800,000 次循环。相同循环体单次迭代约 3 个操作，实际已超过 1,000,000 上限；整个 render 仍成功，`ExecutionTiming.operations` 只有 **34**。

复现二：同一个 100,000 次循环的回调在轻量 root 下执行 **300,007** 个操作成功；root 先循环 250,000 次，再创建该回调，之后独立点击仅执行到 **249,977** 个新操作就报 `ErrorTooManyOperations`。root 先循环 300,000 次时，只剩 **99,977**。`operation_base` 只修正展示差值，没有消除继承计数对限制的影响。

影响：预算既会漏放，也会误杀；Inspector 与基准中的 operation 总量不能支持当前文档所称的“包含 imported nested work”。耗时测量仍有价值，不能连带宣布全部耗时数据失效。

整改：定义一次宿主执行会话及其共享预算。同步嵌套调用消费同一个预算；延迟 callback/timer/effect/virtual invocation 开始新会话。进度回调每次真实触发的消费量与 evaluator 的绝对编号分离；存储的 module provenance 不应携带过期的消费额度。适配层处理 Rhai global counter 时必须同时特性化调用深度等继承状态，不能只把显示值清零。

验收：轻/重父 render 下同一个延迟回调的可用额度相同；多个嵌套组件合计超限必须失败；叶子工作增加时总量单调增加；失败与重载后额度复位；curry、同名 imported helper、嵌套 effect 和 virtual renderer 的来源不变。重建修复后的 operation 基线，给计量语义加版本标识。

### A02 — 脚本错误越过 Result 边界成为 Rust panic

位置：`engine.rs:869–891` 等 AST/FnPtr 调用入口；上游 `rhai-1.26.0/src/eval/stmt.rs:376`。

复现：`fn view() { const m = #{ x: 1 }; m.x = 2; text("done") }` 可以 compile，但 render 触发上游 `unreachable!`。探针仅在最外层用 `catch_unwind` 捕获，以便继续输出；生产执行入口没有对应隔离。

影响：常见脚本编辑失误不能进入 last-good/error-banner 路径；在宿主选择 `panic=abort` 时会终止进程，unwind 模式也会越过现有 Result 回滚逻辑。这里不需要敌对脚本假设。

整改：先为上游缺陷建立子进程特性化测试，选择经过验证的上游修复/最小补丁或可靠的执行前拒绝策略。对可恢复 panic，需在统一执行边界恢复/废弃完整候选状态；禁止简单吞掉 panic 后复用未经验证的 Engine。`catch_unwind` 不能处理 abort 或栈溢出，不能宣称成为进程沙箱。

验收：同类 const Map、Scope 常量及相关嵌套写操作返回结构化诊断；旧树、状态、回调和组件注册表仍可正常工作；后续修正脚本能够热重载。测试进程本身不得被 panic/abort 中断而误判为超时。

### A03 — 热重载准备阶段已经修改活动 Engine

位置：`app.rs:4229–4320`，特别是 `4259–4264`；`engine.rs:1552`。

`reload_scripts` 备份 exports/renderers 后先清空注册表、更换 resolver，再 `preload_component_modules(...)?`。预加载在语法合法但模块初始化不纯、定义不合法、导入依赖错误等情况下失败，会直接返回，绕过后面的恢复分支。旧 AST 保留了，但旧组件注册表已被清掉或只加载了一部分。

探针复现对应公开调用序列：成功加载正式组件；清空 exports；对包含 `let mutable_global = 1` 的候选模块预加载，得到错误；再 render 旧 AST，报 **“has no registered render function”**。这验证了破坏机制；完整 notify/文件观察器路径尚未新增端到端测试。

整改：将候选 resolver、module cache、exports、renderers、AST、schema、stylesheet 放入独立的 program preparation 对象，全部准备成功后再交给 lifecycle；或用覆盖所有提前返回的 RAII 恢复机制作为最小修复。所有准备错误都必须经过同一个退出点。关联检查 `reload_changed_paths`：同批脚本/主题/样式修改目前按次序提交，后项失败不回滚前项，应明确是否以文件批次为原子单位。

验收：语法错误、初始化纯度错误、缺失 import、组件定义错误、stylesheet 错误分别注入；每次失败后让旧页面发生一次真实状态变化并重渲染，不能只断言旧 root 字符串仍在。之后修复文件无需重启即可恢复。

### A04 — “外层回滚”未覆盖 lifecycle 已提交快照

位置：`app.rs:3916–3955, 4013–4032`；`lifecycle.rs:396–407`。

`ScriptViewTransaction` 只保存 runtime snapshot 与 Engine checkpoint，不保存 `ScriptLifecycle.root/retained/state`。`poll_async` 在一个外层事务中先执行 `realize_virtual_requests`，再执行 `render_dirty`；前者成功会立即替换 lifecycle root 和 retained tree。后者失败时外层恢复 Engine/runtime，前者已提交的树仍保留。

触发条件：同一轮既有需要实现的新虚拟项，又有稍后失败的普通 dirty render。结果可能是显示树、invocation/资源注册表与状态属于不同提交。不能因为两个单独函数各有回滚，就推导组合操作原子。

整改：采用“候选阶段可组合、外层唯一 publish”的生命周期 API，或将 root、retained、report、状态及相关 program 元数据纳入统一 checkpoint。梳理所有一次事务调用多个会提交的操作，包括 resume、reload 和未来组合命令。避免通过取消虚拟化或强制所有更新 full render 掩盖问题。

验收：在虚拟投影成功之后、dirty render 提交之前注入故障；比较完整状态、retained NodeId、可视文本、reconcile report、invocation、ref、signal 与 effect activation 指纹；恢复后应与操作前一致，下一次正常交互可完成。证据等级 S，需要在真实 GPUI host 测试中落地。

### A05 — 异步批次的一次失败会丢失其他消息

位置：`app.rs:3981–4032`；`async_runtime.rs:280–292`；`context.rs:290–302`；`app.rs:3702–3710`。

任务、订阅、timer 先 drain 并从队列/活动表中移除，然后才创建脚本事务。多个 delivery 放进同一个循环，任意一个失败即 `?` 退出；之前成功消息的状态被回滚，之后尚未执行的消息随 Vec 丢弃，而外层快照里已经没有这些消息。

另外，暂停视图把已 drain 的结果塞进 256 项 pending 队列；满时返回错误，但溢出部分没有返回给 producer，也不再留在 task registry 中。当前“backpressure”实质上是取出后的丢弃诊断。

整改：定义 delivery envelope、序号与处理状态。独立消息使用独立提交；需要批次原子的消息应在提交后 ack。失败消息应有明确的终态/隔离策略，后续消息保持顺序和可达。暂停容量必须在出队或 task 接纳前检查，或者保存可重试的剩余队列。不可无限自动重放可能包含外部副作用的 callback。

验收：A 成功、B 抛错、C 成功三条消息；若使用独立提交，A/C 最终各生效一次且 B 明确失败；若选择原子批次，三条均须仍可按明确策略处理。另测 timer 与 subscription 混合批次、暂停时第 256/257 项、恢复失败再重试、旧 generation 丢弃。不能只检查最终队列长度。

### A06 — cancellation 不是当前 snapshot 可恢复的状态

位置：`context.rs:518–565, 2230–2258`；`async_runtime.rs:303–304, 666–675, 772–784`。

快照只记录 task/subscription/decode 的活动 ID，恢复时调用 `retain_ids`，只能取消新资源，不能重建已移除资源。宿主事务可以先取消旧订阅，再失败；组件替换过程中，`commit_component_renders` 也会通过 `context.rs:461–472` 的生命周期清理取消旧组件资源，之后新 effect start 仍可能抛错。UI 状态回滚，旧资源却不能复原。

实测一：创建订阅 → snapshot → cancel → restore，活动订阅数仍为 **0**，旧 emitter 的关闭原因为 `Cancelled`。实测二：旧组件拥有订阅，切换到另一个 key 的组件，其新 effect start 抛错；`ScriptLifecycle::render_dirty` 恢复了旧 UI 树，但旧订阅活动数为 **0**、关闭原因为 `ScopeDisposed`。这是 runtime 自己管理的资源损失，不只是文档排除的“外部能力副作用”。

整改：事务内取消先记录 cancellation intent，提交后再向 producer 发布不可撤销关闭；事务内后续操作可观察逻辑已取消状态。若不支持恢复，必须在 API/事务结果中表达部分完成并进入可恢复生命周期，不能恢复一个声称 activation 存活、实际上 producer 已关闭的 descriptor。effect cleanup 中显式取消和 unmount 清理尤其需要覆盖。

验收：旧 task/subscription/decode 的 cancel→失败均保留有效投递能力；新资源 start→失败不投递；同 key effect 重启与旧 activation 清理不相互取消；清理回调抛错后 runtime 与 producer 的生命周期一致。

### A08 — 状态模型的复制成本破坏增量执行收益

位置：`engine.rs:2337–2368`；`state.rs:169–185, 216–225`；`context.rs:518–552`；`app.rs:3423, 3873, 3916`；`lifecycle.rs:326–351`。

正式组件进入 render 时调用 `StateStore::mount_instance`。该函数先 `begin_render()` 克隆整个实例表，再挂载单个实例。N 个组件首次挂载会重复复制不断增长的全表，形成约 O(N²) 的结构性成本；已有 N 个实例时，挂载/更新少数实例仍涉及无关状态。

当前 release 微基准使用空 schema，5 次取中位数：200/400/800/1,600 个实例约 **4.92 / 10.92 / 31.64 / 126.04 ms**。这是状态挂载机制测试，不是完整 UI 帧；800→1,600 约四倍，与代码复杂度一致。

另一条链路中 `UiRuntimeState::snapshot` 深复制 store、component state 等；10,000 个两字段 Map（100 字节 label）的 release 快照 p50/p95 为 **4.395/4.495 ms**。`sync_viewport_class` 每次 host render 都先开事务，之后才判断 viewport 是否变化；`realize_virtual_requests` 也在确定有请求前先创建快照。NativeHandler 写 signal 时同样进入全量事务，所以“零 Rhai 调用”不意味着整个路径与 UI 规模无关。

整改顺序：先把无变化检查移到昂贵快照之前；再让一次正式 render 共享同一状态 staging，单实例 mount 只复制/修改受影响 entry；最后根据测量选择写入日志、按实例 COW 或持久化结构。不要用所有状态都改 `Rc<RefCell<_>>` 的方式消除复制，那会让浅快照失去回滚隔离。

验收：加入 100/1,000/5,000 个正式组件的初次/局部/根更新基准；在固定改动集下，无关 store 数据增大不应线性增加 native signal 提交成本；记录 clone/allocation、peak bytes、script、state stage、reconcile、GPUI layout/paint 的分项。先做结构门槛，再在受控 Mac 上评价绝对时间。

### A09 — 默认 Engine 与文档的 import 限制不一致

位置：`engine.rs:499, 668, 1808–1815`；CLI `theme_studio.rs:354–355`、`lib.rs:558–565`。

`RuntimeEngine::new()` 使用 `rhai::Engine::new()`，配置 limits 后没有替换默认 FileModuleResolver。高层 File/Embedded 主程序路径会安装 RestrictedModuleResolver，但默认公开 Engine、若干独立主题/locale/样式加载路径并未统一约束。

实测：新建 RuntimeEngine 后，绝对路径 import 成功读取探针自己创建的临时 `.rhai` 文件，未经过 ScriptSource 注册。未读取用户私密文件。上游 `engine.rs:299` 明确安装 FileModuleResolver。

整改：默认安装拒绝所有未声明 import 的 resolver，再由程序准备路径显式注入来源；为可信配置文件提供清楚的 import 策略，而不是依赖调用者记住替换。保留支持宿主自定义 resolver 的扩展能力。

验收：默认 Engine、File/Embedded prepare、CLI check、Theme Studio import、theme/locale/stylesheet loader 分别测试绝对路径、`..`、未声明模块与合法逻辑 import。报告中不将此扩大为任意文件字节读取或远程执行漏洞。

### A10 — task 生命周期与资源限额不完整

位置：`async_runtime.rs:188–205, 229–290`；`context.rs:2310–2330, 2380–2398`；`capability.rs:130–140`；`budget.rs:4–44`。

task 每次生成独立 OS thread；结果 channel 无界；RuntimeBudgets 未覆盖任务、订阅数、排队 payload bytes 或 worker 并发。取消仅去除 delivery 注册，不通知 task work 停止。订阅 `from_receiver` 阻塞于 `recv()`，若 sender 长期存活又不发消息，取消 emitter 不会唤醒它。

task work panic 时没有发送完成/失败消息。实测 worker 已退出后：**0 deliveries、active_count=1**，资源条目持续存在。这里应与宿主提供的无限阻塞代码区分：runtime 可以控制 admission、可观察终态、合作取消和队列边界，不能强行安全中断任意 Rust 函数。

整改：明确任务状态机；catch worker unwind 并产生终态；使用有界执行器与队列，设置 task/subscription/payload 总限额，增加合作取消 token；内置 channel adapter 支持取消唤醒。被取消但仍在运行的 native work 应单独计数，不能因 callbacks 被丢弃而从预算中消失。

验收：正常、错误、panic、spawn 失败、取消、迟到结果、view dispose 七类终态；在长时间 mount/suspend/unmount 循环后工作线程/注册项回到基线；过载明确拒绝，不创建无限线程；不通过睡眠等待碰巧成功来判定。

### A11 — CLI 原子性只到单个文件

位置：CLI `lib.rs:1494–1535`；`docs/source-updates.md`。

所有期望内容先校验，随后逐个 temporary-write/rename。第 k 个文件失败时，前 k−1 个已写入，不会回滚。临时文件名固定为 PID+index，且用 `fs::write`，缺少唯一创建与清理。计划执行期间的后续写入仍存在检查与替换之间的竞争窗口。

复现：生成 init plan 后，在第二个临时文件的位置放置目录，不修改任何目标文件。apply 返回错误，但 Cargo.toml 已加入依赖，`ui/main.rhai` 尚不存在。文档所述“source 和 baseline 原子更新”不能从逐文件 rename 推出。

整改：先在专属 staging 目录完成所有写入与验证；为项目级 commit 设计锁、journal 与故障恢复协议。保护应用源码、baseline 和 manifest 的一致性，明确区分单文件原子替换、并发保护和进程崩溃恢复。至少保证可检测、可恢复的部分应用状态；不要只在成功路径删除 temp 文件。

验收：第 1/中间/最后一项 write/rename 失败、并发修改、重复 apply、遗留 staging 后恢复；无静默覆盖。真正的 crash consistency 需子进程 kill 故障注入，不能用一个普通单测声称已证明。

### A12 — update 只升级源码版本，未升级整个依赖图

位置：CLI `lib.rs:657–711`，对照 `plan_add:427–493`。

`plan_add` 会解析传递依赖并安装组件/资产/baseline；`plan_update` 只遍历已安装组件、合并源码、更新 version/hash，没有解析新版本新增依赖、安装/合并新增资产或更新 `installed.dependencies`。

触发条件是将来任一组件升级增加 import 或 icon。当前全部官方组件同步更新掩盖了该问题，不需要声称现有某一具体官方升级已经缺资产。

整改：计算目标 registry 的完整依赖闭包与源码/资产一致性，使用与 add 共用的安装/资产策略，再生成一个完整 ProjectPlan。共享资产与本地修改必须保留来源和冲突信息。组件内容发生变化但版本没变时提供 registry 发布检查，不能静默认为无需更新。

验收：合成 upstream v2 新增直接/传递依赖与新 icon；已有应用修改其中一个共享资产；dry-run 零写入、冲突清晰、成功更新后 fresh `check` 和真实 lifecycle 均通过。

### A13 — import 扫描器与 Rhai 语言不一致

位置：`dependency.rs:81–182`。

tokenizer 处理普通双引号和不嵌套的块注释，没有正确处理 Rhai 反引号模板字符串及嵌套块注释。以下两段都能由当前 Rhai compile，但 `extract_imports` 报非法路径：

```rhai
/* outer /* inner */ import "../not_an_import"; */
fn view() { text("ok") }

fn view() { text(`example: import "../not_an_import" as demo;`) }
```

影响 compile_self_contained、dependency graph、CLI、热重载；CodeViewer 或文档示例包含 import 字样尤其容易触发。

整改：优先使用经过限制配置的 Rhai 解析结果提取 import；若必须预扫描，建立遵循 pinned lexer 的状态机并以 Rhai 自身为差分 oracle。模板中的插值表达式和纯文本必须区分，不能简单跳过整个反引号字符串。

验收：嵌套注释、模板文本、插值中的真实 import、转义、Unicode、动态 import、不同换行与未闭合输入；既不伪造依赖，也不漏掉应禁止的动态/外部导入。

### A14 — 通用值绕过有限浮点验证

位置：`value.rs:111–118, 326–342`；`schema.rs:331–335`；`engine.rs:176–185`。

`ValueSchema::Float/Number` 会拒绝非有限数，但 `ValueSchema::UiValue` 使用 `UiValue::from_dynamic`，后者直接接受 NaN/Infinity。实测 NaN 校验成功、`value != value.clone()`；JSON 输出 `{"type":"float","value":null}`，反序列化回 UiValue 失败。ScriptCallback 声明 `Eq`，其 curry 却可包含这种非自反数据。

整改：在 durable value 边界统一选择有限数策略；建议拒绝非有限值并报告路径。同步检查 Rust 构造、serde 反序列化、state/store/default、capability output 和 callback curry，而非只修 Rhai 入口。为递归值增加深度、总项数与字节预算；UiValuePath 的 64 段限制不是数据树本身的深度限制。

验收：NaN/±Infinity 在全部允许持久化入口行为一致；已接受值的 round-trip 与等价关系成立；深层/大规模输入有有界失败；Node/FnPtr/HostCallback 等已有拒绝规则保持不变。

### A15 — sensitive 只在部分展示路径生效

位置：`app.rs:3820–3827`；`devtools.rs:65–82, 580–587`；`context.rs:889–901`。

state/store 写入追踪会查 sensitive 标记；`handle_node_event` 却把原始 payload 以 `sensitive=false` 放入 TraceBuffer。输入 change payload 可能就是标记为 sensitive 的字段内容，因此 Inspector 可以一边隐藏状态值、一边在事件轨迹显示同一内容。

另需区分 raw InspectorSnapshot 与对外安全快照：当前 raw state/trace 结构仍保留原值，脱敏多发生在展示函数。没有观察到外传行为，风险主要是开发面板、宿主导出和后续诊断集成。

整改：事件 payload 默认不记录或按显式事件敏感策略记录；提供专门的 redacted diagnostic projection，并在输出边界验证，而非要求每个调用者自行调用 display_value。敏感性不能只靠 callback 名字推断。

验收：敏感输入经过原生 change→脚本 state→错误→Inspector/诊断导出完整链路，原始值不能出现在安全输出中；普通非敏感事件仍能提供足够排障信息。

### A16 — automation 的“成功”缺少执行结果

位置：`app.rs:3158–3176, 3190–3268`。

`AutomationCommand::Action` 调用 `handle_node_event` 后忽略返回语义，始终返回 `Ok(Action { id })`。dispatch 路径也只累计 invoked/propagation，脚本错误在 host 中被转换成 stop 并写入 last_error，命令本身仍返回成功。自动化消费者难以区分“正常停止传播”和“回调或重渲染失败”。

整改：将执行结果与 EventResponse 的传播策略分开；让 automation 返回结构化 failure 或包含每次执行的事务结果。last_error 仍可用于 UI，但不能作为唯一机器可见错误通道。AdvanceTime 引发异步失败也需要同样策略。

验收：成功处理、正常 stop、callback throw、dirty render failure、失效 target/generation 分别产生确定结果；测试必须断言事务结果和实际状态，不能只断言 invoked > 0。

### A17 — 认证证据未绑定到可执行覆盖清单

位置：`.github/workflows/ci.yml`；`scripts/audit-visual-baselines.sh`；`scripts/release-smoke.sh`；`docs/core-runtime-v2-audit.md`；`tests/visual/macos/README.md`。

- PNG 审计只读取前 24 字节和尺寸/数量，既不完整解码，也不产生当前截图或比较像素。它是资源完整性检查的一部分，不是视觉回归测试。
- 当前实际有 21 个 Rust examples；smoke 硬编码 19 个，缺少新增 `code_viewer` 和 `diff_viewer`。构建全部 examples 不等于全部实际启动。
- 文档与工具各维护独立计数/清单，原生测试说明仍有 42 个测试的旧数字，当前实测是 49。证据账本日期与后续改动未明确绑定 commit。
- CI 没有执行独立 performance workspace 的结构性测试。主工作区 all-features 不能代替每个 feature 组合；当前原生测试和 package 确实提供了部分 default 覆盖，应保留这一事实。
- CLI package 固定 `--no-verify`，注释仍为“依赖未进入 index 前”。应该有正式发布路径保证依赖入库后干净安装，而不是永久跳过。
- 120 Hz、AX forwarding、真实 IME 和新 Gallery/Studio PNG 扩展在账本中仍是缺口。不能用本次绿色测试替它们签字。

整改：建立机器可读的 verification manifest，声明每个 product example、feature、平台和契约对应的自动/手动 gate。由脚本从同一清单生成执行任务和证据摘要。基础 CI 跑结构不变量，不在共享 runner 上设绝对毫秒门槛；受控 Mac 收集 fresh frames、完整解码、视觉差异及真实输入认证。

验收：新增产品 example 或 feature 时缺少 gate 会失败；release 证据包含 exact commit、dirty、toolchain、features、OS、硬件、命令与结果；旧图片无需重绘也能通过的检查，不得命名或描述成当前版本视觉验收。

## 4. 架构演进建议

### 收敛事务所有者，再拆分文件

`app.rs` 5,237 行、`engine.rs` 4,149 行、`context.rs` 3,948 行、`lifecycle.rs` 2,509 行。问题不在行数本身，而在“谁拥有候选状态、谁能 publish、谁恢复失败”分散在多个模块，并通过长长的 clone/restore 字段列表维持一致。

先明确 app 共享数据与 view/scene 局部资源的所有权，再按责任提炼内部边界，保持公开门面可控：

| 内部边界 | 拥有的契约 | 不应拥有的责任 |
| --- | --- | --- |
| View/scene resource domain | NodeId域、几何、presented frame、pointer capture、组件incarnation | 其他窗口的局部资源或app共享store |
| Program preparation | source graph、resolver、AST、exports/renderers、schema、配置校验 | 活动 UI 树的提交 |
| Execution session | generation、调用 provenance、共享预算、诊断、执行错误 | native entity 生命周期 |
| View transaction | runtime + engine + lifecycle candidate 的唯一提交点、资源变更意图 | 任意外部 capability 效果回滚 |
| Delivery supervisor | admission、队列、ack、取消、producer 终态 | 解释 component props 或 GPUI 布局 |

先用最小故障用例固定契约，再逐一迁移入口。不要同时把 50 个组件、Host API、状态系统和 renderer 改写；每次移动代码应能证明业务行为不变。

### 收缩不必要的公共可变面

`lib.rs` 暴露绝大多数内部模块，`UiRuntimeState` 有大量公开可变字段；`RuntimeEngine`、`LiveScript`、`ScriptLifecycle`、File/Embedded/PreparedView 都提供不同程度的执行路径。对专业宿主开放底层能力是合理的，但必须明确：哪些只是构造/执行工具，哪些承诺预算、资源所有权、回滚、错误隔离和 last-good。

建立 public API 分类表：普通嵌入门面、可信宿主扩展、专家底层机制、实验能力。根据真实下游使用情况再减少公开面；不要因仓库内没有引用就删公共 API。可以在当前允许破坏性调整的迭代中把内部 registry/checkpoint 操作收回，而不是继续永久扩大稳定承诺。

### 用一份能力清单协调 Rust、Rhai 与源码 registry

component metadata 在源码 header 和 define_component 中重复，目前已有一致性验证，是良好的补偿机制。继续让实际 schema 派生 editor metadata、文档参数和 fixture；避免另写一套独立的必填参数清单。loader、check、metadata、embed 和 runtime prepare 共用验证核心，但允许 check 明确报告“需要宿主注册的 capability/primitive”，不要假装 headless check 能验证任意宿主扩展。

### 将结构化诊断接回真实执行路径

`Diagnostic` 已保留位置、code 和 stack，但 app/CLI 多处马上转成 String，automation 再隐藏执行结果。建议保留类型化错误直到 UI/JSON 最后一步，包含 operation、source、component、generation、transaction outcome 与 retryability。保留原始错误与 rollback 错误两者，避免 `rollback.err().unwrap_or(error)` 丢失最初失败原因。

### 清理应以可证明冗余为前提

优先合并重复的程序准备/验证、重复的范围快照、硬编码 example/gate 清单和历史测试数字。暂不直接删除 LiveScript、底层 renderer、Grain 特性、原生集合或 owned-snapshot 机制：它们涉及外部消费者或已经验证的性能/所有权行为。对候选删除项记录调用点、替代路径、版本影响和保留测试；不做“按文件大小拆成多个 crate”的机械重构。

## 5. 测试策略：从案例数量转向状态空间

现有缺陷的共同特征是**单路径成功，组合顺序失败**。优先补以下可执行契约：

| 测试层 | 应补的 oracle | 重点组合 |
| --- | --- | --- |
| Rhai characterization | pinned interpreter 的真实操作、异常、来源语义 | nested / delayed / imported / curry / const mutation |
| Runtime 状态机 | 相同外部输入下增量与完整渲染的规范化结果一致 | mount→event→effect→reload→suspend→resume→dispose |
| 事务故障注入 | reject 前后 committed fingerprint 相同 | 每个 prepare/validate/reconcile/publish 阶段失败 |
| 原生 GPUI | 真实事件、视觉节点、focus、input entity 和逻辑状态一致 | callback 成功但 render 失败、虚拟与普通更新同帧 |
| Async | 有序可达、终态明确、最多一次提交或显式重试 | batch 中间失败、取消后错误、暂停溢出、迟到结果 |
| CLI 与下游 fixture | plan/apply/check/clean build 为同一个图 | 当前版本源码、本地修改、新依赖、新资源、中途 I/O 失败 |
| 性能与存活性 | work/allocation/resource-count 结构不变量 | 大组件树、小改动、大 store、空闲、多 view、反复挂载 |

状态机/属性测试建议固定种子并输出最小失败序列，初期只覆盖核心高价值状态空间。不要生成海量断言当前函数细节的测试，也不要把完整运行时快照当作唯一 oracle：snapshot 如果漏字段，测试会与实现一起漏掉。

至少保留一种独立证据，例如读取真实 native entity 状态、实际 callback 交付、producer 关闭 token、retained NodeId 与组件调用计数。涉及 UI 结果的测试必须验证后续交互仍能继续。

## 6. 性能结果如何使用

本次 release Table 1,000 行观察值：unchanged p95 5.659 ms、reverse 11.653 ms、selection 21.932 ms、native resize 13.026 ms；resize 的 root operations 为 0。文档基准 20,000 行：直接字符串 prepare 48.774 ms，NativeTextDocument prepare 1.728 ms，resize dispatch p95 5.268 ms、settle p95 31.650 ms。二者是不同延迟概念，不能混作 foreground frame time。

这些结果说明 native data boundary 和复用机制值得保留，但本次没有受控 A/B、没有真实 GPU 完整帧/120 Hz 采样，而且 A01 已证明 operation 归因存在缺陷。下一次性能工作应先修计量、拆分 phase，再比较同机相邻二进制。关注空闲 CPU、每视图 16 ms 定时轮询、多个活动 view 的唤醒次数和大量 component/state 的复制；不要只优化 Table 一个数据集。

## 7. 迭代与发布管理

第二轮B01–B03要求先明确作用域和身份；将PR11的身份设计放在事务重构之前。其余按四个阶段完成整改，具体 PR 见[实施任务书](implementation.zh-CN.md)：

1. **建立可信 oracle。** A01/A02 特性化，记录本次可复现失败；先使失败可观测。
2. **统一状态提交。** A03–A06，连同 A10 的终态和资源预算；每项都测试失败后的下一次正常交互。
3. **改善扩展成本与工具链。** A08、A09、A11–A16；用原有 registry/GPUI 用例证明没有改变已正确的交互。
4. **恢复发布认证。** A17，干净下游安装、fresh 视觉与真实输入、受控性能。尚未支持的 GPUI 能力继续明确记为上游限制。

为后续模型建立一份简短维护说明，比继续增长历史实施日记更有价值。内容包括：规范文档层级、不可破坏的边界、各类变更必须新增的故障测试、测试工作区/特性组合、证据保存位置、发布版本规则。实施模型每个 PR 交付“触发条件→失败用例→最小修复→通过证据→剩余限制”，不得用更新审计文档的“Complete”状态代替完成验收。

依赖管理建议将 exact Rhai/GPUI 升级作为专门任务：读取新源码、运行全部 characterization、比对 public/volatile API、重新生成源码能力矩阵，再跑下游 fixture。独立测试工作区的 lockfile 一并审查。补充漏洞/许可证自动审计与来源归属检查；这是缺失的供应链流程建议，本次未确认某个依赖存在已知漏洞，也未重新作法律合规认证。

审计结论的完成标准是上述契约被实现并可重复验证；“所有现在已有的测试继续通过”只是最低基线。
