# 0.1.8 第二轮：Rhai / async / lifecycle signal 验收

基线：`d77b4b49`，2026-09-30。Rhai `=1.26.0`，`internals` / `metadata` / `serde`；本组运行默认 AST 路径，未启用 Grain。继续按 rhai-api skill 核对锁定的 Rhai 源码。没有修改产品、旧审计或正式测试。

## 结论

- **#86 的交付正确性修复通过本组正向验收**：不可消费的返回值在进入 success callback 前转为一次可读错误；超大 Host error 和 schema error 都安全截断；没有 callback catch-and-retry。
- **#80 未见回退**：修改未触碰 retained-context adapter；本轮再次验证真正同步递归仍被限制。上一轮真实 200-hop chain 与分回合 operation budget 结果仍是相关基线，本轮没有重复声称运行过它。
- 新发现 **一个 P2 性能问题**：新增限额预检放在完整 schema 验证之后，超限结果仍可在前台创建数十万诊断并格式化巨大错误文本，最终才截断。
- direct signal cancellation lane 保留原 SignalRegistry 的原子/类型/stale 校验；本组源码审查未发现新安全绕过。没有把这条结论扩大为所有 native interaction 场景已由本组测试。

## P2：将异步资源预检前置，避免在前台为必拒绝结果构建海量 schema 错误

定位：`crates/gpui-rhai/src/async_runtime.rs:1199-1203`。

当前顺序是 `output.validate_ui_value(value)` → `error.to_string()` → 有界错误封装，只有 schema 全部通过后才执行 `validate_rhai_delivery(value)`。而 `ValueSchema::validate_ui_value` 在 `schema.rs:282-286` 会完整遍历 UiValue、clone/转成 Dynamic，并由 `:267-273` 收集全部 schema 错误；`SchemaValidationError::Display` 在 `:833-840` 再格式化全部错误。

**具体触发：** Host capability 或 subscription 返回 `Array<Null>`，长度 900,000，output schema 声明 `Array<Integer>`。它尚未超出 UiValue 的 1,000,000 总项上限，但远超过 Rhai 的 10,000 数组项上限，第一步就可以判定不可交付。实际却先生成 **900,000 个 SchemaIssue** 和 **33,188,888 bytes 的完整错误文本**，然后才截成最多 1,027 bytes。

同一个值即使 schema 完全匹配，也会先完整遍历、复制并转换后才拒绝。这些工作在 `app.rs:4922-4923` 的前台 drain 路径上执行，不受 Rhai operation hook 中断。

独立探针本机 debug profile 的单条 drain 耗时：

| 数组项数 | schema 合法但超 Rhai quota | 每一项均 schema 不合法 |
|---|---:|---:|
| 10,001 | 2.014 ms | 4.893 ms |
| 100,000 | 20.249 ms | 48.886 ms |
| 900,000 | 219.119 ms | 482.897 ms |

这些是定位成本来源的 debug 观测，**不是 release 性能基准或线上延迟承诺**。更硬的证据是实际构造的 900,000 个错误与 33 MB 中间文本，以及“配额检查位于全部工作之后”的控制流；不需要依赖特定机器阈值判断问题。

建议：

1. 先在 `UiValue` 上执行不分配 Dynamic 的资源预检；超限直接构造 `async_delivery_limit`，再对通过预检的值执行域/schema 检查。避免改变配额或放宽限额来掩盖问题。
2. 异步边界的 schema diagnostic 使用数量/字节上限或 fail-fast 投影；至少不要对明知只会保留约 1 KB 的错误先 `to_string()` 全量诊断。完整离线 schema 检查仍可保留详尽诊断。
3. 新验收对超大结果应检查在 schema 全量遍历前拒绝；不要仅断言最终 error message 很短。再以受控 release 测量检查前台耗时，而不是照搬这里的 debug 毫秒数作为门槛。

本项是资源拒绝路径的效率缺口，**不是 #86 的 loading 状态和 callback 路由仍未修复**，也不等同于宣称 Rust Host 内存分配已被沙箱化。

## 正向验证矩阵

六个独立测试最终 **6/6 通过**，包含：

- 顶层字符串精确 1,048,576 bytes 成功，+1 byte 转 error。
- 两个字符串的累计值恰好在限额成功，合计 1,200,000 bytes 转 error。
- 数组总项恰好 10,000 成功，10,001 转 error；嵌套数组正确包含外层元素计数。
- Map 恰好 100,000 项成功，100,001 转 error，并使用实际 `Engine::ensure_data_size_within_limits` 对照。
- Map key 不计入 Rhai string quota 与上游一致；一个 1,100,000-byte key 不应被错误算入字符串 value 预算。UiValue 自身另有包含 key 的 16 MiB 域限额。
- 结构深度 64 成功、65 返回可读错误；更早的 UiValue 域验证已在深度 65 拦截，所以当前该错误是 `async_error`，没有漏到无限深 Dynamic 转换。
- 1.5 MB UTF-8 Host error 和超大 enum schema error 被正确截断，错误 callback 可读取；没有破坏 UTF-8。
- 每个被拒绝任务仅 error callback 执行一次，success callback 中的 Host marker 副作用为 0；task entry 被消费后不再次投递。
- Success callback 自己在 Host 副作用后 `throw` 时，仍走原事务失败，不追加 error callback，也不重试 success。这与执行前校验失败有明确区别。
- subscription 缓冲 `[正常、超限、超长错误、正常]` 后 producer `close()`，按 `[success, failure, failure, success]` 顺序各交付一次；close 后拒绝新 emit。
- task 和 subscription 的显式取消、旧 generation 均丢弃消息，不因新增“转 error”逻辑重新投递已经失效的工作。
- 真同步递归仍报 Stack overflow。

取消探针覆盖的是“未 drain 前取消”；本组没有声称改变或重新认证已经取出 delivery 后所有复杂 scope teardown 竞态。

## direct signal cancellation lane 的源码核验

- `app.rs:4425-4437` 仅在 Host quiesce / cancel 的同步区间打开 lane，`DirectSignalWriteGuard::drop` 随退出复位。
- `app.rs:3329-3334` 直接访问当前 Runtime 的 SignalRegistry，仍调用 `write_batch_from(..., SignalWriter::Primitive)`；不会进入 Rhai callback，也不会重置脚本 quota 或 retained context。
- `signal.rs:438` 的全批验证仍覆盖 stale signal、重复 signal、类型与有限数值；全部验证成功才写入，保持原子性。
- `renderer.rs:186` 在调用 signal writer 前过滤空 batch，新的错误处理中的 `first().expect(...)` 不会因普通空批而被触发。
- suspend 中脚本事务先成功完成，再由 quiesce 同步清除原生 preview；脚本 suspend 失败且 native compensation 成功时不提前清理 active interaction。

这条 lane 的作用是避开同一个 ScriptHostView Entity 的重入借用。原生组件的 owner/retained identity、受控值更替后的 cancel 行为由其他交互审查探针覆盖；本组没有为它另外启动 native window。

## 材料

- [探针源码](probes.rs)：当前修复的正向断言和新增成本特征探针，未复用旧缺陷断言来制造测试失败。
- [运行器](run-probes.py)：临时 workspace，复制现有 manifest/lock，`--locked --offline`，不改正式测试。
- [完整结果](probes.log)

运行：`python3 docs/audits/2026-09-30-interaction-0.1.8-round2/rhai-async/run-probes.py`。
