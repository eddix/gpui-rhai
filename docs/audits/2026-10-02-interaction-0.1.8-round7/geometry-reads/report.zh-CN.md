# 第七轮：geometry / ElementRef read-contribution 生命周期

基线：`cda11ce7d80f817e5675543cc7a95f577b977320`，对照 `83b19b1d`。本子域确认 **1 个 P2**，由三个 binding 生命周期场景复现；不把它们拆成三个独立问题。

产品、正式测试、旧审计和其他临时目录均未修改。测试复用 `tests/native-keyboard/target`，没有新 profile 或大型缓存。

## 证据

- [独立原生探针](src/lib.rs)
- [最终执行日志](probes.log)
- [manifest](Cargo.toml)、[锁文件](Cargo.lock)

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round7/geometry-reads/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

最终结果：**2 passed / 3 failed**。所有检查都通过公共 Rhai 组件、ScriptViewHost、真实 GPUI 点击和布局运行。

`settle` 每轮明确推进后台 clock **32ms**，重复四轮并刷新窗口，覆盖框架 16ms geometry/async poll。此前只 `run_until_parked` 的诊断中间结果不作为结论；最终 same-node 正对照已经通过，排除了未推进 poll 的夹具错误。

## R7-GEO-1 / P2：ElementRef binding 变更没有维持对同一 ref 的长期订阅

涉及代码：

- [element_ref.rs:142](../../../../crates/gpui-rhai/src/element_ref.rs#L142) 至 151：render 读取 ref 时把 ReadDependency 放进 pending map，并返回当前 NodeId。
- [element_ref.rs:169](../../../../crates/gpui-rhai/src/element_ref.rs#L169) 至 176：reconcile 无论 ref 是否绑定成功，都先 `remove` pending readers；成功后只返回当前 NodeId 对应的 readers。
- [context.rs:1613](../../../../crates/gpui-rhai/src/context.rs#L1613) 至 1626：已解析的订阅落到 GeometryRegistry 的 NodeId 上。
- [geometry.rs:817](../../../../crates/gpui-rhai/src/geometry.rs#L817) 至 822：节点离开 retained tree 后删除它的 geometry/readers，没有为失去 binding 的 ref reader 产生失效通知。

### 公共复现场景

同一个始终挂载的正式 Provider 声明 `element_ref("field")`，通过 typed Ref props 传给两个独立正式组件：

- Producer 独立维护状态，决定这个 ref 是否绑定、绑定到 Text 还是 Box，以及宽度。
- Reader 在 render 中读取 `ctx.element_bounds(props.target)`，显示 `pending` 或宽度。

只有 Producer 接收点击并更新自身状态；Reader 的 props、身份和 incarnation 不变。这是依赖系统本应处理的源变化，不是销毁 ref 所属组件后再复用旧回调。

| 场景 | 实际目标 | Reader 实际显示 | 期望 |
|---|---|---|---|
| 首次读取时尚未绑定；随后 Producer 显示目标 | width=120 | 一直 `pending` | `120.0` |
| 先测得120；同一个 ref 从 Text 换成 Box | NodeId **4→6**，width=180 | 一直 `120.0` | `180.0` |
| 先测得180；Producer 移除绑定目标 | target=None | 一直 `180.0` | `pending` |

NodeId 数字仅是本次日志中的实例值；探针独立断言前后 ID 确实不同。目标宽度/移除由实际 accessibility geometry 验证，不以 Producer 状态变量替代布局事实。

### 根因

当前 pending 结构实际是“一次 reconcile 的转接表”，不是对稳定 ref 的订阅关系：

1. 未绑定的 ref 一旦经过本轮 reconcile，reader 就被丢弃，之后绑定出现时没有人可通知。
2. 已绑定 ref 的 reader 被迁移为一次性的 NodeId 依赖；reader 不重跑时，相同 ref 重新绑定新 NodeId 不会迁移订阅。
3. 绑定消失时旧 NodeId 及其 readers 被直接裁掉，consumer 没有机会把旧几何刷新成 null。

新的 `ReadDependency.owner` 在这些场景中指向真实、仍挂载、可执行的 Reader；不是结构化虚拟命名空间被错误当成可执行 owner。问题在于 **绑定变化这一数据源没有保留并发布失效**。

### 整改要求

保持稳定 `ElementRefId → live ReadDependency` 关系，在 contribution reset/prune、owner 卸载或 ref scope 生命周期结束时清理；不能在一次 reconcile 中无条件遗忘它。

reconcile 应比较旧/新 binding，分别处理 unresolved→resolved、old NodeId→new NodeId、resolved→unresolved：迁移实际 node 几何订阅，通知真实 owner，且不留下对旧 NodeId 的过期依赖。允许目标第一次测量后再发送可用几何，但不可永久保留之前的 null/旧尺寸。

清理必须同时考虑 ref 生命周期和 reader/contribution 生命周期，避免为了修通知而累积历史 pending refs、已卸载 reader 或已离屏虚拟 item。复用现有 ReadDependency，不增加第二个无 owner 的订阅体系。

建议回归包含：

- 本报告三个 binding 转换与下面两个正对照；
- 多个独立 readers、相同 owner 的 direct/virtual 贡献并存；
- 当前 contribution 重读另一个 ref，旧 ref 不再触发；
- reader 卸载、virtual item prune、dispose 后无旧 owner 调度；
- failed render rollback 恢复之前的 ref binding 和贡献；
- suspend 时保留契约但不运行 reader，resume 后能刷新有效的新几何。

## 通过的正对照与静态覆盖

1. **相同 NodeId 的宽度更新正常。** Producer 把 Text width 从120改为180，Reader 实际显示180。说明测试时钟、poll、几何提交及常规 NodeId dirty 路径工作，失败不是“所有 geometry 都不会刷新”。
2. **虚拟行正式组件的首次 geometry read 能调度。** 一行通过 raw virtual renderer 创建正式 GeometryRow；该正式组件声明并读取自身 ref。初始 pending 在原生 prepaint 后变为120，验证正式 row owner 可执行。没有通过不支持的 ElementRef curry 制造测试；最终夹具不含这种非法传参。
3. **read-contribution 接线已覆盖 geometry registry 与 pending ref registry。** `reset_component_readers` / `for_virtual_item` 调用 geometry contribution reset，`retain_virtual_read_contributions` 同时处理 pending refs、默认 geometry 和各 presentation geometry。记录和失效都从 ReadDependency 提取 owner；没有发现把 VirtualItem contribution key 直接当作渲染入口的改动。
4. **Canvas drawable 更新使用同一 reader owner 映射。** `GeometryRegistry::update_canvas_drawable` 与外层 `update` 都从该 NodeId 的 ReadDependency 集合提取 executable owner。PrimitiveContext 的即时 Canvas bounds/inverse 查询继续是原生 imperative read，不人为引入逐帧 Rhai 订阅。本轮不重新扩展之前的 scale 数学审查。
5. **scope 删除/贡献 pruning 的静态入口存在，但不冒称完整生命周期验收通过。** `ElementRefRegistry::remove_scope` 清理 active refs 以及相关 pending owner；reset/retain 使用贡献身份。上述 unresolved/binding 生命周期缺口必须先修，才能宣称整个 ref 订阅链闭合。本报告没有把未独立复现的 resolved-reader teardown 情况另列 finding。

## 范围限制

本报告只确认上述一个 ref-binding 生命周期问题。跨 window/presentation 的独立路由、完整 suspend/dispose/virtual pruning 矩阵没有全部在此重复执行；共同 read-contribution 核心和其他 registry 由并行审查分工负责。产品161项原生测试由主审统一运行。
