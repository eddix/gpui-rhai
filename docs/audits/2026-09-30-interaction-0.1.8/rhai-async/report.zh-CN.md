# 0.1.8 Rhai / async / capability 对抗审查

- 审查基线：`0b9b887c`（0.1.8 release candidate），2026-09-30。
- 范围：`invocation.rs` retained context、同步递归与操作预算、task/subscription 输出与错误投递、相关 capability API。
- 依赖核验：Rhai `=1.26.0`，workspace features 为 `internals`、`metadata`、`serde`，没有 `sync`；本组独立探针运行在默认 AST backend，复用 native-keyboard 的依赖图，没有启用 Grain。
- 方式：按 rhai-api skill 读取实际锁定源码，GitHub issue 只读拉取，独立临时 workspace 探针；没有修改产品或正式测试，也没有提交或向 issue 发消息。

## 结论

`#80` 修复通过本组验收。`#86` 在当前版本仍可复现，并且影响范围比原 issue 的顶层字符串更大，建议作为一个 P1 根因组纳入本轮出货前整改。`#91` 应进入后续独立设计，不在 0.1.8 临时增加基于时间或任意窗口点击的权限判断。

## P1：异步交付缺少脚本可接收性校验，连错误回调本身也可能无法读取错误

相关 issue：[86](https://github.com/eddix/gpui-rhai/issues/86)。这是既有缺陷在当前 release candidate 上仍存在；不是因 0.1.8 API breaking change 而列出的兼容性问题。

定位：

- `crates/gpui-rhai/src/async_runtime.rs:493`：`task_delivery` 只验证输出 schema，直接选 success callback。
- `crates/gpui-rhai/src/async_runtime.rs:1166`：`subscription_delivery` 使用相同策略。
- `crates/gpui-rhai/src/async_runtime.rs:1191`：`error_payload` 将任意长度 Host 错误原样放入 `message`。
- `crates/gpui-rhai/src/lifecycle.rs:786`：payload 直接转成 `Dynamic` 并进入 callback；失败回滚 UI 状态，不能撤销此前的 Host 外部副作用。
- `crates/gpui-rhai/src/engine.rs:2252`：字符串预算 1,048,576 bytes、数组预算 10,000、Map 预算 100,000。

独立实测：

| 输入 | 结果 |
|---|---|
| Task 成功返回 1,000,000 字符 | success 正常读取；状态 `loaded` |
| Task 成功返回 1,100,000 字符 | 读取时 `Length of string too large`；状态回滚至 `loading` |
| Task 返回短 `Err(String)` | error 正常读取；状态 `failed` |
| Task 返回 1,100,000 字符的 `Err(String)` | error 读取自身参数时失败；状态仍为 `loading` |
| Task 成功返回 10,001 项数组 | `Size of array/BLOB too large`；状态仍为 `loading` |
| Task 返回两个各 600,000 字符的字符串组成的数组 | 每个字符串单独低于限制，但总体仍被拒绝；状态仍为 `loading` |
| Subscription 成功消息或 `emit_error` 的 message 超限 | 相同不可读取问题 |

订阅探针直接通过真实 `SubscriptionRegistry` 注册/emit/drain，并复用真实 task 创建路径捕获的 callback context；它没有启动持续 subscription producer 线程，这不影响本项对 delivery 分支的覆盖。

**原 issue 的“回调任何代码都不会执行”不是准确的一般结论。** 对抗探针进一步证明：如果回调忽略超大参数，则它可以成功执行；如果先调用 Host capability，再读取超大参数，则 Host 调用已经执行一次，随后 Rhai 报错，UI 状态回滚为 `loading`。因此不能在 callback 失败后简单转调 error callback，更不能自动重试整个 success callback。正确修复边界应在选定并执行回调之前。

底层证据：Rhai 1.26.0 `src/eval/data_check.rs` 的 `calc_data_sizes` / `throw_on_size` 对整个值递归统计字符串字节、数组元素、Map 项。不能只校验顶层 String 或逐个字符串；需要按匹配当前 Rhai 规则的累计量校验。payload 本身虽然通过 schema，仍可能不是脚本能合法消费的值。

建议整改：

1. 用同一份限额配置驱动 Engine 配置与异步边界校验，避免两套硬编码漂移；在 `UiValue -> Dynamic` 和执行 callback 前完成校验。
2. task、subscription、schema failure、Host error 都走同一入口；不可接收的 success 结果转为一次有界 error delivery，保证错误对象自身可读。
3. 错误对象包含稳定 kind、违反的维度、实际量和限额；message 按 UTF-8 边界有界处理，不要把超大原始 payload 再复制进诊断字段。
4. 只路由**执行前**的交付校验失败。脚本自身执行期间抛错、预算耗尽或 Host 调用失败保留原有事务错误语义，不任意二次执行回调。
5. 验收覆盖表中变体及恰好等于限额的边界、嵌套 Map/Array、普通业务 schema error、取消/旧 generation 不投递。原始失败 payload 不应以重试队列形式长期留存。

本项并不声称上述校验能约束 Host 分配内存：Host 构造巨大字符串发生在校验之前，队列容量目前以消息数计。需要严格内存约束的 Host 仍应在生产端限制输出/流量；可另行规划按字节背压，不把它作为本轮未证实的新漏洞。

## #80：修复成立，同步保护仍有效

Issue：[80](https://github.com/eddix/gpui-rhai/issues/80)。`invocation.rs:21` 和 `:42` 仅对保存供以后重入的上下文将 `stored.global.level` 设为 0；`operation_base` 仍保留以供进度统计对齐。初次组件调用在 `engine.rs:2470` 仍使用当前原生 call context 的 `call_within_context`，没有把同步组件嵌套改成新栈。

本组没有仅依赖新增的 adapter 单元测试，而是运行真实 `ctx.start_task -> completion -> ctx.start_task`：

- 200 跳全部成功；每跳执行 2,000 次算术循环，累计操作明显超过单次 1,000,000 预算，独立回调没有继承已消费配额。
- 同一个 retained callback 内递归调用自己，仍报 `Stack overflow`。
- 同一个 retained callback 内无限循环，仍被 progress hook 终止为 `Script terminated`。
- 两种真正失败都保持 UI 状态回滚。

建议：保留本次修复并在实际合并后关闭 #80；不要为了修 #86 重新放开递归/操作限额。Rhai 升级时继续把 `NativeCallContextStore` 视为受版本约束的 internals 适配器。上述结果不是对所有导入模块/Grain 路径重新认证的声明。

## #91：后续专门定义 origin 与 user activation

Issue：[91](https://github.com/eddix/gpui-rhai/issues/91)。`capability.rs:77` 的当前 Handler 只收到 method 与 input，确实没有调用来源；这是 API 能力缺口，不能据此声称当前已有的 user-activation 安全承诺被绕过，因为当前并未提供该承诺。

建议独立规划，原因：

- `InvocationOrigin` 是“由什么路径触发”的诊断/策略事实；“是否拥有可消费的用户激活权限”是另一条安全契约，两者不能直接等同。
- 真实输入、Automation、AX action、Timer、TaskCompletion、Subscription、Effect 与同步 helper 调用需要统一来源传播；延迟回调不能继承旧用户激活。
- 来源必须由 Host/runtime 派发路径提供，脚本不可自行伪造；需要 view 归属、嵌套还原及明确的作用域/过期/消费规则。
- 不采用窗口级最近 1.5 秒点击、全局布尔标志或 click 任意控件解锁所有视图的临时做法。

0.1.8 中保持当前 capability manifest/schema 授权语义，后续对 #91 单独设计与验收。已有应用可在 Host 继续使用自身明确允许的能力边界；不应因这个 issue 自动放开任何权限。

## 验证材料

- [探针源码](probes.rs)
- [运行器](run-probes.py)：复制既有 native-test manifest/lock 到临时目录，`--locked --offline`，复用 target；不会改正式测试。
- [完整输出](probes.log)：最终 **5/5 通过**。这些是现状特征断言，超限投递测试的“通过”表示缺陷被稳定复现，并不表示产品修复已通过。

复现：在仓库根目录运行 `python3 docs/audits/2026-09-30-interaction-0.1.8/rhai-async/run-probes.py`。
