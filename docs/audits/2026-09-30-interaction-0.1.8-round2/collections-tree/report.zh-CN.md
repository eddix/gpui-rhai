# 0.1.8 第二轮：集合与 Tree 整改复核

基线：`d77b4b49a667da618fe9b9583d331d8c325cf180`；对照 `0b9b887c`。本目录是新一轮证据，未修改产品、正式测试或上一轮材料。

本轮通过真实 `ScriptLifecycle` 的小型独立探针复核 Tree，新增 Host 动态注册与正式组件缓存的组合场景。没有重跑上一轮秒级长链。证据见 [probe.log](probe.log)，源程序见 [probe/src/main.rs](probe/src/main.rs)。

## 已经修好的部分

| 上轮问题 | 本轮结论 |
|---|---|
| Tree active=null / 空树挂载失败 | 已修复；`no active` 和 `empty` 均成功挂载 |
| 当前子节点隐藏后 reveal_key 不在数据中 | 已修复渲染异常；折叠后的合法受控状态可以挂载 |
| ArrowLeft 提议 disabled 父节点 | 已修复直接提议；父节点不可用时返回 none，向更高层查找可用祖先 |
| 全图二次祖先遍历、折叠绕过 depth | 已修复；祖先深度缓存避免重复遍历，校验独立于展开状态 |
| drag pin 每帧扫全部数据 | 已改为 collection scope + source index 单次查键，扫描已移除 |
| PR #88 realized map 沿查找路径重复 clone | 已纳入，Option::take 仅在命中目标时移动，静态复核未发现新问题 |

全图小探针：257 节点链（最大 depth=256）正常；源数据反向排列、全部展开仍正常；258 节点链不论排列顺序或展开状态都拒绝；1,000 节点反向折叠链约 7.6ms 拒绝。这里只作为行为与量级检查，未把并行负载下的 debug 时间当发布性能指标。

## CT2-1 · P1：动态注册集合后，缓存的正式子组件仍永久显示 missing

定位：`crates/gpui-rhai/src/context.rs:253-254`；`native_collection.rs:1682-1690`；`engine.rs:2627-2629`。新公开入口在 `app.rs:946-963`。

`register_native_collection_from_host` 成功写入集合后只把 root 标 dirty。一个正式子组件在集合尚未创建时调用 `get_native_collection("late")`，捕获 UnknownCollection 后展示 loading/missing，是动态创建数据源的正常占位流程。但是缺失读取在 `read_tracked` 记录 reader 前就返回错误；后续注册没有精确 reader 可失效。root dirty 又不禁止复用 props 未变、自身未 dirty 的正式子组件。

实际探针输出：

```text
live register initial root: Text { text: "missing" }
live register render_dirty: Ok(true)
live register after root invalidation: Text { text: "missing" }
live register after forced full render: Text { text: "ready:1" }
```

也就是 API 返回成功且执行了一轮渲染，集合已经存在，UI 却没有消费新集合。只有额外强制全渲染或改变该子组件的 props 才会恢复。Issue #87 的“无需预声明、运行中创建集合”使用场景因此尚未完全闭环。

修复建议：追踪读取不存在名字的依赖，并在注册时失效这些具体 reader；或明确给注册提供能覆盖相关正式组件的失效语义。不要无条件关闭全项目的组件复用优化。缺失依赖也要遵守渲染事务回滚与组件卸载清理，不能只在成功读到集合时登记。

验收：root / 正式 child / 多层 child 三种位置都先展示 fallback，再经一次公开 live-register 更新为数据；无关组件仍可复用；失败的候选 render 不留下幽灵 reader；重复注册保持错误且不覆盖既有集合。

## CT2-2 · P2：Tree 新增的 disabled 祖先导航重新引入全表重复扫描

定位：`registry/components/tree.rhai:105-110`、`:125`。

`eligible_parent` 对每个祖先都扫一遍整个 eligible 数组，再扫一遍整个 rows 数组找父节点。这发生在每次 render 中，即使用户没有按 Left。修好的 Rust 图校验是有界的，但新 Rhai 导航重新产生 `rows × ancestor depth` 工作量。

独立探针使用相同的合法 1,000 条数据：其中一条 129 节点链（depth=128），其余为根节点；链展开，active 是叶子 k128。唯一变化是前 128 个祖先是否 disabled：

- 全部 enabled：成功挂载，Left payload=k127。
- 祖先 disabled、叶子 enabled：首次挂载 `Script terminated`，`ExecutionTiming.operations=1000001`，正好超过正式运行时 1,000,000 operation budget。

这个输入低于 10,000 items 上限，深度也低于 256 上限。这里报告的是有效数据触发预算失败，不把 debug 耗时当生产帧率证据。

修复建议：复用 Rust outline 索引，在投影时同时计算每行最近的可导航祖先；或者提供 bounded key lookup，单次渲染最多进行线性的投影工作与 O(depth) 的祖先查找。不要提高全局 Rhai budget 或把完整大索引塞回受限 Rhai Map。

验收：上述 enabled/disabled 两组均可挂载；对父、祖父 disabled 的混合链，Left 目标正确；随总行数和深度变化增加确定性的访问次数/operations 门禁。

## 尚需明确的导航策略，不重复升级为本轮阻塞项

空 active 现在合法，但组件将视觉 active 置空，同时把 current 设为 0。两个节点 a/b、无 active 时，Down 的 payload 是 b，Enter 的 payload 是 a；折叠隐藏当前项后也存在同样“视觉无 active、动作以第一个项为基准”的状态。建议定义进入树时首个按键的规则并统一有效 active；本报告把它列为交互规格及测试补齐项，避免与上轮已修复的非法 reveal 异常混为一谈。

未知 expanded key 仍主动拒绝（删除展开节点后调用方必须同步整理 expanded）。当前源码一直是这一校验契约，本轮未将它单列为新回归。

## pin 的顺序变化与生命周期核查

Sortable 的 `collection_id` 来自 `ElementRef.scope`，格式与虚拟列表使用的 `component:key` 一致；`source_index` 是对应投影的 payload.index。prepaint 会检查 index 范围和该位置 key 与 source 相等，因此过滤/重排后不会直接把另一行当成原来的来源 pin。

虽然没有显式 projection revision，Sortable fingerprint 包含 source_index、前后键和 disabled；变化时取消交互。因此本轮没有仅因“缺少 revision 字段”而报告错误。不同提交入口的取消一致性另见 resize-controls 的异步 axes 反例，不能假定所有 Host/async dirty render 都已经统一取消。动态重排后的产品政策应保持明确：取消当前拖拽，或按稳定 key 重定位；不能悄悄延续旧位置。来源滚出 viewport 后是否仍保持拖拽，交由原生 drag/sort 审查的单独探针确认。

## Host 注册的其他边界

- disposed guard 已存在；重复/非法 name 在插入前拒绝。
- suspended view 注册保留集合并标 dirty，不主动通知活动绘制；本组未执行该注册／恢复组合的原生复核，不计为已验证通过。
- Host 注册发生在后续脚本 render 的事务外；后续脚本失败不应撤销已接受的 Host 数据，这与现有 replace 一致。
- NativeCollection 仍是可信 Host / application-owned registry；本次没有新增脚本任意创建入口。因此未把没有脚本配额视作脚本沙箱漏洞。动态临时集合的 unregister/所有权清理仍值得补充设计，当前实现不能据此宣称“任意大量短命集合已具备自动回收”。

## 重现

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round2/collections-tree/probe/Cargo.toml --offline
```

探针同时打印已修复行为与缺陷表征。它的进程成功退出不代表日志中的产品 ERROR 已通过验收；以每项的预期/实际对照为准。
