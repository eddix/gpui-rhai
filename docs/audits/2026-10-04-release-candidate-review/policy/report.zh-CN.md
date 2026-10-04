# R8 · D1 Host execution policy 验收

基线：`736025a8b2b1ac44d41d893bc9a929c990b97532`（PR #108 候选），2026-10-04。对照 `cda11ce7`，范围为 Host operation/parser policy、执行回合统计、retained callback、配置传播与失败诊断。按 rhai-api 要求核对实际 Rhai `1.26.0` 源码。本组不评审 disktree。

**结论：本组范围通过，未确认新的 P0/P1/P2。** 当前实现完成了旧 #96 评审要求的收窄与边界修正；没有把原 PR 的全局 function depth 128 直接作为所有应用的新默认。

## 1. 配置与传播

- `engine.rs:551` 默认 quota 为 `DEFAULT_SCRIPT_OPERATION_LIMIT = 1_000_000`；`configure_engine_limits` 继续显式设置 global/function parser depth 为 `64/32`，不随 Rhai 的 debug/release 默认差异漂移。
- `set_operation_limit` 与 `set_expression_depth_limits` 分开，均将零规范为 1；没有 Rhai 脚本提额 API，没有改变调用层数、Array/Map/String 限制。
- `app.rs:1497-1511` 的私有 policy applier 被 File prepare、Embedded prepare、secondary factory 共用；都在 trusted extension 之前应用。该优先级已经在 builder 文档和 performance contract 中明确。
- `engine.rs:622-628` 的 reload candidate 从新 Engine 创建独立 Cell/Tracker，再复制当前有效 quota 和 parser depth；`app.rs:5737-5742` 再运行可信 extension 配置。失败候选不会反向改写正在运行的 Engine 配置。
- Parser 设置只约束后续编译；operation 设置约束后续执行回合。操作数不是帧时间或 native Rust 抢占保证，当前文档已经修正旧表述。

Rhai 原生 `set_max_operations(0)` 仍用于关闭 evaluator-local 的绝对计数，由项目的累计 progress adapter 执行 Host quota。这与 Host setter 的零值语义不同，代码与文档均保持明确。

## 2. 共享回合与 retained context

`OperationTracker` 原有 begin/observe/align 规则保留；新 policy 只改变比较的额度值，没有为每个 nested evaluator 重置累计消费。普通执行入口 begin 新回合，incremental sibling/virtual item 的 align 只改绝对基线。

`invoke_callback_for_generation` 从真实 stored NativeCallContext 取 operation base，再 begin delayed round。`invocation.rs` 仍只对 retained capture 清零已返回的调用深度，没有把真实同步递归改成无限制。

除了复跑已有 root policy fixture，本组新增真实 `ctx.start_task → retained completion → ctx.start_task` 链，避免仅用 `engine.callback` 创建无 stored context 的 FnPtr 来代表全部延迟路径：

- 200 跳全部完成；每回合限额 10,000，累计 **1,207,787 operations**，没有把先前回合消费计入下一跳。
- Host 后续将限额降为 100，同一真实 retained callback 在 101 operations 被终止，事务状态保持在 hops=200。
- Host 再把额度改为 500,000，已记录失败诊断仍报告当次 `maximum=100 / consumed=101`，并保留命名来源 `policy/task-chain.rhai`。

## 3. 诊断和 parser 独立性

`ExecutionTiming.operations` 仍表示局部 timing span；新增 `round_operations` 和执行起点的 `operation_limit` 用于诊断，避免第二个 incremental component 的局部量较小却掩盖累计超额。

独立 parser fixture 在 operation quota 17 下因默认 function depth 32 拒绝深表达式，compile timing 的 runtime operations 为 0；只将 parser depth 改为 64 即可编译同一源码，再以正常 quota 成功渲染。调用层数与数据限额保持 `(64, 10_000, 100_000, 1_048_576)`。

命名 compile timing 保留 `policy/deep-view.rhai`，通过公开 `DiagnosticContext` 构造后的 ScriptCompile 诊断保留 source 与位置。本组未将它称为 CLI 彩色输出或初始 File prepare 的完整原生错误界面测试。

## 4. 新鲜执行结果与既有材料

| 本组当前 SHA 实际执行 | 结果 |
|---|---|
| `cargo test -p gpui-rhai --test host_execution_policy --locked --offline -- --nocapture` | **6/6 PASS** |
| 两项独立 public API probes | **2/2 PASS** |

正式有限工作量对照实测：默认在 **1,000,001** 拒绝；提高后消耗 **1,350,012** 并返回 `450000`；降低后在 **500,001** 拒绝。这里的 debug 耗时不用于 release 性能或帧率结论。

六项正式 fixture 还覆盖了非默认额度下 nested/sibling 和 dirty-sibling 累计、retained parent-history、不带 stored context 的 delayed 回合、restricted import、failed reload 的 last-good callback、parser 32/48 拒绝而 64 通过、File/Embedded builder、zero 和 extension precedence。

已完整读取 `tests/native-keyboard/tests/host_execution_policy.rs` 的真实 File/Embedded mount、secondary engine、parser mount 三项，主审负责当前完整 native suite，本组没有重复运行并冒称自己的结果。另已读先前独立 review 的 debug/release/candidate 证据；核实 `be7a8eb8 → 736025a8` 的 engine/app/invocation/diagnostic/source 及两份 policy fixture 字节一致，见 [source-metadata.json](source-metadata.json)。先前 release/native/候选存储测试属于已有材料，不计入本组新鲜执行数量。

当前没有需要额外引入通用 execution-policy 框架、脚本 `with_budget` 或提高全局默认的证据。Rust 常量重命名和 ExecutionTiming 新字段属于约定的 pre-1.0 接口调整，本组没有列为兼容性缺陷。

## 证据

- [正式 policy 输出](product-policy.log)
- [新增探针](probes.rs) / [运行器](run-probes.py) / [输出](probes.log)
- [源文件哈希与执行归属](source-metadata.json)

新增探针复用现有 native dependency cache，运行前后校验 HEAD 和产品 diff 未变化；没有修改产品、正式测试、旧报告、依赖或 profile，也未执行 GitHub 写操作。本结论仅对应 D1，不替代整版候选的其它验收门槛。
