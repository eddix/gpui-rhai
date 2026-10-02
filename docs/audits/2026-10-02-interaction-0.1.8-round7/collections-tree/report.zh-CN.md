# 0.1.8 第七轮：虚拟批次状态与最终存活清单验收

基线：`cda11ce7d80f817e5675543cc7a95f577b977320`，对照 R6 的 `83b19b1d`。已读取第六轮整改说明。本分支只审框架内的 StateStore/batch/manifest；读取贡献另由 Rhai 分支验收，没有涉及 disktree。

**结论：本分支未发现新的确认缺陷；R6 的两个批次 P1 可以关闭。** 该结论限于下列行为与源码边界，不代替整个发布验收。

## R6 两个 P1 的独立复验

探针使用产品 RuntimeEngine / ScriptLifecycle 和公有 virtual request API。不是输出旧错误再把 exit 0 当作通过；本轮为正确行为写了明确 assertions，最终进程成功退出。

### CT6-1：父子同批导致新内层组件回调 stale——关闭

测试数据、正式 Counter、自身 state/callback 与上一轮一致。外层目标为 `[0,10]`，保留 outer-row-0 的内层目标为 `[10]`。覆盖三种顺序：

| 顺序 | 新内层 Counter 自身 callback |
|---|---|
| 先 outer 完成，再 inner | 7 |
| 先 inner 完成，再 outer | 7 |
| outer + inner 同一 batch | 7 |

三种方式均能访问新组件自身的合法 state，未再出现 StaleComponentCallback。

### CT6-2：并列提交复活已删除的状态——关闭

两个并列 collection A/B 的 row-0 分别改为 9 和 11，然后都切到 `[10]`，最后同时再次挂载 `[0]`。覆盖 A→B、B→A、同批三种方式。

全部 assertions 通过：移出的 A/B row-0 state 均不存在；重新挂载都从默认 7 开始；新实例自身 callback 都返回 7。未再被后一个 scope 的旧全局快照带回 9/11。

## 进一步组合验证

在同一个生命周期里，连续执行三批重叠目标：

1. outer `[0,10]` + inner `[10]`。
2. outer `[0,11]` + inner `[10,12]`。
3. outer `[0,12]` + inner `[12]`。

每一批中，保留或新创建的内层正式组件自身 callback 都有效。

随后让外层候选新增 branch 13，同时内层候选包含会抛错的 row 19，验证：

- 外层及内层 UiNode 恢复到错误发生前真实的 last-good target。
- 之前保留的内层 callback 仍返回 7。
- branch 13 中已准备的候选组件 state 没有泄漏到提交结果。
- 解除故意错误后可以重试，新的 row 19 callback 有效，运行时没有被失败候选卡住。
- 再同批裁掉包含内层的 outer-row-0，并带一个已过期的 inner 请求：父裁剪成功，内层不被重新创建，其旧 callback 被正常拒绝。

失败回滚的预期取自候选开始前的真实 snapshot。一次 root state 刷新会合法地根据现有 viewport metrics 补 seed 行，因此不能把某个更早的手动 target 列表硬编码为 rollback 预期；本轮探针已按此修正。

## 源码判断

- `state.rs:211-244` 的 `commit_render_batch` 从当前 state 出发，只合入每个 scope 负责且本次 seen 的 candidate 实例，最后按最终 active 集合清理被覆盖范围。它不再用某个旧 scope snapshot 替换全局 instances。
- `engine.rs:1424-1438` 以最终 invocation/virtual collection 图及真实保留根形成 active 集合；`:1449-1455` 先合并 state，再针对互不重叠的顶层 scope 统一做生命周期清理。
- 上轮父级中间 active 清单删除后建子 incarnation 的问题，已经从提交模型上消除；没有看到放宽 generation/incarnation 校验来绕过问题。
- `topmost_paths` 使用结构化 component path 顺序及 `is_within`，scope 覆盖采用结构身份而非字符串前缀。当前检查未发现把兄弟 scope 误当祖先的路径。

这种修改与差分对照结果一致，属于模型修复，而不是只覆盖原先一次失败的调用顺序。

## 证据与范围

- [probe.log](probe.log)：三组父子顺序、三组并列顺序、连续重叠/回滚/恢复/过期子请求全部断言通过。
- [probe/src/main.rs](probe/src/main.rs)：独立有界探针，数据仅 20 行，未修改历史探针或产品测试。
- 产品 `crates/gpui-rhai/tests/virtual_transactions.rs` 另有 effect/timer/signal/ref 资源回归；本分支阅读了这些断言，但没有重复执行，正式 suite 结果由主审统一报告。本次独立动态覆盖重点是 state、incarnation、自身 callback、候选 rollback 与 request 组合。
- 没有运行原生窗口、压力耗尽、磁盘树应用或新性能基线。
- 使用现有 target 缓存，无独立大缓存、无额外 profile；产品文件保持只读。

复现：

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round7/collections-tree/probe/Cargo.toml --offline
```
