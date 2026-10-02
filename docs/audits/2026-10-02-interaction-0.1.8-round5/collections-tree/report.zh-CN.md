# 0.1.8 第五轮：集合与虚拟组件整改验收

基线：`23fce23829c6694c3e077a61a20a9038b87f0759`，对照 `f6e936a5`。本轮全程产品工作树保持干净；仅新增本目录。使用唯一的 round5 package/bin、复用现有 target 缓存，没有压力耗尽试验。

结论：R4 的单层 manifest 和 raw-row NativeCollection 反例已经修复；继续扩大到透明正式组件组合、精确 store path 以及嵌套 virtual collection 后，确认 **1 个 P1、2 个 P2**。

## CT5-1 · P1：透明 Wrapper 的正式后代不在 UiNode 索引中，仍被误判为卸载

定位：`crates/gpui-rhai/src/engine.rs:2676-2681`；根因关联 `engine.rs:2537-2539` 与 `node.rs:1070-1077` 的单个 component_root 覆盖。

新增 `retain_virtual_component_manifest` 用 `reuse.subtrees.contains(path)` 判断哪个正式组件仍然存活。但是 `ComponentSubtreeIndex` 记录的是 UiNode 上可见的 component_root。一个正式 Wrapper 直接返回另一个正式 Counter 的节点时，外层会把同一个 UiNode 的 component_root 标记为 Wrapper；Counter 仍然是已挂载的正式组件，却不再拥有独立的 UiNode root 标记。

因此完整 target 中保留 Wrapper 的这一行，仍会丢失其中 Counter 的 state、incarnation 和 recipe。把展示节点索引当作逻辑组件存活清单，不能覆盖无额外布局的正式组件组合。

### 实际反例与两组正向对照

每组都先显示行 0、1，Counter 自身 `count=7`，自身回调 `read_count` 初始返回 7。然后请求完整 target `[0,10]`，保留行 0 并新增行 10：

| 行的实现 | realization | 保留行 0 的 Counter state | 之后调用原 Counter 自身回调 |
|---|---|---|---|
| 直接返回 Counter | Ok(true) | Some(7) | Ok(7) |
| Wrapper 直接返回 Counter | Ok(true) | **None** | **StaleComponentCallback** |
| Wrapper 返回 column([Counter]) | Ok(true) | Some(7) | Ok(7) |

三组没有父回调 prop，排除了 callback forwarding 的影响。唯一差别是是否有额外 UiNode 层级，正式组件是否存活不应由这点决定。

证据：[probe.log](probe.log) 的 `wrapper mode=direct/bare/boxed`，源程序 [main.rs](probe/src/main.rs)。这属于本轮 manifest 整改仍未覆盖的组合，不是要求兼容旧 API。

修复方向：为每个已实现 item 保留其完整逻辑组件/资源清单，或从保留的正式归属根沿 invocation/ownership 关系得到完整闭包；不要只保留有独立展示根的路径。现有普通 subtree reuse 已有“透明子组件”处理经验，可以复用相同身份模型。给用户组件强行加一层 column 只是本探针的正向对照，不是产品修复。

验收：透明 Wrapper 一层/多层、带状态的内层组件、内层 callbacks/effects/signals，连续新增和 prune-only；保留行不丢状态，退出 target 后完整清理。必须同时保留这里两个正控，不能为了避免清理而无条件留住所有后代。

## CT5-2 · P2：store 的 whole-field 与 path getter 仍使用不同的读取归属

定位：`crates/gpui-rhai/src/context.rs:1823`、`:1924-1929`。

`get_app_store`、`get_window_store` 已改为使用真实 `callback_component` 记录依赖，但对应 `get_app_store_path` / `get_window_store_path` 仍向 synthetic `self.component` 登记。raw virtual row render 后，该 synthetic 依赖会被生命周期清理；后续 store 更新不触发 UI 重算。

实际对照：每组的 store/model 初始为 `#{label:"before"}`，行 renderer 读取 label；Host 通过正常 UiContext Event setter 把 model 改为 `#{label:"after"}`：

| getter | render_dirty | 实际显示 |
|---|---|---|
| get_app_store(...).label | Ok(true) | after |
| get_app_store_path(...,["label"]) | **Ok(false)** | **before** |
| get_window_store(...).label | Ok(true) | after |
| get_window_store_path(...,["label"]) | **Ok(false)** | **before** |
| get_state(...).label | Ok(true) | after |
| get_state_path(...,["label"]) | Ok(true) | after |

`get_state_path` 已通过 `self.get_state` 委托取得正确归属，因此没有将它误报为第三个遗漏。

证据：[probe.log](probe.log) 的六组 `store`；源程序同上。

修复方向：统一 observable read 的可调度 owner，保证精确 path 只改变订阅粒度，不改变组件身份。不要把所有 `self.component` 机械替换掉：例如主题/语言的作用域查找、结构 ref 的命名空间可能仍需要结构身份；这里只针对订阅 reader 的身份。

验收：app/window 的 whole/path 对照、path 写入与 whole-field 写入两种更新、精确未命中更新不重算、reader 卸载和失败回滚。两种 getter 属于同一个根因，仅记一项。

## CT5-3 · P2：接受了嵌套 virtual collection，却无法实现其后续行

定位：`crates/gpui-rhai/src/node.rs:1232-1247`、`:1268-1291`。

最小合法组合：外层 virtual_collection 只有一个 item，该 item 返回另一个 virtual_collection；内层有 20 条数据，height/estimated_height 均明确，初始只实现头部。初次挂载成功。

从已渲染的内层 `VirtualCollectionNodeSpec.id` 请求此前未实现的 index 10：

- 同一内层 collection 单独作为 root：`realize_virtual_requests=Ok(true)`。
- 放在外层已实现 item 内：`Err(MissingVirtualCollection(... inner-a))`。

不是手造未知 ID，也不是超出 data.len：probe 从实际内层 spec 取 ID，并断言 index 10 初始未实现。

原因是 `virtual_collection_spec` 仅在当前 VirtualCollection 的 ID 正好匹配时返回；不匹配就落入 `_ => None`，没有继续搜索 `spec.realized`。替换 items 的匹配遍历也没有对应递归。滚动请求无法穿过外层 collection，虽然挂载、布局可接受这个结构。

证据：[probe.log](probe.log) 的 `nested=false/true`；源程序同上。这是本轮扩大组合验证发现的**现存未覆盖能力**，不声称由最新补丁引入。

检索公开 `docs/virtual-list.md`、组件/作者指南及其他非审计文档，没有找到禁止 nested virtual_collection 的公开契约或入口校验。核心审计中仍列有待完善的 nested scroll containment，但那不是“已接受的嵌套集合不允许实现后续行”的契约。

修复方向：集合查找和替换使用一致的、覆盖所有承载子树的遍历。继续检查嵌套 collection 的 recipe、资源清单与父目标 pruning 一起保留/删除；当前 `retain_virtual_component_manifest` 只以正式 component ID 过滤 raw nested recipe，也需要纳入该验收。后者本轮仅作相关静态风险提示，未另算一个已确认 bug。

验收：内层单独滚动、外层保留/移除内层所在 item、外内请求同一批次、失败回滚；嵌套 collection 在正常 Box/正式 Wrapper 下也应一致。如果产品明确不支持这种组合，则须在构建边界及时拒绝并明确文档，而不能先正常显示后在滚动中报 MissingVirtualCollection。

## 已通过的整改验收与覆盖边界

### 原 R4 探针

将 R4 源程序原样复制到本轮目录后，针对当前产品运行，保留历史文件不动。结果见 [r4-acceptance.log](r4-acceptance.log)、[源副本](probe/src/bin/r4_acceptance.rs)。

- Raw row `get_native_collection` 的 missing register 与已存在 replace 都能正确 dirty 并刷新，与普通 root 正控一致。
- 单层正式 Counter 行在 target `[0,10]` 后保留 state 和有效自身 callback。
- 单层 prune-only target `[0]` 清除行 1 state，其旧自身 callback 变 stale。
- Missing dependency 的 Event-only 不登记、名字验证、64/reader、4096 pairs、64KiB、去重、拒绝无副作用、release 配额释放及 rollback 继续通过。

### 新的失败候选边界

对已经成功提交 `[0,10]` 的直接 Counter 场景，请求 `[0,10,11,19]`，让行 11 成功构建、行 19 renderer 抛错。确认目标仍是 `[0,10]`，行 11 的候选 state 不存在，行 0 的旧 callback 仍返回 7。该正常 renderer 失败回滚路径通过。

这里没有把它扩大为所有资源/布局/原生错误的回滚认证，也没有用一轮 target 变化冒充完整的源数据重排性能认证。

### 复杂度观察

新增 manifest 构造处理已实现子树及该 scope 的 recipe，不再重复执行已经保留行的 Rhai renderer。它仍需要克隆/索引目标内 UiNode 和扫描相关清单，不等于零成本或只与 missing 行数相关。本次没有重新建立整应用性能基线，也没有发现新的“每帧扫全部 data”证据；当前应先使完整逻辑身份的保留与清理正确，再针对 profiler 中实际占比优化。

## 重现

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round5/collections-tree/probe/Cargo.toml --offline --bin audit-018-collections-tree-round5
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round5/collections-tree/probe/Cargo.toml --offline --bin audit-018-collections-tree-round5-r4-acceptance
```

程序包含正向 assertions 和缺陷表征。exit 0 仅说明探针完成；上面三项的实际失败已经逐项记录，不应汇总成“全部产品行为通过”。
