# 第六轮：mount_window 的原生窗口授权验收

日期：2026-10-02。PR #104 head：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`，base：`c088e96f`。只审阅产品源码并增加本轮证据；未修改产品、旧审计、正式测试或外部 PR 状态。

## R6-CORE-1 · P1：最后一个 Handle 被释放并交接窗口后，旧 owner 的排队关闭仍能关闭新 owner

**公开 API 原生反例确认，应在 PR #104 合入前修复。** 显式 `dispose` 的防护已经有效，但普通 Rust Handle 释放和授权 lease 的生命周期不同步。新的 owner 已成功获权，旧 owner 仍能执行排队的强制关闭。

### 正常 Host 替换流程中的复现

探针使用与正式 `window_ownership.rs` 同构的实际 Host：Host 的 `views` 保存唯一 `ScriptViewHandle`，Render 用 `host.container(...)` 显示它。先完成真实布局和呈现，不依赖未绘制的临时 Element。

在同一个 Host update 中：

1. 从 `root.views` 移出旧 owner Handle。
2. 调用该 View 的真实 Close 节点（Automation 走已有 Rhai callback），执行 `ctx.close_window(ctx.window_id())`；原生关闭被 defer。
3. `drop(owner)` 释放最后一个公开 Handle。
4. 为同一个 GPUI native window 创建另一个 `ScriptViewHost`，调用 `mount_window` 安装新 owner；注册成功。
5. Host 显示新 View，事件周期继续执行先前排队的工作。

预期：旧 owner 的授权已经释放并交给新 owner，旧 deferred close 不得再作用于这扇窗口。新 owner 的 close-confirmation callback 保持有效。

实测：

```text
explicit_dispose=false: window_remains=false, replacement_state=Disposed
```

同构对照仅在 `drop(owner)` 前增加已有公开 `owner.dispose(cx)`：

```text
explicit_dispose=true: window_remains=true, replacement_state=Active
```

对照进一步验证新 owner 的 native close confirmation 仍能拒绝关闭。失败组不只是丢了一次回调：新 owner 管理的真实测试窗口被删掉，窗口关闭 hook 随后将新 View 置为 Disposed。

### 根因

- `app.rs:695–699`：`ScriptViewHandleInner::drop` 立即调用 `host.unregister_view(view_id)`，但不会同时把仍存活的 ScriptHostView Entity 标记为 Disposed。
- `app.rs:346–353`：unregister 删除 Host 的 `NativeWindowCommandOwner`，释放其 `Rc<()>` lease。
- `app.rs:325–340`：App 的真实 `WindowId` owner 表会清除失效 weak lease，因此 replacement 可以在旧 Entity 完全释放前成功获得同一 native window 的授权。
- `app.rs:4882–4891`：deferred Focus/Close 只持有旧 `source_state` 和旧 registry，并检查 `Disposed` 与 target `WindowId`；没有保存或核验实际授权 lease。
- 在本例中，已呈现的旧 GPUI View 在该 effect 边界仍存活，`source_state` 还是 Active，旧 registry 中仍有同一原生 WindowId。
- `app.rs:4901–4903`：旧命令直接 `window.remove_window()`，不能由 replacement 的新 close handler 拦截。

“真实目标 WindowId 没变”只能证明操作对象仍是同一扇物理窗口，不能证明发起者仍有权限操作它。需要同时验证发起者的当前授权。Focus 走同一个 deferred 分支，也受这一校验缺口影响；本轮直接验证的是 Close，没有把 Focus 的表现冒充已单独运行。

这不是要求使用者保持历史 API 兼容。即使文档推荐显式 dispose，只要 `drop` 已释放旧 claim 并允许新 owner 注册，就必须保证旧命令不再执行。不能同时允许交接、又借旧 Entity 暂时 Active 继续行使权限。

### 修复边界

把原生窗口权限实现为可验证的每次 mount 授权身份，贯穿 owner 注册、View/Handle 生命周期及 queued command 执行；不能只以 view 字符串或 `ScriptViewState` 代替授权。

可以选择一致的生命周期模型：

- 如果最后一个 Handle 的 drop 表示撤销授权，则排队工作须持有旧 lease/epoch，并在执行时确认它仍有效、仍是该 native WindowId 的当前 owner；保留的 GPUI Entity 不能延长已经撤销的权限。
- 如果授权应持续到实际 View Entity 销毁，则 Handle drop 不应提前释放 claim，让 replacement 误以为已经获权。

关键是同一套定义覆盖显式 dispose、Handle drop、native close、失败 mount 和 replacement。执行时校验 source lease 和目标实际 WindowId；旧取消/清理也应只作用于对应 mount。不要仅在 Close 闭包再加一个“weak Entity 尚存在”的条件——本例中 Entity 存在正是允许陈旧命令通过的原因。

最小验收：显式 dispose 和最后 Handle drop 两种路径，分别交接给同 Host/新 Host；重复使用或更换 view id；queued Close/Focus；replacement 的确认回调有效；旧指令无作用，新的 owner 仍能正常操作自己的窗口。

## 当前实现中有效的部分

- `mount_window` 确实连接了真实 handle registry 和 close interceptor，比原 PR #100 的单纯公开 policy 构造器完整。
- 默认 `mount` 固定使用 Disabled，旁边存在 owner 不会自动继承权限。
- owner map 使用真实 GPUI `WindowId`，而不是只靠自定义字符串 ID；这能够阻止同时通过不同 Host 别名重复获权。
- close interceptor 替换既有 should-close callback 的语义已经在公开方法文档说明；本轮不把已明确授权的替换本身当作缺陷。
- 显式 dispose 撤销 queued close 的正向对照通过。
- 同 Host、同 view id 的旧呈现 View 清理后，另一个竞争 owner 仍收到 `WindowCommandOwner`，replacement 保持 Active。该具体排列通过，不把此前对 raw view-id 清理的源码疑虑另算成已确认问题。

正式 155 项 native suite 和既有 smoke 由主审负责，本子审查未重复运行。普通 mount、冲突、失败 init、native close hook 等正式测试已阅读，但这里不以“看过测试”代替自己执行的证明。

## 独立探针

```sh
python3 docs/audits/2026-10-02-interaction-0.1.8-round6/runtime-core/run-probes.py
```

源码：[probes.rs](probes.rs)。最终日志：[probes.log](probes.log)。

| Probe | 结果 |
| --- | --- |
| `explicit_dispose_revokes_queued_authority_control` | PASS |
| `dropping_last_handle_revokes_queued_authority` | FAIL，确认 R6-CORE-1 |
| `late_old_view_cleanup_cannot_unregister_same_id_replacement` | PASS |

最终 **3 项：2 PASS、1 FAIL**。失败保留正确授权契约断言，不能算验收通过。

唯一 target 为 `runtime_core_round6_audit`，临时前缀为 `gpui-runtime-core-round6-audit-`，复用 `tests/native-keyboard/target` 并正常等待 Cargo 锁。本轮只有这一组有界 suite，没有新 profile、大型新 target 或全量重复构建。最初未实际呈现节点的脚手架不构成产品证据，已被当前真实 Host Render 版本替换；最终日志无该脚手架的 NoMatch/arena leak 结果。所有操作均为本地 headless GPUI 测试。
