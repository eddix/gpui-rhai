# 第四轮在线 schema 与 issue #93 核验

基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`。日期：2026-10-01。

## 在线与离线接受域

通过公开 `SubscriptionRegistry` 的真实入队、drain 和成功/失败 callback 分流，比较新的在线 validator 与公开 `ValueSchema::validate_ui_value` 的接受结果；未抽取、改写或替换产品 validator。

**198 个 schema × 65 个值 = 12,870 组，接受结果差异为 0。** 覆盖 null、bool、整数边界、float、number、枚举、空/非空数组与 map、必填/可选/未知 object 字段、optional、one_of、不同 kind 的 handle、不能进入 durable 值域的原生类型 schema、嵌套值、NaN 和 Infinity。值与 schema 都保持小规模，测试约 1 秒。

本轮没有确认新的接受域错误。第三轮新实现直接遍历 `UiValue`，普通 array/map/object 错误使用首项退出，避免了旧的完整 `Dynamic` 转换和全量错误列表。此结论不能推广为任意 schema 的恒定开销：`one_of` 当前仍会求值各分支，成功分支没有短路；本轮没有把这项局部优化机会列为发布缺陷，也没有声称穷尽全部 schema 组合或资源边界。

## 已知 issue #93 仍可复现

[GitHub issue #93](https://github.com/eddix/gpui-rhai/issues/93) 的普通 map 默认值在当前版本的 `root_state_schema` 路径返回：

```text
failed to evaluate Rhai UI source: Output type incorrect: string (expecting Map)
```

只把 map 的每个 value 改为递归的 `{ type, value }` 描述，即可成功解码。探针固定这两项预期，验证了 issue 的契约/诊断问题，未把它误判为本次整改新引入的 bug。

建议本轮补充状态默认值文档中的非空 map、非空 array 和嵌套默认值示例，并在 decode 错误上保留字段路径。无需为此引入两种兼容语法。该项是已知 P2 文档/诊断问题，与本轮新增交互缺陷分开统计。

## 重现

```sh
python3 docs/audits/2026-10-01-interaction-0.1.8-round4/schema/run-probes.py
```

[源码](probes.rs) · [日志](probes.log)。独立临时 workspace 与唯一 `schema_round4` 测试 target，复用原生依赖缓存。两项测试通过，其中 issue #93 一项是现状特征检查；不能把“测试通过”解释为 issue 已修复。只新增本轮审计材料。
