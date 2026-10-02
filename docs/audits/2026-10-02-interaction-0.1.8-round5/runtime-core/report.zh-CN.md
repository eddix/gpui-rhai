# 第五轮核心验收及 PR #100 处置建议

日期：2026-10-02。main 基线：`23fce23829c6694c3e077a61a20a9038b87f0759`，对比 `f6e936a5`。PR #100 单独按 head `23d009ab51d2e57957cc44d228b76e9af7f63f8e` 审阅，**尚未合入 main**。

## main 核心整改：本子范围没有确认新的阻断缺陷

旧 R4 probes 由主审统一复跑，本轮新增三个组合测试，均通过：

| 新组合 | 结果 |
| --- | --- |
| precise wheel 启动旧 debounce，Escape 取消；随后同 owner 启动新 explicit wheel，再让旧 timeout 到期 | 新 preview 保留，新 Escape 正确取消，之后空闲 Escape 可交回 Host |
| precise wheel pending 时 suspend，等待旧 timeout，再 resume | source 恢复，View 无诊断，第一次空闲 Escape 可交回 Host |
| Rotatable 已有约 92.194° 原生 preview 时 suspend→resume | suspend 返回 `Ok(true)`，恢复后角度为 0，无 Entity 读取重入或诊断 |

第三项在得知并行 transform 组也覆盖此路径前已运行；只是补充一致证据，不重复计成另一项发现。

源码核验：

- `pan_zoom.rs:295–303` 仍先检查 generation，再调用 `invalidate_wheel` 释放 auxiliary ownership、恢复 source 和提交 proposal。新清理不会无条件作用于更晚一轮 wheel；上面的跨 generation 组合验证了这一点。
- `app.rs:3200–3222` 的 `PrimitiveRuntimeReader` 将 geometry/Canvas 读取纳入现有 lifecycle access lease；在 lease 内直接读取 Runtime，平时仍通过 weak owner 读取。
- `app.rs:3441–3474` 的普通 element bounds、Canvas local point、Canvas drawable bounds 都使用该 reader。Rotatable cancel 的真实 geometry 查询已通过 suspend 路径验证。

结论有明确范围：没有在上述新组合发现主干回归，不等于对所有 arbitrary Host extension、全部 native 失败补偿、多窗口排列作了穷举，也不替代主审的整体发布判断。

## PR #100：公开 policy 构造器没有完成它宣称的 embedded close 场景

PR 链接：[feat(app): make ScriptViewHost::new_with_policy public](https://github.com/eddix/gpui-rhai/pull/100)。原始资料在 [`pr-100.json`](../evidence/pr-100.json)、[`pr-100.diff`](../evidence/pr-100.diff)。CI portable 为 SUCCESS，但变更仅包含 visibility 和文档。

**建议不按当前说明直接合入；由主开发接手有限的 Host 窗口所有权接入，再与作者协作完成 API 和测试。** `pub` 本身不是不安全操作，也可能有独立的策略配置用途；问题是它不足以让手工 mount 的顶层 Host 窗口被 `ctx.close_window` 关闭。

### 三层机制中，PR 只开放了第一层

1. **逻辑权限。** `WindowCommandPolicy::ApplicationOwned` 让 `WindowCommandRegistry::require_commands` 通过。`new_with_policy` 主要把这个策略传入逻辑窗口注册。
2. **原生窗口定位。** `process_window_commands` 必须在 `NativeWindowRegistry.handles` 找到真实 GPUI window handle。
3. **关闭和生命周期规则。** 原生 close interceptor、脚本 close handler、forced close 标记和其它同窗 View/Host 内容的清理，需要有确定的所有者。

源码证据：

- `app.rs:2457`：公开 `PreparedScriptView::mount` 给 embedded view 创建一个空的 `NativeWindowRegistry`。
- `app.rs:3547–3555`：Host policy 只进入 `mount_lifecycle` 的逻辑注册。
- `app.rs:2725–2730`：standalone `ScriptApplication::run` 才把顶层 window handle 插入表；secondary window 路径也有自己的注册。
- `app.rs:4762–4787`：Focus/Close 在原生 handle 缺失时返回 `native window ... is unavailable`。
- `app.rs:2704` 附近：standalone mount 成功后安装 close interceptor；普通 embedded mount 没有同等安装。因此仅放开 policy 也不能推断 embedded `set_close_handler` 自动拦截操作系统关窗。

这里不能用“policy 检查通过”代替“窗口已经可由脚本控制”。`open_window` 创建的 secondary window 可能走到既有 factory 路径，也不代表当前 Rust Host 自己创建的顶层窗口已经注册。

### 可执行证据：权限放开后错误变了，窗口仍未关闭

为了不改 main 或把 PR 假装已合入，probe 使用当前公开 `ScriptViewExtension::configure_window`：在 lifecycle init 前将同一逻辑窗口的 registry record 设为 `ApplicationOwned`。这精确隔离了该 constructor 提供的权限变化，没有伪造 native handle，也没有调用尚未公开的方法。

| 情况 | `ctx.close_window(ctx.window_id())` | 实际原生窗口 |
| --- | --- | --- |
| 默认 Disabled | 拒绝：`unavailable for embedded view` | 保留，符合当前默认契约 |
| 逻辑 policy 为 ApplicationOwned | 拒绝：`native window embedded-window is unavailable` | 仍保留 |

同一个启用策略的窗口调用 `focus_window` 也得到相同的原生窗口不可用错误。失败发生在命令执行阶段，印证上述注册链缺失。

这不是 PR head 的完整集成构建；它是当前 main 上对“只改变 policy 能否完成该场景”的公开 API 对照。结合 PR 仅新增 `pub` 的实际 diff，足以说明其功能说明没有完成。**这项失败归属开放 PR 的合入门槛，不应算成 main 新增的 release blocker。**

### 合入方式及 0.1.8 范围

- 若本轮必须支持 embedded 脚本关闭 Host 顶层窗口：主开发应完成一个有限、显式的 Host ownership 接口，将逻辑 ID、真实 WindowHandle、command registry 和关闭拦截连接起来。处理同窗多个 ScriptView 时，不应意外覆盖 Host 已有的 close callback 或授予未选择的 View 原生窗口控制权。
- 作者可以保留这份构造器入口作为后续实现的一部分，并补外部 crate 编译与端到端场景；不能只把文档改成“成功调用 close_window”而不测实际窗口消失。
- 如果眼下只为 disktree-rhai 的 `q` 键关窗，Rust Host 已有自己的 action/事件能力，可以由 Host 完成关闭；无需为这一个快捷键紧急公开尚未完整接入的全部 window-command policy。完整 embedded window authority 可以延后，默认 Disabled 应保持。
- 如果团队只希望现在公开 policy 以支持其它已有用途，应显式收窄 PR 标题/说明，写清当前 embedded self-close/focus 限制，并承认原 dogfooding 诉求仍未解决；不能把纯 visibility 变更当成该问题已关闭。

另外 PR 新文档把错误写成 `ScriptViewError::Window` 和“view id invalid”；实际入参是 window id，校验返回 `ScriptViewError::InvalidId`。这是作者可直接修正的小项，但不是功能接入缺口的替代修复。

### 最小正式验收

1. 从外部 crate 使用新的公开接口，手工创建 GPUI 顶层窗口并 mount；点击关闭后实际窗口集合减少，而不只检查命令入队。
2. Focus 指向正确 native window；默认 Disabled 仍拒绝操作。
3. 多个嵌入 View 和两个 Host/window 的授权隔离：谁能关闭当前窗口、谁拥有 close callback、关闭时谁负责所有 View 的 dispose，规则明确。
4. 原生关闭请求、脚本确认后强制关闭、已关闭/重复关闭、View dispose、Host 自定义 close handler 都有实际结果断言；不得静默替换 Host 既有规则。
5. 如果继续承诺 ApplicationOwned 的完整已有语义，secondary open/focus/close 及当前窗口路径一起测试。

## 本轮运行材料

源码：[probes.rs](probes.rs)。日志：[probes.log](probes.log)。

```sh
python3 docs/audits/2026-10-02-interaction-0.1.8-round5/runtime-core/run-probes.py
```

共 **5 项：4 PASS、1 FAIL**。其中 main 的三个新增交互组合和 Disabled 对照通过；FAIL 是刻意保留的 PR #100 目标行为断言，显示只有 policy 无法关闭真实 embedded window。

唯一 target `runtime_core_round5_audit`，唯一临时前缀 `gpui-runtime-core-round5-audit-`，只使用自己的 TemporaryDirectory 和共享依赖缓存。最后一次增量编译 2.46 秒，未新建大型 target、未执行全量 matrix。没有产品修改、旧审计修改、分支切换或 GitHub 写入。
