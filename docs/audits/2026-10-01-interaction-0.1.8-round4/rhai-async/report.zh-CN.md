# 第四轮：异步 effect activation 交付验收

基线 `f6e936a5`，2026-10-01。范围限定为 effect activation 在交付瞬间的有效性、正常 producer 收尾、依赖替换和暂停失败补偿，以及 callback owner 错误边界。Rhai 仍为锁定的 `1.26.0`；按 rhai-api 的版本/源码核验规则工作。在线 schema validator 由主审查处理，本组没有重复压力测试。

## 结论

上一轮“同批旧 effect 消息在 cleanup 后污染新查询状态”的核心修复已通过正向验收。正常 producer 返回、同 activation 重绘、失败替换/暂停的补偿也没有发生有效消息误丢。

剩余 **1 个 P2**：组件正常卸载后，一条已经从真实订阅队列 drain 出来的旧 effect 消息会先触发 callback owner 错误，无法走到新增的旧 activation 丢弃分支。它不会执行旧脚本或污染新组件，但会把正常的取消结果变成应用错误。

最终本组 5 个有界本地测试完成：4 个正向测试，加 1 个缺陷特征测试（含同步旧 callback 应报错的对照）。未修改产品、正式测试、旧报告或其他会话临时目录；没有外部利用、压力耗尽或 GitHub 写操作。

## P2：已失效 effect 消息在组件卸载后被误报为 callback 错误

定位：`crates/gpui-rhai/src/lifecycle.rs:785-793`。

`invoke_async_delivery` 先调用 `validate_callback_owner`，之后才检查 `EffectRegistry::contains_scope`。同组件替换 effect 时 incarnation 仍然有效，因此新过滤生效；但组件一并卸载时，旧 callback incarnation 先被判定无效并返回错误，已经取消的 effect delivery 来不及被正常丢弃。

独立复现全过程：

1. 正常挂载一个声明 `watch` effect 的 formal component；该 effect 通过真实 `ctx.start_subscription` 创建 producer。
2. producer 发出一条正常字符串，测试从真实 `runtime.subscriptions.drain(generation)` 取出 `AsyncDelivery`，保留其原始 callback、scope 与 activation，未伪造任何身份字段。
3. 调用根组件正常事件，将该子组件从视图移除，再执行 `render_dirty`。
4. 确认子组件状态已移除，旧 producer 的 `close_reason()` 为 `Some(ScopeDisposed)`；此时旧 activation 的生命周期已结束。
5. 交付步骤 2 已取出的消息，实际返回：`callback received belongs to an unmounted incarnation of component /App[root]/StreamProbe[stream]`。

这是应用现有 batch 边界可以到达的状态：同批较早的 callback 可以让组件卸载，而后续消息已经取到局部 Vec，取消旧 effect 时无法再从 runtime pending 队列中清除它。

**已直接实测的结论**是 lifecycle 对真实、已取消 scope 的 delivery 返回 `StaleComponentCallback`；没有直接启动 GPUI 窗口测量原生错误横幅。**源码推导的应用影响**是 `app.rs:5093-5095` 将该错误转为 `ScriptFailure`，`:5107-5115` 保存为 delivery error，`poll_async` 的错误分支再调用 `set_failure`。因此正常卸载会被记录为 view failure，而非一次无声的旧消息丢弃。

整改边界：在异步 delivery 路径先确认 effect activation 的交付资格；已经取消/替换的 activation 应直接丢弃，再对仍有资格的 delivery 验证 callback owner/generation。不要全局吞掉 stale callback 错误，也不要放宽 callback incarnation 验证。

正对照已同时运行：对同一个已卸载 callback，**同步显式调用** `invoke_callback_transactional` 仍正确返回 stale owner 错误；这项防护应保留。修复应只区分“已取消异步消息”与“主动调用无效 callback”，不能把二者一起变成成功。

建议回归断言：

- 同批前项卸载组件，后项旧 effect success/error delivery 无副作用、无 view failure。
- 同 path remount 后，旧 incarnation/activation 消息无声丢弃，新 activation 消息照常交付。
- 仍有效 activation 上出现真实无效 callback identity 时继续报错；显式同步旧 callback 继续报错。
- 正常 producer close / WorkReturned 的已接收消息不能因 registry entry 已移除而丢失。

## 已通过的正向边界

| 场景 | 实际结果 |
|---|---|
| 同批第一条消息改变 effect 依赖；后两条是旧 success 和旧 error | 旧 producer 为 ScopeDisposed；两条旧消息均不修改 `new-query` 状态，新 activation 消息正常更新状态 |
| Producer 正常从 SubscriptionWork 返回，registry 在 drain 后为空 | effect 仍有效，close 前接受的消息仍正常交付 |
| 普通消息触发重绘，但 effect 依赖没变 | 不产生 cleanup，后续同 activation 消息仍有效 |
| 新 activation 的 subscription startup 失败 | 事务恢复旧 activation；旧 producer 未被错误关闭，已取出的旧消息仍可交付 |
| suspend cleanup 失败 | 生命周期补偿保留旧 activation，旧消息仍有效 |
| suspend 成功 | 旧 producer ScopeDisposed，已取出的旧消息无声丢弃 |
| resume 成功 | 重新创建不同 activation，新的消息照常交付 |

这些探针通过真实 SubscriptionCapabilityHandler / SubscriptionWork / ScriptLifecycle 路径运行。Producer 循环有 10 秒兜底，正常测试在约 1 秒内完成并显式 dispose；没有长时间占用后台线程。

## 材料

- [探针源码](probes.rs)
- [独立运行器](run-probes.py)：临时 workspace 前缀 `gpui-rhai-r4-async-`，唯一 test target `audit_r4_rhai_async`，复用既有 dependency cache；不修改历史测试。
- [运行输出](probes.log)

执行：`python3 docs/audits/2026-10-01-interaction-0.1.8-round4/rhai-async/run-probes.py`。

探针中的 unmount 测试用现状断言记录已确认缺陷，因此“5/5 通过”不表示该 P2 已修复；前四项才是当前修复的正向验收。
