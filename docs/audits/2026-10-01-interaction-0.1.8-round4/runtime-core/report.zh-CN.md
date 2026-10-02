# 第四轮核心交互所有权验收

日期：2026-10-01。基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`，对比 `815bb82b`。依据本轮 ADR 0022 的实际契约，不要求已移除的 awaiting-control 设计。

## R4-CORE-1 · P2：PanZoom debounce 完成后没有释放 Escape 所有权

**确认的新发现，原生公开 API 探针可复现。** 新增的 Host/window Escape interceptor 能取消 wheel，但 precise wheel 的 debounce 完成路径没有同步注销辅助会话；wheel 已经结束之后，第一次 Escape 仍被它消费。

最小触发：

1. 挂载公开 `PanZoom`，受控 transform 为 `{x:0,y:0,scale:1}`，不提供 `on_transform_change`，`wheel_zoom="always"`。
2. 在控件内发送 precise pixel wheel，`TouchPhase::Moved`，Y delta=-40；没有显式 `Ended`。这是组件自己支持的 80ms debounce 路径。
3. 等该定时器完成，原生 scale signal 从 `1.1051709180756477` 恢复到 `1.0`，已无未提交 preview。
4. 将键盘焦点保持在 Host，按 Escape。

期望：已结束的手势不消费 Escape，Host/overlay 可以正常处理第一次按键。

实测：第一次 Host key handler 计数仍为 **0**，第二次才变为 **1**。同构的显式 Started→Ended 路径计数依次为 **1、2**；从未开始 wheel 的 idle 对照也能处理第一次 Escape。

### 根因与影响范围

- `crates/gpui-rhai/src/pan_zoom.rs:288`：`schedule_wheel_commit` 在 generation 匹配时清除 pending、恢复 transform 并提交 proposal，但没有清除 auxiliary cancellation registration。
- `crates/gpui-rhai/src/pan_zoom.rs:310`：显式 `commit_wheel` 会调用 `invalidate_wheel`。
- `crates/gpui-rhai/src/pan_zoom.rs:645`：只有这条统一清理路径调用 `clear_interaction_cancellation`。
- `crates/gpui-rhai/src/interaction.rs:968`：仍登记的 auxiliary 被视为可取消的 active owner。
- `crates/gpui-rhai/src/app.rs:160`：App 级 interceptor 看到 `cancel_window == true` 即停止后续 Escape 分发。

如果 proposal 引起成功业务 rerender，现有 View contract cancellation 可能顺带清掉旧 registration，掩盖问题；无观察者、拒绝且未改 state 的处理器等正常使用方式没有这个额外清理机会。本探针直接证明无观察者路径，不假称已经逐一执行所有拒绝变体。

此项不属于版本兼容问题，也不是要求新增键盘行为。它违反当前 ADR 已写明的“没有 active owner 时 Escape 继续交给 overlays 和 application shortcuts”。

### 修复边界

precise debounce、显式 Ended、非 precise 即时提交应共享终止/释放逻辑。generation 仍须先核验，只有当前会话的完成才能注销 ownership；不能让旧 timeout 清理一个后续新 wheel 会话。清理应在完成阶段执行，不能依赖下游业务是否选择更新 state。

建议补验：无 callback、同值拒绝、接受更新；在 debounce 完成前/后按 Escape；上一轮 timeout 晚于新一轮 Started；在 dialog 内完成缩放后第一次 Escape 可按 overlay 规则处理；source disable/unmount/dispose 后不残留所有权。

## 本轮确认通过的所有权边界

| 新增探针 | 结果 |
| --- | --- |
| `idle_escape_reaches_host` | PASS：空闲 PanZoom 不阻断 Host Escape |
| `explicitly_completed_wheel_releases_escape` | PASS：显式 Ended 完成后两次 Escape 均到达 Host |
| `debounced_wheel_releases_escape` | FAIL：确认 R4-CORE-1 |
| `other_window_escape_does_not_cancel_owner` | PASS：同一 App 下两个独立 Host/window，B 的 Escape 不改变 A 的 active preview；A 的 Escape 清理 A 且被正确消费 |
| `disposing_view_releases_window_escape_owner` | PASS：active wheel 所属 View dispose 后，第一次 Escape 可到达原 Host |

这些正向结果意味着不能把 App 级订阅本身描述为“全局键盘被无条件劫持”。当前 registration 通过 GPUI WindowId 隔离；问题出在一种正常完成方式没有结束 owner 的生命周期。

## 其他整改的源码核验

- Host interceptor 使用 `Rc::Weak` 回指 Host state，Subscription 由 Host 持有；源码未见 subscription 自身构成 Host 强引用环。dispose 的原生对照也证明了旧 View owner 不再吞 Escape。
- 原生实例 identity 现在由 presented retained key 和 NodeId 构造，render 与 retained cleanup 不再分别选构造器 key 和外层 key；旧 identity 对照由主审统一复跑，本子审查未重复执行。
- `app.rs:5035` 在汇总 sibling error 之前记录 `delivery_contract_changed`，保留了前项独立成功提交的失效事实，正对第三轮的异步 partial-success 缺陷。历史断言的最终复跑以主审汇总为准。
- direct signal read/write lease 的 previous-value RAII 恢复机制保留；本轮没有发现由这一小段改动直接引入的新重入反例。不能由此推导所有任意 Host extension/嵌套失败组合都已穷尽。

## 重现与证据

```sh
python3 docs/audits/2026-10-01-interaction-0.1.8-round4/runtime-core/run-probes.py
```

源码：[probes.rs](probes.rs)。运行器：[run-probes.py](run-probes.py)。最终完整日志：[probes.log](probes.log)。文件散列见 `SHA256SUMS`。

唯一 test target：`runtime_core_round4_audit`；唯一临时前缀：`gpui-runtime-core-round4-audit-`。只写运行器自己创建的 TemporaryDirectory，复用 `tests/native-keyboard/target`，正常等待 Cargo 锁，没有修改或枚举写入他人的临时目录。

最终 **5 项：4 PASS、1 FAIL**。FAIL 是正确契约断言保留的产品缺陷，不能表述为“全部验收通过”。只新增本轮证据，没有修改产品、旧审计、正式 tests 或 GitHub 状态，没有重复执行完整 build matrix。测试是确定性的 headless GPUI 本地输入，不代表已经完成真实设备上的所有触控板手势、多 Host 同窗口排列或操作系统窗口关闭排列。
