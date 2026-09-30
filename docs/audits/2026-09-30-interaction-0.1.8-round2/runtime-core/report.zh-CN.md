# Interaction Runtime 第二轮对抗审查

日期：2026-09-30。基线：`d77b4b49a667da618fe9b9583d331d8c325cf180`，审查整改范围 `0b9b887c → d77b4b49`。

依据整改记录和修订后的 ADR 0022：本轮按“source/constraint 替换取消旧手势、恢复受控值、最终提交一次 deferred proposal”的现行契约验收，不继续要求已从本轮契约收缩掉的 awaiting-control 协议。

## RC2-1 [P1，独立原生探针确认] 仅挂载 PanZoom 就无法暂停，失败补偿会销毁整个 View

**性质：上轮生命周期修复的剩余缺口，不是旧 Draggable probe 重复失败。** 新增的直接写入通道覆盖了 coordinator 的 cancel 阶段，但没有覆盖 native primitive 的 suspend hook，也没有覆盖该 hook 所需的 signal read。

触发仅需：

1. 挂载一个正常工作的公开 `components/pan_zoom::PanZoom`，transform 为 `{x:0,y:0,scale:1}`。
2. 不需要按下鼠标，不需要启动 wheel 或异步任务。
3. Host 调用 `view.suspend(window, cx)`。

期望：返回 `Ok(true)`，View 进入 `Suspended`；恢复后继续使用原受控值。

实测先发生：

```text
cannot read gpui_rhai::app::ScriptHostView while it is already being updated
```

补偿又发生：

```text
cannot update gpui_rhai::pan_zoom::PanZoomEntity while it is already being updated
```

最终返回：

```text
Err(Suspend("native primitive suspend failed: ... panicked during suspend;
compensation failed: ... panicked during resume")), state=Disposed
```

因此影响不只是“暂停时多了一条日志”。任何挂载 PanZoom 的 View 都可能在正常页面切换或 Host 暂停时被永久隔离并丢弃。

### 准确路径与修复范围

- `crates/gpui-rhai/src/app.rs:4383`：`suspend_view` 在 ScriptHostView 自己的 Entity update 内先执行 `primitives.suspend_mounted(cx)`。
- `crates/gpui-rhai/src/pan_zoom.rs:368`：PanZoom suspend 更新内部 Entity，再调用 `invalidate_wheel` 和 `restore_source`。
- `crates/gpui-rhai/src/pan_zoom.rs:619`：`next_wheel_generation` 首先读取 generation signal；这是第一个重入点。
- `crates/gpui-rhai/src/app.rs:3351`：`with_signal_read` 总是重新 `read_with` 原 ScriptHostView，因此与外层 update lease 冲突。
- `crates/gpui-rhai/src/app.rs:4425`：新 direct flag 仅在稍后的 `quiesce_host_view` 打开；hook 失败时尚未进入该阶段。
- `crates/gpui-rhai/src/app.rs:3329`：直接通道仅覆盖 write；单纯提前打开 flag 仍不能消除前面的 read 重入。

修复需要明确 native lifecycle hook 读取/写入自身 Runtime signal 的安全上下文，在 suspend prepare、cancel、补偿和 resume 的必要阶段均不重新 lease 正在更新的 ScriptHostView。不要只在 PanZoom 内加 catch 或忽略读取失败，也不要把清理拖到暂停提交以后；wheel generation 失效和 preview 清理应在对应阶段完成，迟到 wheel callback 不得再提交。

最小验收矩阵：PanZoom idle、pointer active、wheel active、wheel pending commit 下的 suspend→resume；和其他 primitive 共存；前后 native hook 注入失败的补偿路径；暂停后旧 wheel timer 的结果被拒绝；无 Entity 重入、状态保持正确、正常暂停不变成 Disposed。

## 本轮正向与反向证据

同一公开 API 脚手架验证了正常手势以及两种来源替换：

| 探针 | 实际结果 | 判断 |
| --- | --- | --- |
| `pan_without_replacement_control` | 拖动 delta X=50，最终 `x=50.0, commits=1` | PASS，真实手势对照有效 |
| `script_source_replacement_control` | 拖动中 Rhai 事件替换 x=300；松手后 `x=300.0, commits=0` | PASS，旧手势未覆盖新来源 |
| `native_source_replacement_is_equivalent` | 拖动中 native handler 调用 Host state API 替换 x=300；松手后 `x=300.0, commits=0` | PASS，此 PanZoom 场景未复现重入或错误提交 |
| `idle_pan_zoom_suspend_is_safe` | `Err(Suspend(...))`，`Disposed` | FAIL，确认 RC2-1 |

源码初读曾怀疑 PanZoom native 来源替换触发内部 Entity 取消重入；上述控制值、提交次数和诊断检查未支持该猜测，**不将它列为 finding**。这也说明不能只看统一 cancel 是否从一个函数调用，便断言所有组件都错误。

## 对整改架构的评价

- `InteractionOwner` 新增 retained NodeId，`PrimitiveContext` 接收实际 instance，再将其传入 owner；这明显优于要求各 primitive 自行拼接完整身份字符串。
- Host 的捕获路由由一个 Option 改为按 View 存储的 map，源缺席时的 application-drag 豁免已删除。原四个回归 probe 由主审统一复跑，本子审查不将整改记录中的通过声明充作自己的独立运行证据。
- 直接 signal lane 修正了原来 Draggable cancel 回调的同步重入，但当前边界仍是“某几个函数临时切换 flag”，不能据此推导所有 native 生命周期 hook 均安全。RC2-1 是这个覆盖不完整的确定反例。
- 成功 rerender 的公共 cancel 调用目前在 `handle_node_event`；native handler、Host/async、viewport 入口没有相同调用。Draggable axes 的异步入口反例由并行 resize-controls 审查负责，本报告不重复计数。建议在真正的成功提交边界判定失效，避免复制多个入口条件。
- “任何成功 script rerender 都取消当前 native gesture”是修订 ADR 的明确策略。因此本轮不把无关状态变化导致取消简单重复列为契约 bug；但它有明确的体验代价，若以后增加后台状态展示或计时内容，仍需评估是否应收窄到 owner/有效来源契约实际变化，不能让定时刷新使拖动一直中断。

## 可复现材料与边界

源码：[`probes.rs`](probes.rs)。脚手架：[`run-probes.py`](run-probes.py)。最终日志：[`probes.log`](probes.log)。

```sh
python3 docs/audits/2026-09-30-interaction-0.1.8-round2/runtime-core/run-probes.py
```

脚手架使用当前 native-keyboard manifest/lock、离线依赖与既有编译缓存。只在自己的 `TemporaryDirectory` 内复制测试；本轮使用专属前缀 `gpui-runtime-core-round2-audit-` 和专属 test target `runtime_core_round2_audit`，避免与主审旧探针复跑共享同名产物。最初沿用旧名的非隔离运行已作废；本报告只使用日志明确显示该新 target、且包含以上四项测试的最终运行。

最终结果 **4 项，3 PASS、1 FAIL**；失败保留正确契约断言，表示产品缺陷。没有改产品、正式 tests 或旧审计仓库文件，没有全量重复构建，没有 GitHub 写入。全部证据基于 headless 原生 GPUI 测试，不宣称完成实机 GUI 或所有跨 Host/重挂载排列验证。
