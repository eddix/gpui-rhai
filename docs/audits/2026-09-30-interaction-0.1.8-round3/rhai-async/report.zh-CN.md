# 第三轮异步交付与有界诊断复核

基线：`815bb82b8103c88ef6ba50620128780c9d64b216`，Rhai `=1.26.0`。日期：2026-09-30。主审核对已执行日志和源码后收口；并行分支整理阶段被平台自动安全检查中断，不据未完成推测形成结论。

## R3-ASYNC-1 · P1：从队列取出的旧 effect 消息，在 activation 清理后仍可执行

现有 generation／component incarnation 校验不能代替 effect activation 校验。同一正式组件仍然存活时，一个 effect 可以因依赖变化被清理并重新启动。

独立探针使用真实 SubscriptionRegistry、正式组件和 ScriptLifecycle：

1. effect `watch` 的 activation=1 产生 `replace` 和 `old-query-result` 两条消息；一次 drain 把二者取成 Vec。
2. 第一条 callback 修改依赖 epoch，随后真实 render_dirty 执行旧 effect cleanup，并启动 epoch=1 的新工作。
3. 已确认旧 emitter 的 close reason 为 `ScopeDisposed`，cleanup 恰好一次，组件当前 phase=`new-query`。
4. 调用同一公开 delivery 入口处理此前取出的第二条，结果仍 `Ok(())`，phase 被改回 `old-query-result`。

这是正常批量投递在单线程上即可发生的顺序问题，不依赖线程竞态。它会把已经关闭的旧查询、旧订阅或旧 effect 结果应用到同一组件的新状态。

定位：

- `app.rs:4960–4994` 在执行 callback 前取出整批消息；`:5048–5084` 逐条独立执行与重渲染。
- `context.rs:537–545` 的 `cancel_async_scope` 清 registry 和 Runtime 的 pending_async，却接触不到已取出的本地 Vec。
- `lifecycle.rs:780–808` 的 `invoke_async_delivery` 只校验 callback owner，再把旧 `delivery.scope` 填进 context，没有确认该 effect activation 仍是当前有效所有者。

证据输出：

```text
OLD_SCOPE Effect { ..., key: "watch", activation: 1 }
AFTER_CLEANUP old_closed=Some(ScopeDisposed) cleanup_calls=1
callback_result=Ok(()) phase=Some(String("old-query-result"))
```

整改：每条消息在真正执行前验证其 committed activation/作用域有效性；一条前序回调触发 cleanup 后，本地批次里的后续旧消息也应失效。不要只检查 producer 是否仍在线或 subscription entry 是否仍在 map：正常 producer close 后，已接受的缓冲消息仍有合法交付语义。区分工作源的结束、作用域的撤销、脚本 generation 与组件挂载身份。

验收：同组件旧 effect→新 effect、两条同批消息、前项失败回滚不错误撤销仍有效作用域、组件卸载、正常 producer close 继续 flush，以及取消不转成旧作用域 error callback。不要对已经可能产生副作用的 callback 做自动重试。

本项是本轮扩大检查发现的现存生命周期缺口，不声称由“配额前置”补丁新引入，也不重开 #86 的已修尺寸校验根因。

## R3-ASYNC-2 · P2：通过资源限额的值，仍可在 schema 拒绝路径构造完整大诊断

配额前置确实修好了上一轮900k项数组先跑schema的问题。新对照中10,001/900,000项均直接变为 `async_delivery_limit`，没有构造逐元素schema错误。

剩余边界在 `async_runtime.rs:1199–1203`：对通过配额的值，仍完整调用 `validate_ui_value`，收集全部错误、`to_string()` 后才截断。合法大小的100,000项 Map，值为Null、schema要求Integer，实际产生100,000个 SchemaIssue 和 **4,199,998 bytes** 中间诊断，最终只保留约1KB。debug drain约163ms，只作为成本定位，未当作release性能保证。

因此“资源检查前置”和“在线错误诊断有界”是两个条件；修前者不能代替后者。建议在异步交付的 schema 检查中限制错误数／格式化字节，或使用 fail-fast 结果；离线详尽校验仍可保留完整错误列表。

## 正向结果与验证边界

当前 [probes.log](probes.log) 记录4个特征探针均执行完成：其中旧activation和诊断成本测试断言的是当前不正确行为／成本，不能把 `4 passed` 解释为产品通过。

- 配额先于schema，超限数组被正确早期拒绝。
- UiValue自己的总字节限制和有限数规则仍生效；Rhai不统计Map key字符串长度的规则没有被错误替代。
- 旧effect消息场景记录上述状态覆盖事实。
- 配额内错误Map记录上述完整schema诊断成本。

探针使用小型、有界的本地数据与确定性cleanup；没有执行内存耗尽或外部系统操作。数据大小测试与默认AST路径不能代替全平台／其他backend认证。

源码：[probes.rs](probes.rs)，运行器：[run-probes.py](run-probes.py)。运行器复用既有native依赖图和缓存，未修改产品、正式测试或旧审计。上轮6项交付边界回归由主审统一执行并保存在本轮baseline目录。
