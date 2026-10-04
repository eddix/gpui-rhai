# R8：窗口命令来源与 reservation 验收

日期：2026-10-04。整合候选：PR #108 head `736025a8b2b1ac44d41d893bc9a929c990b97532`；产品源码冻结于 `53a62e3f2be05e5ae7a961936648195bc4fe9477`。已核对后者至当前 head 的 core/CLI/registry/performance 产品内容无差异。

依据 [最终 Runtime 不变量](../../../adr/0022-final-runtime-invariants.md) 与 [收敛交付报告](../../2026-10-02-release-convergence/report.zh-CN.md)审查。本子范围**未确认新的阻断问题**；R7 的共享 pump 撤权反例已关闭。

## 独立运行结果：5/5 通过

| 验证 | 实际结果 |
| --- | --- |
| R7：parent 入队 Close(child)，同批另一 timer 失败，随后 dispose parent，child pump 继续 | 保持两扇窗口，child 存活；旧 source 不被重新授权 |
| R7：相同条件下入队 Open(late) | 保持两扇窗口；随后存活 child 用正常键盘 callback 再 Open(late)，成功变为三扇，取消的 reservation 可复用 |
| 同一 callback 中 Open→Focus→Close，连续两轮重用相同 ID | 每轮结束均只有 parent，没有残留窗口或诊断 |
| 显式可信 Rust Host 入队 Close(child)，随后 dispose 承载该 handler 的 View | child 被正确关闭，Rust parent 窗口保留；Host origin 没被错误降格成被撤销的 View origin |
| child init 读取 native Light，随后自行 Focus/Close；同时使用自定义执行策略 | child 正常关闭，primary/secondary 引擎均观察到 quota=300000、parser depths=(96,48) |

最后一项是 D1/D3 与窗口初始化的组合：主题选用 System/Default，primary 和 child 的 init 都会在读取到非 Light 时明确失败；child 的命令来自 init，当时窗口仍处于创建/登记流程。它验证本轮 qualification 顺序和 builder 策略继承，没有把两次 engine 配置记录等同于所有 init/effect 的调用计数。

可信 Host 对照通过公开 NativeHandler 注册 Rust 回调，由回调调用不带 source 的 `WindowCommandRegistry::request_close`；这是该 API 明确的 Host origin。Rhai 自己的 `ctx.close_window` 仍使用 View origin。两者的差异是测试目标，不是为绕过撤权添加的变通。

## 来源与目标身份的源码核验

### 原始发起者已经在入队时保存

`window.rs:80–121` 的 `QueuedWindowCommand` 包含 command、origin 和 target identity。View origin 保存原始逻辑 ID 及该次注册的 `Rc<WindowIdentity>`；Host origin 是显式分支。

`app.rs:2605–2619` 的统一检查组合了当前 target identity、当前 origin identity、source native binding 和真实 NativeWindowAuthority。`process_window_commands` 在 `:5113–5124` 检查每条 queued 命令，不再用 pump 自己的 View lease 代替 source。R7 两个真实 secondary-window 反例原有“不应执行”的断言现已通过。

### qualification 可补齐，但不能换成另一个 mount

`window.rs:441–485` 将 reservation identity 先关联 mount weak lease，再补齐 native WindowId；不同 mount lease 或不同 native binding 会被拒绝。`source_binding`/`target_binding` 从已捕获的同一 identity 读取后续补齐结果，符合 init 与 Pending Open 的契约，并非在执行时重新按字符串寻找一个新 owner。

primary 在 mount 前建立实际 appearance；secondary 先创建 native window、完成同一 reservation 的 binding，再执行 init。独立 child-init Focus/Close 组合实际通过。

### source/target 同名替换不会只靠字符串认领旧命令

`window.rs:488–507` 对 source 和 target 使用 `Rc::ptr_eq` 检查当前注册身份，同时检查 mount lease 仍有效。即使 ID 文本相同，新注册 identity 也不等于旧 identity。`app.rs:5162–5175` 在 deferred native 操作执行前再次检查 queued qualification、target authority 和 actual WindowId。

本轮对这部分完成源码核验，并实际验证 Open/Focus/Close 两轮重用相同 ID；没有声称穷举每种“同名替换发生在两个具体 effect 之间”的原生排列。既有针对 replacement 的正式测试由主审统一执行。

### Open 的取消与 reservation 回收一致

`window.rs:398–410` 撤销一条注册时按 identity 清除相关排队来源/目标，`cancel_open` (`:510–516`) 只删除仍为当前 identity、且状态仍是 Pending 的那一次 reservation。`app.rs:5122–5124` 对已经取出但失效的 queued Open 也走相同回收逻辑。

独立反例修复测试不仅断言旧 Open 没执行，还让活 child 再次使用 `late` ID 并确认新 native window 真正出现。因此没有用“窗口数没增加”掩盖 Pending tombstone 阻止后续合法 Open 的问题。

## 验收结论与边界

窗口命令这一条链路现已具备一致的身份来源：enqueue origin → 当前 source mount/native authority → 原 target reservation → native 执行前复核。已验证的失败 sibling、source dispose、reservation 重用、合法链式命令和可信 Host origin 之间没有出现新冲突。本子报告没有新的 P1/P2 finding。

这不是整包出货批准。主审仍负责 208 项正式 native、其它 Runtime 工作包、精确 head CI 及交付报告列出的 OS 人工/物理显示器门槛；本轮没有实机键盘布局、IME、VoiceOver 或 120Hz 认证。disktree 完全排除。

## 材料与重现

```sh
python3 docs/audits/2026-10-04-release-candidate-review/runtime-core/run-probes.py
```

源码：[probes.rs](probes.rs)。日志：[probes.log](probes.log)。结果 **5 passed / 0 failed**，执行 0.83 秒；增量编译 3.17 秒。

唯一 target：`runtime_core_r8_audit`；唯一临时前缀：`gpui-runtime-core-r8-audit-`。只在自己的临时 workspace 复制 probe 和 manifest，复用 `tests/native-keyboard/target`，没有复制产品到新大型 target，没有修改产品或历史证据，没有评论、合并或其它外部写入。
