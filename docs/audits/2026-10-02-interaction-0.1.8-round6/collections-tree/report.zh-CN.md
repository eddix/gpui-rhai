# 0.1.8 第六轮：多集合提交与读取依赖存活

审查基线：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`，重点核查 PR #102（`c088e96f`）对 `23fce238` 的整改。已阅读上一轮 remediation。产品保持只读；历史验收与本轮产品native复跑的来源以总报告为准；本分支只增加有界的新模型探针。

结论：单个完整 target 的修复仍不能保证多个 target 同批提交正确，确认 **2 个 P1、1 个 P2**。这些反例都使用真实 RuntimeEngine/ScriptLifecycle，不是另写实现来模拟结果。

## CT6-1 · P1：父子虚拟 target 同批提交，使新内层组件一出生就成为 stale owner

定位：`crates/gpui-rhai/src/engine.rs:1062-1069`、`:1416-1422`，`context.rs:731-732`。

场景：outer 的行中含 inner virtual collection；inner 行是真正拥有 state 的正式 Counter，自身回调读取 `count=7`。同时需要把 outer 目标变为 `[0,10]`，并把保留的 outer-row-0 内层目标变为 `[10]`。

| 调度方式 | realization | 新 inner Counter state | 新 Counter 自身回调 |
|---|---|---|---|
| 先提交 outer，再提交 inner | Ok(true) | Some(7) | Ok(7) |
| 两个请求在一个 realization batch 内提交 | Ok(true) | Some(7) | **StaleComponentCallback** |

同批最终 UiNode 与 state 已存在，自己的 callback 却被拒绝。没有父 callback prop、没有异步 delivery，排除了前几轮 callback forwarding 问题。

原因：outer 的 pending commit 在 inner 新行生成之前固定了 `active.seen`；最后按祖先先行顺序提交，outer 的旧存活集合清除了后来创建的 inner Counter incarnation。inner commit 保留其 state，却不重新创建被祖先清掉的原 incarnation。此前新增的 nested 正式测试中，inner 只返回 raw text，callback 属于 Panel，因此没有覆盖内层正式 owner。

证据：[probe.log](probe.log) 的 `nested batch=false/true`；源程序 [main.rs](probe/src/main.rs)。

修复要求：以所有选中 target 的最终候选树为准，合并重叠 scope 的组件存活关系，然后统一提交 incarnation、event handlers、依赖和资源。不能让祖先依据中间清单清理后续子 target 创建的 owner。不要放宽 stale 验证，也不要仅改变提交排序来隐藏这个顺序。

验收：父保留并新增兄弟 + 子替换正式行、父纯裁剪 + 子过期请求、两层以上嵌套；顺序执行与同批执行的最终可操作性一致。带 effect/timer/signal 的新子行也必须覆盖，不能只测试 raw text 或回调归外部 root 的节点。

## CT6-2 · P1：并列集合同批提交，后一个全量 StateStore snapshot 复活前一个已清理的状态

定位：`crates/gpui-rhai/src/state.rs:186-202`，调用处 `engine.rs:1416-1422`。

两个独立的虚拟集合 A/B，各自的行是 Counter。先把 A-row-0 的 count 从默认 7 改为 9，再让两个集合都将目标切到 `[10]`；最后让 A 再次实现 `[0]`。

| 调度方式 | A-row-0 已移出后的 state | A-row-0 再次挂载后的 state |
|---|---|---|
| A、B 分别完成各自提交 | None | Some(7) |
| A、B 请求同批提交 | **Some(9)** | **Some(9)** |

这与“offscreen 行卸载 state、重新挂载使用默认值”的现有契约不一致。结果取决于两个独立目标是否被调度器合并到一个 batch。

`begin_render_scope` 保存的是整份 instances 快照；`commit_render` 清理本 scope 后，直接 `self.instances = transaction.instances`。后提交的 B 快照仍包含准备阶段的旧 A-row-0，因此覆盖了 A commit 的删除。即使修正 CT6-1 的 incarnation 清理，这个并列 scope 问题仍会存在。

证据：[probe.log](probe.log) 的 `siblings batch=false/true`。这里实际检查了移出后的 state 和再次挂载的结果，未仅根据快照代码推断。

修复要求：scope 事务不能用旧的全局快照覆盖其他已提交 scope；应把 scope 内的 candidate delta 合入同一个最终 state，或建立一次全树候选提交。保留 scope 外最新状态和删除结果。需要与 CT6-1 一起设计，单改 commit 顺序无法建立正确的事务隔离。

验收：A→B、B→A、同批三种提交方式；删除后重新出现、候选失败回滚、多个窗口/视图同一 runtime 的 scope 隔离。最终 UI、组件 state、incarnation 和资源清单必须互相一致。

## CT6-3 · P2：raw row 的依赖归 root 后失去 item 存活信息，滚动历史耗尽 missing 配额

定位：`crates/gpui-rhai/src/context.rs:1760-1763`、`:1166-1184`；清理在 `native_collection.rs:1787-1804`。

把 raw row 的读取归属改为可执行 root，解决了此前注册后不能刷新；但清理仍按 reader 的 component scope 执行。root 在 `VirtualCollection` scope 外，因此离开 target 的 raw row 所读取的名字不能随行解除。回调/调度 owner 和依赖的产生位置仍被压成了一个身份。

探针只有 66 条合法数据，滚动时 target 始终只有一条；每行读取自己名下的缺失 collection，捕获 UnknownCollection 显示占位。对照把完全相同读取包进一个正式 Loader 行组件。

| 阶段 | raw row | 正式 Loader 行对照 |
|---|---|---|
| index 2，realized=1 | 仍保留 3 个历史缺失名字 | 1 个名字 |
| index 63，realized=1 | 64 个历史名字 | 1 个名字 |
| index 65，realized=1 | **MissingDependencyBudget，limit 64** | 正常 UnknownCollection |
| Host 成功注册当前 source-65 | **render_dirty=false，仍显示预算错误** | render_dirty=true，显示 ready |

这里没有资源耗尽：只是正常跨过 64 个不同 item 的有界滚动。由于历史依赖占满配额，当前可见项甚至没能登记等待关系，之后 live-register 也无法唤醒它。

证据：[probe.log](probe.log) 的 `row formal=false/true`；登记名字数按 registry Debug 中的 map key 统计，不把正式 component path 中同名 key 重复计数。

修复要求：同时保留“哪个可执行 owner 应被失效”和“哪一个仍存活的 item/render contribution 产生了该依赖”。完整 target 移除 item 时释放其贡献，多个 item 共同读取的名字应保留必要引用；root 自身和其他集合的依赖不能被连带清空。不要增加 64 上限来掩盖存活关系，也不要简单在每次虚拟提交时清空 root 全部 reader。

验收：同一窗口连续超过 64 个 raw item、并列集合共享与不同名字、root 自身也读取名字、移除最后一个贡献后释放、保留 item 不漏订阅、失败候选回滚、当前 item 的 late register 可以刷新。

## 对当前整改模型的判断

这轮源码确实落实了上一轮要求的方向：透明后代沿 invocation parent 图保留；查找/替换穿透 nested virtual 和 Custom Node/Nodes；snapshot 更新传播到包含目标的正式 owner；path 与 whole-field 读取归到同一个真实 owner。本分支没有重报这些已经修复的单目标反例。

新的失败集中在两个仍缺少独立模型的边界：

1. **一次 foreground batch 的最终状态。** 多个完整 target 各自正确，不能推出把多个旧快照顺序赋回 runtime 也正确；父子 target 的存活集合还会重叠。
2. **依赖贡献的存活与调度归属。** 唤醒真实 root 是正确的，但 root 身份不能同时代替每个虚拟 item 的依赖生命周期。

建议用“相同最终 target，分次提交与合批提交应等价”的差分测试约束批次模型，再验证 raw/formal 两种行表达在同样的依赖生命周期下等价。这样比继续为单个旧反例添加特判更容易覆盖后续组合。

## 验证范围与限制

- 嵌套和并列场景均提供真实顺序提交正控，同批负控。
- raw 依赖场景提供同数据、同滚动序列、同读取逻辑的正式组件正控。
- 使用公有 API 发真实 virtual requests，由产品 `realize_virtual_requests` 执行；本分支未运行 GPUI 原生窗口或重新跑历史大 suite。
- 数据量为 20 或 66 行，没有资源耗尽或大性能基准。
- 编译前等待主审清理空间，随后只使用仓库原有 target 缓存；未建立独立大缓存。
- 日志为表征输出，exit 0 只表示探针完成，不表示三个负控符合产品预期。

复现：

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round6/collections-tree/probe/Cargo.toml --offline
```
