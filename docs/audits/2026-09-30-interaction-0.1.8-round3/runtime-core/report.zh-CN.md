# 第三轮核心提交边界复核

基线：`815bb82b8103c88ef6ba50620128780c9d64b216`，2026-09-30。主审接手核对已执行探针和产品源码后整理本报告；原并行分支在整理阶段被平台自动安全检查中断，未完成的猜测不计为确认问题。

## R3-CORE-1 · P1：同批后续消息失败，丢失前项已经成功提交的手势失效标记

上一轮的单个 timer 修改 axes 已修好。但异步 batch 中每条 delivery 是独立事务，整个 batch 的返回错误不能抹掉前面已经提交的事实。

原生最小反例：

1. Draggable 从 `{20,20}, axes=horizontal` 开始向右拖40。
2. 同一截止时间的两个声明式 timer 排入同一批。
3. 第一条将 axes 改成 vertical 并成功提交、重渲染；第二条仅 throw 一个独立错误。
4. AdvanceTime 正确返回第二条错误，画面与 Runtime 状态也已经是 `vertical:none`。
5. 松手却提交旧横向结果，最后显示 `vertical:60.0,20.0`。

单个成功 timer 的同构对照保持 `vertical:none`，不会提交旧手势。

定位：`app.rs:4999–5032`。`deliver_async_batch` 正确返回了 `delivery_contract_changed`；之后 `(Ok(_), Some(error)) => Err(error)` 丢弃整个 Ok 内容，只有最终 Ok 分支才 `mark_interaction_contract_changed`。第二条的失败只能回滚第二条事务，却使第一条已经生效的交互契约失去取消记录。

整改：每个成功提交的失效记录应与该提交共同生效，不依赖整批是否全成功。可汇总通知和首个错误，但不能汇总掉先前已提交的 epoch/取消事实。不要回滚整个 batch 来掩盖问题，那会改变现有独立事务与外部副作用语义。

验收包含：成功→失败、失败→成功、多次成功夹失败；source/constraint 改变与无关更新；后续 mouseup 不得按过期契约提交，正常可用的最后成功状态保留。

证据：[reviewed-probes.rs](reviewed-probes.rs)、[最终原生日志](reviewed-probes.log)，测试 `failed_sibling_delivery_preserves_successful_invalidation`。

## 本轮通过的生命周期对照

最终四项原生测试为 **3 passed / 1 failed**：

| 用例 | 结果 |
|---|---|
| 单成功 timer 修改 axes | PASS，无旧手势提交 |
| 成功 timer 后追加失败 delivery | FAIL，确认上述问题 |
| 正常 PanZoom suspend→resume→pan | PASS，`x=50, commits=1` |
| 后置 native suspend hook 注入失败，补偿、重试、恢复再 pan | PASS，错误先被准确返回且 View 保持 Active，重试后正常提交 |

失败补偿测试中的 `injected late primitive suspend failure` 是有意注入的可控错误，被产品正常处理，不作为新的 panic finding。

首次脚手架根据 View state 条件式卸下内容，却只刷新窗口而未通知 Host 重新组合，导致正常恢复后的输入对照失败。主审明确通知 Host 后重新执行，断言没有改变，正常与补偿两组均通过。因此原始 [probes.log](probes.log) 中的该失败已排除；只使用 [reviewed-probes.log](reviewed-probes.log) 作为最终计数。这不是在产品代码中加 workaround。

新增 direct read/write lease 对已验证的 PanZoom suspend、resume、补偿路径有效，不能因为仍有 CORE-1 就认定旧重入修复无效。

## 未升级为确认缺陷的源码关注点

motion callback、viewport 等其他 render_dirty 入口也应进入同一提交失效政策，但本轮没有对每个入口都构造新的独立反例，未按源码相似性重复计数。建议用提交记录本身维护这条不变量，而不是持续在各 handler 末尾附加条件。

## 重现

```sh
python3 docs/audits/2026-09-30-interaction-0.1.8-round3/runtime-core/run-reviewed.py
```

该 runner 使用唯一 `runtime_core_round3_reviewed` target、自己的 TemporaryDirectory 和共享依赖缓存。`resume-control.rs` / `resume-control.log` 保留了主审单独验证脚手架修正的过程。未修改产品、正式测试或前两轮审计。
