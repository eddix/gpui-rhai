# R7：共享读取贡献模型验收

基线 `cda11ce7d80f817e5675543cc7a95f577b977320`。本组检查 `read_dependency.rs`、UiContext、Store/NativeDocument/NativeCollection/Environment 的贡献登记、替换和释放；geometry/element-ref 的引用生命周期由 transforms 组独立检查，StateStore 多目标事务由 collections 组检查。

**本组没有确认新的缺陷。R6 的 66 行 raw-reader 累积问题，以及对应 shared/direct/rollback/key-reorder 边界通过独立验收。** 三个测试各运行 raw/formal 两种行表示，最终 **3/3 通过**。

## 模型核验

- `read_dependency.rs:14-16` 将可执行 `owner` 和依赖产生的 `contribution` 分开；VirtualItem 贡献由 **完整 collection id + stable item key** 标识，不再拿 raw row 的结构 scope 充当调度 owner。
- `context.rs:1227-1249` 在某一行实际重新执行前，只清空该行上一轮贡献，不会清除 root 直接读取或另一个 collection 的读者。
- `context.rs:648-672` 对所有相关 registry 应用同一贡献保留策略，包含 pending element refs、全局与各 presentation 的 geometry registry。
- `read_dependency.rs:55-76` 分别处理 owner 是否存活和 contribution 是否存活；`:80-83` 在实际失效时投影回可执行 owner，并自然去重。
- `lifecycle.rs` 以已接受候选树的 `virtual_read_contributions()` 做最终裁剪。读取登记仍在原有 runtime/geometry 快照事务之内，不在失败候选后遗留新边。

这解决了上一轮“调度归属正确，但丢失读取来源生命周期”的模型问题。本组没有建议再为不同 registry 分别添加特殊清理补丁。

## 独立实测

每个场景都同时读取 app-store path、window-store path、已有 NativeTextDocument、缺失 NativeCollection、locale、theme motion token、viewport。使用真实 RuntimeEngine / ScriptLifecycle 与 public virtual request API，不复制 registry 实现。

| 场景 | raw row | formal row |
|---|---|---|
| A 连续切换 66 个不同单项 target，B 保持自己的两项，root 继续直接读取 | 正常；当前行仍报告普通 UnknownCollection，没有触及历史 missing-name budget | 同样正常 |
| 已离屏 row-2 的 store/document 发生变化 | 返回空 invalidation owner 集合 | 同样为空 |
| A 已离开 row-0，但 B 仍读取 row-0 | 返回正确 root owner | 返回 B 中真实 Reader owner |
| 注册已离屏缺失名字 row-2 | 不触发 render_dirty | 同样不触发 |
| 注册当前可见名字 row-67 | 触发正确执行 owner，行内容变为 ready | 同样正确 |
| A/B 都移除贡献，但 root 也直接读取 shared | shared 仍唤醒 root | 同样正确 |
| 最后一个 row/direct contribution 都移除 | store/document 无残留；late register 不唤醒；environment 计数归零 | 同样正确 |
| 候选行在读取各 registry **之后**抛错 | 旧 row-65 全部边恢复；失败 row-79 没有残留；旧可见名字仍能 late-register 唤醒 | 同样正确 |
| 相同 stable keys 更改数据顺序，再按新 realized key 更新数据 | 新 key 的读者正确；旧 row-65 全部边消失 | 同样正确 |

Environment 的 locale/theme/viewport 计数在两个 collection 初始各两行加 root 时均为 5；A 单项滚动时固定为 4；移除 A/B 后只保留 root 的 1；最终为 0。这验证登记数量按**当前贡献**变化，而不是随滚动历史增长。没有把 Inspector 计数当作所有系统主题切换或原生几何反馈均已验证的声明。

## 避免一个夹具误判

初版夹具在将 `root.direct` 设为 false 后立即期待三个 environment count 为 0；实测为 4。核查真实树后发现这次 root 重绘正常重新 seed 了 A/B 各 `[0,1]`，四个读者都有活着的来源，不是泄漏。

最终测试记录这次 legitimate reseed，再对新帧的两个 collection 分别提交空 target，明确检查 `realized.is_empty()` 后验证 count 为 0。未通过放宽断言隐藏问题，也未把错误的测试前提列成产品缺陷。

## 证据与边界

- [探针源码](probes.rs)
- [运行器](run-probes.py)：唯一临时 test target `audit_r7_read_contributions`，复用既有 native dependency cache；运行前后确认 HEAD 为固定 cda11ce7 且 crates/registry/Cargo 产品 diff 为空。
- [最终输出](probes.log)：3/3 通过，含 raw/formal 正对照和实际 reseed 日志。

运行：`python3 docs/audits/2026-10-02-interaction-0.1.8-round7/read-contributions/run-probes.py`。

每次 public `request` 后立即 realization，所以该次输入就是本探针的完整目标；没有把多个未提交 partial request 当成 replacement，也没有调用私有 `request_target`。本文的 key-reorder 数据来自小型 Rhai 数组，不声称覆盖所有 NativeCollection 排序/筛选适配器。

没有修改产品、正式测试或历史材料；没有创建新 profile/独立大 target。disktree 项目按本轮要求完全排除。geometry 组确认的持久 ElementRef rebinding 问题是另一层引用失效规则，本组不重复计数、也不据本组通过而宣称它已解决。
