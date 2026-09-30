# 0.1.8 第三轮：集合依赖、Tree 投影、虚拟行作用域

基线：`815bb82b8103c88ef6ba50620128780c9d64b216`，对照上一轮 `d77b4b49`。只新增本目录材料，未修改产品、正式测试或历史审计。

本轮使用独立 `audit-018-collections-tree-round3` package 做小内存真实 ScriptLifecycle 探针；Tree 的性能表征逐字提取当前产品函数后以 `rustc -O` 编译。没有运行上一轮超深非法链的长时间基准。原生虚拟行回调证据来自并行 drag/sort 审查，标明来源并交叉引用。

## CT3-1 · P1：missing-read tracking 新增无界的脚本可写原生缓存

定位：`crates/gpui-rhai/src/native_collection.rs:1686-1696`；注册名验证见 `:1617-1623`；Rhai 字符串单项限制见 `engine.rs:2255` 附近。

缺失读取现在会在返回 UnknownCollection 前，把传入 name 保存到 `missing_readers`。但是这里没有调用 `validate_name`，也没有名字总数、总字节或每组件负依赖预算。脚本只需捕获读取错误，便能在不修改任何受控状态的情况下累积原生字符串和集合。

真实 ScriptLifecycle 探针每次 callback 请求 64 个不同的、长度约 527 字节的名字，并在 Rhai `try/catch` 中处理 UnknownCollection。三个 callback 都成功提交，`render_dirty=false`：

| callback 批次 | 持久保留的非法名字数 | registry Debug 输出字节数 |
|---:|---:|---:|
| 1 | 64 | 39,938 |
| 2 | 128 | 79,800 |
| 3 | 192 | 119,662 |

Debug 字节数只是低成本观测值，不是精确堆大小。相同名字经 Host register 返回 InvalidName，因为注册允许的长度至多 256 字节。这些负依赖从定义上永远不可能被注册满足。另两次 callback 使用正常短名字，仍观察到 64→128 个额外负依赖，说明只增加格式验证也不足以限制总量。

当前清理依赖组件实际重新 render 或卸载；这些 callback 没有 dirty 状态，不能指望 GPUI repaint 自动执行该清理。每次事件有新的 Rhai operation budget，native registry 的总量却跨事件累积；已有的 Array、Map、UiValue 单值上限覆盖不到它。每次事务还会克隆该 registry 的 snapshot，使增长影响后续事件成本。

证据：[probe.log](probe.log)、[probe/src/main.rs](probe/src/main.rs)。全部输入与内存规模很小，没有运行内存耗尽试验。

修复：在任何持久记录前验证 name；为缺失依赖设定明确的每组件及全 runtime 数量/字节预算；界定 event-phase 读取是否应形成持续 render 依赖。达到限制时应产生可控错误并保持最后正常状态，不能继续记账，也不能靠提高 Rhai budget。保留已修好的“正式 child 等待缺失集合后能被精确唤醒”行为。

验收：非法名字不留下缓存；重复名字去重；很多合法名字与重复 callback 有有界总资源；失败事务恢复预算与依赖；组件切换/卸载正确释放；正常缺失集合注册后仍只刷新依赖它的组件。

## CT3-2 · P2：nearest enabled parent 移到 Rust 后仍对每节点重复走相同祖先

定位：`crates/gpui-rhai/src/collection_projection.rs:110-128`。

上一轮 Rhai operation budget 溢出已解决，但实现是为每一个 source 从头扫描 disabled 祖先。既有图校验已经缓存 depth，这个新索引却没有复用最近可导航父节点的结果。复杂度为 `N × D` 次祖先查找，D 有 256 的合法深度上限；这不是原先非法无限深链的二次校验问题。

新的合法输入形状：一个 128/256 深度的主干，许多叶节点共用该主干末端作为父节点；完全折叠，只显示根一行。同一输入仅切换主干祖先的 disabled 值。逐字产品函数 `rustc -O` 表征：

| 总节点数 / 深度 | 祖先 enabled | 祖先 disabled |
|---|---:|---:|
| 1,000 / 128 | 1.34 ms | 9.42 ms |
| 4,096 / 256 | 9.36 ms | 105 ms |
| 10,000 / 256 | 16.6 ms | 290 ms |

只输出一行也执行完整索引；这个 projection 在 Tree render 时重建，导航或展开状态更新会重复付出成本。计时受到并行工作负载影响，不是整应用帧率基线；对照及源代码共同证明重复工作。

证据：[enabled-parent-bench.py](enabled-parent-bench.py)、[enabled-parent-bench.log](enabled-parent-bench.log)。产品源 SHA-256：`640ab35f8929409f9764805d6c2ea1a15a6af1af4beabee38eb7ea78e6e7fb53`。探针没有改写算法，另外确认源数据反向排列时 child 的 eligible_parent 正确为 root。

修复：沿已验证的图顺序计算并缓存最近 enabled 祖先，或者在 parent-first flatten 时把最近可导航祖先向下传递，使每条边只有常数次处理。若保留全图索引，按稳定数据版本复用。验收采用“共享长 disabled 主干 + 大量叶子”的合法森林，不仅测一条链或全部根节点。

## CT3-3 · P1：虚拟行内创建正式组件的回调会绑定到未存活的虚拟作用域

原生证据由 drag/sort 审查发现并保留：[raw-virtual-callback.rs](../drag-sort/raw-virtual-callback.rs)、[raw-virtual-callback.log](../drag-sort/raw-virtual-callback.log)。本分支负责源码因果确认，没有重复运行相同原生探针。

触发：公共 `virtual_collection` 的 `row_target(ctx,payload)` 返回正式 `DropZone`，并给它 `on_drop: Fn("dropped")`；dropped 是应用脚本中更新根状态的普通回调。实际拖放匹配目标后报错：

```text
callback `dropped` belongs to an unmounted incarnation of component
`/View[audit-drag-sort]/VirtualCollection[targets]`
```

因果链：

1. `engine.rs:3333-3340` 为 collection 创建 synthetic context。
2. `enter_virtual_collection_scope`（`:3595-3597`）仅把这个命名空间压入 render stack，它不是一个已挂载、拥有状态 schema 的正式组件。
3. 行内正式组件的 callback prop 经 `caller_component_binding`（`:2533-2535`）绑定这个 synthetic context 的 component/incarnation。
4. 成功 render 的 `context.rs:731-732` 清理非 active 正式组件的 incarnation，synthetic parent 被删除。
5. 真正派发时，`lifecycle.rs:1457-1474` 正确拒绝已经失效的 owner。

保留 renderer 的 `event_context` 并在后续 realization 对 node 做 bind，并不能覆盖已经带有 formal callback provenance 的 callback prop。问题是虚拟列表的结构命名空间被当成事件/状态所有者，不能通过关闭 incarnation 验证解决。仅给 synthetic parent 补一个空 state mount 也不充分：之后根状态字段会在错误的 state scope 里查找。

修复：在 virtual row 的构建身份、正式 item 组件状态身份、回调原始所有者之间维持清晰的映射。回调应继续指向真实拥有它的根/正式组件；item 子组件本身仍使用稳定的虚拟行身份。验证初始 seeded rows、后来 realization 的行、滚出后再进入、hot reload/dispose 后陈旧回调拒绝四个阶段。

该缺陷不一定由第三轮补丁引入；这是本轮扩大组合检查发现的现存公共能力缺陷。drag/sort 的独立 auto-scroll 测试已使用源端 `on_drag_end` 做正向对照，避免把这个回调错误误当滚动问题。

## missing-read tracking 其余边界的已验证结果

- 真实 `UiRuntimeState::begin_transaction / rollback_transaction` 恢复负依赖：回滚期间新增的名字注册后不产生 dirty；回滚前已有的名字仍正常产生 dirty。
- `release_window` 清空所属 root 的 missing readers。
- A/B 两个 root 分别等不同名字，通过 B 注册 A 等待的名字仅标 A dirty；B 的负依赖仍保留，之后注册才标 B dirty。
- A 卸载后，替换原有正向集合不再触发 A 的 reader；正负依赖的 scope 清理保持一致。
- `UiRuntimeState` 的常规 snapshot/restore 克隆整个 NativeCollectionRegistry，因此新字段没有遗漏在 rollback 之外。
- 活动 view 的周期 foreground pump 检查各自 `has_window_dirty`，共享 runtime 的另一个 view 不只依赖注册调用方的 notify 才能消费 dirty；挂起 view 仍由恢复流程消费。

## virtual source lease 静态结论

`source_item` 来自 realization 的 payload.item；租约续期再次取得同一投影的 data.item(index)，同时核对 collection、index、key、snapshot。不同来源集合或重排到别的 key 不会仅凭同名 row key 续期。fingerprint 也包含 snapshot。没有发现单独的错绑证据。

不过它使用完整行值深比较作为等价依据，会在 prepaint 克隆当前 item；后续可考虑稳定的 collection revision/row revision 避免大 payload 热路径成本。当前没有专门性能证据，不另列 finding。滚出视口续期、目标 auto-scroll、取消及输入路由由 drag/sort 原生审查负责。

## 复现

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round3/collections-tree/probe/Cargo.toml --offline
python3 docs/audits/2026-09-30-interaction-0.1.8-round3/collections-tree/enabled-parent-bench.py
```

日志是行为表征，进程 exit 0 表示探针运行完成，并不表示上面缺陷符合产品预期。
