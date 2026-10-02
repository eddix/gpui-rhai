# 第五轮 Rhai / async 验收

基线：`23fce23829c6694c3e077a61a20a9038b87f0759`，2026-10-02。Rhai `1.26.0`（workspace 配置 `internals` / `metadata` / `serde`），本组为默认 AST 路径。运行器从固定 Git commit 导出源码，不依赖其他会话可能修改的 live workspace。

## 结论

本组负责的 **R4 异步 scope/owner 顺序问题已修复；issue #93 的文档与默认值诊断也达到所选契约的要求**。独立探针 4/4 通过，未确认新的本组问题。没有将这一结论扩大为整体版本可出货结论。

raw virtual row 的 app/window store-path dependency owner 遗漏与 collections 组交叉确认后交由其独立探针和报告承接，本组不重复列同一问题。

## R4 取消后消息过滤：通过

`lifecycle.rs:776-785` 现在先检查精确 effect activation 是否还有效，失效消息正常返回后，才对有资格的消息执行 callback owner 校验。

新增探针比单纯卸载多覆盖了一次 **同路径重新挂载**：

1. 通过真实 `ctx.start_subscription` 创建 effect producer，发出消息并从真实队列 drain 出原始 delivery。
2. Host 正常将子组件从视图移除并 `render_dirty`，确认旧 producer `ScopeDisposed`、cleanup 执行一次。
3. 将同一个组件路径重新挂载，得到新 incarnation 和新 effect activation。
4. 交付旧消息正常丢弃，不修改新组件的 `idle` 状态。
5. 新 producer 的真实消息正常修改新组件状态。

两个身份校验正对照也通过：

- 显式同步调用旧 callback 仍报 `unmounted incarnation`。
- 在仅用于本地 API 防御性对照的 delivery 副本中，保留真实新 active scope、替换为旧 callback，仍报 owner 错误且不改变状态；未放宽有效 scope 上的 callback 身份检查。

因此修复没有用“普遍吞掉所有 stale callback”掩盖问题。正常 producer close、失败替换和 suspend/resume 的历史正确性由主审查复跑，本组不重复计数为本次新探针。

## Issue #93 默认值：通过

当前实现选择了原 issue 的第二种方案：保持递归 tagged `UiValue` 编码，并给出字段与编码说明，没有改成接受普通 Rhai Map/Array 默认值。

源码定位：

- `engine.rs:1679`：root `state_schema()` 在反序列化前校验字段默认值编码。
- `engine.rs:2317-2324`：formal component 的 `schema.state` 使用相同校验。
- `engine.rs:2350-2371`：逐字段解释默认值并包装字段路径和编码要求。
- `USER_GUIDE.md` §4：明确嵌套 map entry / array item 也需独立 type descriptor；给出 map、空 map、空 array、null 的合法形式。

独立验证覆盖：

| 场景 | 结果 |
|---|---|
| root default 为 tagged Map，但内部直接放普通字符串 | 错误包含 `state_schema.fields.data.default` 和 recursively tagged UiValue 要求 |
| formal component 同样错误 | 错误包含 `component.schema.state.fields.data.default` 和编码要求 |
| tagged Array 内含普通字符串 | root/formal 都能定位所属状态字段 |
| Map → Array → Map 内含普通字符串 | root/formal 同样明确为该字段默认值的编码问题 |
| tagged null、空 map、空 array | root/formal 均正确接受 |
| 合法嵌套 Array → Map → String/Integer | 挂载后读取的真实 `UiValue` 与预期完全一致 |

诊断目前给出**所属状态字段**，没有精确到错误的嵌套数组下标；这与 #93 要求的字段名及编码说明一致，本组未将尚未承诺的逐叶路径增强列为缺陷。

回滚对照：

- 无效 root schema 在候选 decode 阶段被拒绝，运行中的 generation 和状态未改变。
- 无效 formal default 在候选 `init` 已把 `count` 改为 99 之后，于 candidate view 中被拒绝；reload 失败后状态恢复为 7，last-good root 和 generation 保留。
- 原 callback 仍可在原 Engine 上执行，把旧状态从 7 更新为 8，证明没有只保留外观而失去旧执行上下文。

## 证据与限制

- [探针源码](probes.rs)
- [固定源码运行器](run-probes.py)：`git archive 23fce238`，唯一 test target `audit_r5_rhai_async`，复用依赖缓存，`--locked --offline`。
- [结果](probes.log)：4/4 正向通过。

执行：`python3 docs/audits/2026-10-02-interaction-0.1.8-round5/rhai-async/run-probes.py`。

没有修改产品、正式测试、历史审计或 GitHub 状态；没有重新审查未变更的 #96/#97/#99，也没有执行无界或内存耗尽测试。本组不包含 Native GPUI 窗口截图/错误横幅实测。
