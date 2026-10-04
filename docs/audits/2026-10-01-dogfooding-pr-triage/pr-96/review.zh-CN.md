# PR #96 分流评审：Host operation budget / function expression depth

评审对象：[PR #96](https://github.com/eddix/gpui-rhai/pull/96)，head `befbe926ee7d53be2f540538112766ae07aec792`，PR base `815bb82b`；对照主审查基线 `f6e936a5`。所有 PR 行号均指该固定 PR head，不指正在被其他会话修改的本地文件。Rhai 核验基于锁定 `1.26.0` 源码与 rhai-api skill。

## 处理建议

**选择“作者小改后合入”，其中 Host 可配置 operation budget 纳入 0.1.8 这一轮；把全局 function expression depth 从 32 提高到 128 的修改拆出。** 不建议原样合入，也没有理由让主开发接管重做 operation budget 架构。

| 内容 | 决策 | 理由 |
|---|---|---|
| RuntimeEngine setter/getter 与两个 view builder 的 operation limit | 作者补少量测试/文档后，本轮合入 | 实际应用有超过固定 1M 的一次性计算需求；设置属于 Host 权限，默认配额保持不变，现有计数模型可直接复用 |
| 默认 function expression depth `32 → 128` | 拆成独立变更，暂不随本 PR 合入 | 它改变所有应用的默认解析策略，不只是允许 Host 配置；目前给出的真实函数深度约 22 仍小于现有 32，没有足够失败/边界证据支持全局四倍调整 |
| 脚本 `ctx.with_budget(n, Fn(...))` | 不加入本轮 | 脚本自行扩大预算会削弱 Host 配额边界；若将来需要按阶段细分，优先设计 Rust Host 的执行策略，而不是给脚本提额权限 |
| 将全部布局计算迁到 Rust / 重建 execution-policy 框架 | 不作为此 PR 的前置要求 | 当前问题是合理 Host 策略无法表达，增加一个受限配置入口已经足够；跨语言计算布局是另一项产品选择 |

## 正向评价与源码核验

1. **继续使用既有累计计数器是正确方向。** PR `engine.rs:644-652` 的 progress hook 只把固定常量比较改成读取该 Engine 的 `Rc<Cell<u64>>`，没有重置 `OperationTracker`，因此没有把 nested evaluator、sibling component、virtual item 的共享总额意外拆开。没有为每个操作引入分配或锁；这里的开销判断来自源码，没有声称测过 PR 性能。
2. **Host 独占配置权。** `engine.rs:2058` 只暴露 Rust setter，未注册 Rhai setter。提高额度是嵌入应用明确作出的策略决定，不是脚本绕过默认限制。
3. **运行入口传递在源码上齐全。** File prepare `app.rs:1515-1518`、Embedded prepare `:1776-1779`、secondary window factory `:2352-2355` 都在运行脚本前设置限额。dev-reload 的 `candidate_engine` 在 `engine.rs:618-621` 复制当前有效值。
4. **reload 候选与正在运行的 Engine 不共用可变额度 Cell。** `candidate_engine` 先 `Self::new()`，再复制数值，不会因修改候选引擎额度而顺便修改正在运行的引擎。
5. **默认 1M 保持不变。** 普通使用方不需要因这个 Host API 自动提高所有脚本的预算。Rhai 的 `set_max_operations(0)` 仍是刻意关闭 evaluator-local 的绝对计数器，由项目自己的累计 hook 执行约束，不能在适配时改回原生绝对计数而破坏 retained callback 语义。

Host extensions 的 `configure_engine` 当前在 builder limit 设置之后运行，因而它也可以覆盖额度。它属于可信 Host 配置路径，不应误报为脚本越权；文档应明确顺序，或者作者选择统一的优先级并测试。

## 合入前最小修改清单

### 1. 拆出默认解析深度修改

移出 PR `engine.rs:2285-2291`（diff 中 `set_max_expr_depths(64, 32)` 改为 `(64, 128)` 的 hunk）。operation limit API 本身不依赖它，可以独立交付。

Rhai 1.26.0 的实际默认在 `src/api/limits.rs:9-37` 按构建 profile 区分：debug 的 function expression depth 是 16，release 是 32。PR 注释只写“Rhai 默认 16”不完整。

表达式复杂度由 parser 的 `ParseSettings::level_up` 检查，`set_max_call_levels` 限制运行时函数调用深度，operation hook 约束求值步骤。后两者不等价于 parser 深度保护，不能用“仍有递归和 operation 限制”证明解析深度翻四倍的成本无关紧要；`parse stack cost trivial` 也没有测量支持。

独立变更至少应提供一个真实 `ExprTooDeep` 的最小函数、debug/release 解析与执行结果、明确的输入复杂度边界。若需求只来自特定嵌入应用，Host 可配置表达式深度且默认保持 32，比无条件改所有应用更符合本次政策。无需为这个后续设计阻塞 operation budget 部分。

### 2. 补一条真正超过默认额度的正向测试

PR `engine.rs:4944-4970` 的测试明确验证同一份 60,000 次循环在默认额度已经成功；改为 50M 后再次成功，并不能证明“原来被 1M 拒绝的实际工作现在能够完成”。

最小关键对照应为：同一份 AST / 工作量在默认 1M 下被 budget 拒绝，Host 提额后成功且结果一致，再降额后重新拒绝。使用有限循环即可，不需要不受控压力测试。

再补覆盖配置传播的针对性测试：

- File / Embedded prepare 能接收并应用非默认额度。
- secondary window 与 dev-reload candidate 保留额度；候选设置改变或失败不会泄漏到 active engine。
- 在非默认额度下，nested/sibling work 仍共享一次 execution 总额，retained/delayed callback 不继承父执行已消费预算。可复用已有探针框架，避免仅测试字段赋值。

不要求为此次小 API 另建一套框架或大量重复单元测试。

### 3. 修正文档的时间保证与零值语义

- `engine.rs:2052-2055` 的“默认 1M keeps ... inside one frame budget”不成立。操作数并非时间；硬件、操作种类、Host native function 都会改变耗时。slow threshold 是事后诊断，不会抢占正在执行的 Rhai 或阻断 Rust native 工作。
- 本设置适用于该 Engine/View 的所有相应执行回合，**不是仅给某个一次性布局阶段提额**。应直说提高它也会扩大事件 callback、effect 等的可用额度；78 ms 的一次性前台计算是否可接受由 Host 产品体验决定。
- `set_operation_limit(0)` 当前被 `max(1)` 静默归一为 1，和 Rhai 原生 `set_max_operations(0)` 的“无限制”相反。可以保留当前策略，但必须文档明确并测试；不必为了这个边界重构 public API。
- builder 注释应写“未调用此方法时默认 1M”，避免向只接受 `u64` 的公开方法描述内部 `Option::None` 参数。

### 4. 修 fmt，基于合入时主干重跑项目要求的检查

主审查已确认 CI 当前失败在 Format（新增 engine 测试排版），后续检查尚未运行。这不是测试已证明产品失败，也不是仅修 fmt 就已经证明其余门槛通过。详见上一级 `evidence/pr-96-ci-failed.log`。本组不重复拉取 CI，也未向 GitHub 写评论或修改 PR。

## 独立验证与边界

- [源码事实来源](../evidence/pr-96.diff)、[PR 元数据](../evidence/pr-96.json)。函数/API 语义另外核对本机锁定 Rhai 1.26.0 的 `src/api/limits.rs`、`src/parser.rs`，以及基线 execution-session 入口。
- [小型基线探针](probes.rs) / [运行器](run-probes.py) / [输出](probes.log)：运行器通过 `git archive f6e936a5` 创建固定、隔离的产品源码快照，复用已下载依赖与编译缓存。测试的目的仅为刻画当前 1M 计数及 parser 深度与 runtime operations 的区别，**不是 PR #96 的完整构建或通过声明**。
- 最终固定快照探针 **2/2 通过**：PR 使用的 60,000 次循环仅消耗 **180,009 operations**；400,000 次循环在 **1,000,001 operations** 被拒绝。4 层表达式括号可编译，64 层被 parser 拒绝，两次 compile 的 runtime operations 均为 0。这直接说明当前新增测试未覆盖真正跨越 1M 的提额收益，也说明 operation budget 不能替代表达式解析限额。
- 首次尝试引用了后来被另一会话修改的 live workspace，且初选 250,000 次循环本身未超限。该次输出已另存 `live-worktree-discarded.log`，不用于固定基线或 PR 验收；最终探针改用快照和更明确的 400,000 次有限循环。
- PR 内容没有 checkout 到用户工作区，没有覆盖其他会话的修改。

最终选择：**把 PR #96 收窄为 Host operation budget，作者小改并通过 CI 后纳入本轮；表达式深度独立给出证据再定，不需要主开发接管全面重写。**
