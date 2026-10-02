# 0.1.8 第四轮：集合依赖、Tree 与虚拟组件生命周期

基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`，对照 `815bb82b`。仅写入本轮审计目录；未修改产品、正式测试、历史审计或其他会话的临时文件。独立 target 为 `audit-018-collections-tree-round4`，复用仓库现有 target 缓存。

## CT4-1 · P1：增量 realization 的“新增行集合”被当成完整组件存活集合

定位：`crates/gpui-rhai/src/lifecycle.rs:448-473`、`engine.rs:1065-1091`、`context.rs:725-732`。

`realize_virtual_requests` 保留目标窗口里已经存在的 UiNode，只把 missing indices 交给 engine 执行行 renderer。但是 `finish_component_render` 把这批新生成的 `active.seen` 当成整个 `VirtualCollection` scope 的新状态，替换该 scope 的 invocation、effect、timer、signal、ref 等声明。提交时，仍在窗口内、这次没有重新执行 renderer 的旧正式行被当成已卸载。

反方向也存在：如果目标变化只是移除行，`missing` 为空，engine 的作用域提交路径根本不会执行，已经不在目标窗口的正式行状态仍被保留。

**独立实际生命周期反例及对照：** 同一个最小正式 `Counter` 组件有自己的 state `count=7`，其 `read_count` 回调只读取自身 state。它没有父组件 callback prop，因此完全不依赖“真实回调 owner 转发”是否修好。

| 操作 | UiNode 目标结果 | 实际正式组件状态／回调 | 预期 |
|---|---|---|---|
| 初始 seed 行 0、1 | 0、1 均存在 | 回调 `Ok(7)`，state=7 | 对照通过 |
| 目标改为 `[0,10]`，保留 0、新增 10 | 0 仍存在 | 行 0 state=None；原自身回调 `StaleComponentCallback` | 保留 state 和有效回调 |
| 独立实例只改为 `[0]`，删除 1、没有新行 | 1 已不存在 | 行 1 state 仍为 7；保存的自身回调仍 `Ok(7)` | 清理 state，并拒绝陈旧自身回调 |

证据：[probe.log](probe.log)、[probe/src/main.rs](probe/src/main.rs) 的 `manifest_probe`。并行 drag/sort 审查还有实际窗口对照：初始行回调成功，小幅滚动新增邻行后，仍可见的旧 DropZone 自身回调失效；参见 [原生结果](../drag-sort/probe-results.log) 的 `still_visible_seed_callback_survives_neighbor_realization`。

这是完整目标与增量工作的模型不一致。修复 `for_structural_scope` 单行不能修复它。实现需要以完整 realization target 提交组件存活清单：保留目标内旧行的 state、incarnation、recipe 和声明，合入新增行，移除已退出目标的行；即使没有 missing，也必须处理移除部分。资源与 UiNode 提交、失败回滚应使用同一个目标。

验收：上述新增／保留／纯移除三种路径；连续滑窗而非只有首次 seed；带 state、effect、timer、signal 的正式行；保留行不重复启动 effect，移除行停止资源；候选 renderer 失败时 UiNode、state、资源与 callback owner 一起保持上个成功目标。本次动态证明覆盖 state/incarnation；其他资源共享相同提交路径，应补独立回归，不能把它们未经执行就记作已测。

## CT4-2 · P2：直接在虚拟行 renderer 中读取 NativeCollection，依赖在成功提交时被清除

定位：`crates/gpui-rhai/src/context.rs:1758-1761`、`:725-726`；虚拟结构 context 建立于 `engine.rs:3333-3340`。

本次修复区分了 structural scope 与 callback owner，但 `get_native_collection` 仍把依赖登记到 `self.component`。在 raw `virtual_collection` 的行 renderer 中，该 component 是 synthetic `VirtualCollection[rows]`，它没有正式组件的可调度 render recipe，也不在 root 提交的 active 正式组件集合内。`retain_reader_scope` 随后将其正向与负向读取依赖一起清除。

**实际正反对照：** 完全相同的 `render_row(ctx,payload)` 先捕获 `get_native_collection("late")` 的 UnknownCollection 并显示 missing；集合存在时显示其行数。一个场景由普通 root 直接调用它，另一个由一条数据的 virtual_collection 调用它。

| 场景 | Host 注册 1 行的 late | 强制完整 render 后 | Host replace 为 2 行 |
|---|---|---|---|
| 普通 root 直接调用 | render_dirty=true，显示 ready:1 | ready:1 | changed=true、render_dirty=true，显示 ready:2 |
| 虚拟行 renderer 调用 | render_dirty=false，仍 missing | ready:1 | changed=false、render_dirty=false，仍 ready:1 |

证据：[probe.log](probe.log) 的 `virtualized=false/true`；源程序同上。这里的 replace 已成功更换 Host 数据，但因为没有存活 reader，其返回的 changed=false 表示没有被失效的读者。

范围是**renderer 内直接通过 ctx 读取的另一份集合**，未泛化为普通 `config.data` 订阅失效，也未声称所有正式 item 组件的集合读取都失败。

修复：为这类投影读取指定真实、可调度的 owner（例如所属正式组件/root，或专门的 collection projection 调度身份），并让负依赖预算、清理、注册唤醒和 replace 失效遵循同一归属。不要单纯让 synthetic reader 永久不清理：那样即使能标 dirty，也没有正确的 render recipe 可以执行。

验收：普通 root 对照、seed row、后来实现的 row、正式组件内部的 collection、row 切换以及失败回滚；注册缺失名字与替换已有名字分别覆盖。需要明确是否失效 owning component 或特定行投影，并证明一次更新能完成。

## 已关闭的上轮问题及边界覆盖

### Missing dependency 资源边界

本轮新公开 API 探针验证通过：

- Event phase 连续读取 256 个不同缺失名字，不产生 missing-read 缓存。
- 超长非法名字在记账前拒绝。
- Render phase 每 reader 64 名配额；重复名字去重；第 65 名拒绝且 registry 不变。
- 64 个 reader × 64 个共享名字达到 4,096 pairs 后，新 pair 拒绝且 registry 不变。
- 256 个不同的 256 字节名字达到 64KiB 后，新增名字拒绝且 registry 不变。
- `release_window` 归还所属 reader 配额。
- 真实 runtime snapshot/rollback 撤销临时负依赖；原先负依赖仍在注册时精确唤醒。

测试数据止于配置边界，未做内存耗尽。上一轮的“无界 Event negative cache”反例已经关闭。这里对容量错误使用正向断言；日志中的 `passed` 只对应这些列出的边界。

### Tree parent-first 索引

逐字提取当前产品 projection（保留算法）后，与独立简单森林遍历 oracle 对比：192 组打乱／反转输入顺序、展开／折叠、切换 disabled 的森林全部匹配 source sibling order、depth 和最近 enabled parent。

旧合法最坏形状仅做一次有界对照：10,000 节点共享 depth256 主干，完全折叠输出一行；enabled 15.1ms、disabled 16.2ms。上一轮 disabled 对照约 290ms，重复祖先扫描已消除。数字是并行负载下原函数的优化微基准，不是整应用的发布性能承诺。

证据：[tree-property-probe.py](tree-property-probe.py)、[tree-property-probe.log](tree-property-probe.log)。产品源码 SHA-256：`d5713b619fa3638893c495e3050adc8c6fcf7f7ee218d201cb00f7eec952e98d`。

### Callback owner 另行验证

初始 seed 已使用 `for_structural_scope`，但延迟 realization 的 `engine.rs:1502-1505` 仍调用 `for_component`。上面 CT4-1 使用正式行自身 callback，因此其反例不依赖该差异。drag/sort 已独立断言 Action13 不在初始 realized 集合：初始回调成功，新行转发回调进入应用函数后却在 synthetic `VirtualCollection[actions]` 中寻找根字段 count 并失败。该外部 callback owner finding 归 drag/sort 分报告，不在此重复计数。不能把 CT4-1 的自身 callback 失效与该错误 owner 合并为一次单行修复。

## 重现与判读

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-01-interaction-0.1.8-round4/collections-tree/probe/Cargo.toml --offline
python3 docs/audits/2026-10-01-interaction-0.1.8-round4/collections-tree/tree-property-probe.py
```

主程序包含正向容量断言和缺陷表征输出。exit 0 表示程序执行完毕，**不表示 CT4-1/CT4-2 的实际结果达到预期**；两者的失败条件已在表格中逐项列出。
