# Issue #91 与 #93：运行时来源和状态默认值

依据固定提交 `f6e936a509d9aaf75f2e28836bedd439fe83887e`，不包含另一会话正在进行的工作区修复。Issue 原文保存在 evidence。

## #91：由主开发负责的独立运行时设计，建议 0.1.9

需求成立。`CapabilityHandler::call(method,input)` 没有调用来源；`UiContext::call_capability` 在通过 mutation/schema 检查后只传这两项。`ExecutionPhase::Event` 也不足以代替来源：`ScriptLifecycle::invoke_async_delivery` 为 task/subscription 等 callback 同样构造 Event context；Automation 又复用 `handle_node_event`。从“处于 Event phase”无法得出“真实用户刚刚操作此 View”。

不建议让 dogfooding 方在每个应用继续维护“窗口最近点击时间”的猜测，也不建议只给 CapabilityHandler 加一个 bool 就结束。本项目应统一持有一份可信、不可由 Rhai payload 声称的调用上下文：

- 实际入口类别：platform input、Automation、timer、task completion、subscription、effect、lifecycle/Host 主动调用；render 目前不允许 mutation/capability call，不应因增加 metadata 放宽该规则。
- 归属：Host/View/window/component incarnation；至少保证 View 级归属，不能把同窗任意一次输入视作全部 View 的授权。
- 同步嵌套调用继承本次 origin；跨 task/timer/subscription 交付切换到新的异步 origin，不能保留以前的 UserInput。
- Native primitive 的语义事件当前会 defer；它可能仍因一次真实输入产生。应通过受控事件 envelope 保留这一事实，不能简单把所有 defer 都算 TaskCompletion，也不能让保存的 callback 任意延长用户激活资格。
- 嵌套、错误、rollback、重入后恢复上层 context；Automation 明确单列，测试不靠伪造真实输入获得权限。

**origin 是事实元数据，是否具有“用户激活授权”是另一项 Host 策略。** 如果要类比浏览器的短暂激活，需要定义可消费性、有效期和边界；一个 `UserInput` enum 本身不足以作完整授权协议。可以先交付精准 origin，而不在这轮顺带实现通用权限系统。

本项目处于允许 breaking change 的阶段，没有必要照 issue 示例保留 `call`/`call_with` 两套长期 API。可由主开发选择一个一致的 handler context 入口，统一覆盖 sync/task/subscription 的启动与回调边界。现有 `&'static str event` 也未必能表达动态 action/semantic event，应先定义所需粒度。

验证应涵盖：真实鼠标/键盘与 Automation 区分、同步 helper 继承、异步资格不继承、两 View/两窗口隔离、原生 defer、error/rollback 重入，以及事件路径无额外分配/锁的测量。

这项能力目前没有被承诺，因此不把缺少 origin 单独列为现版本权限漏洞。建议在 0.1.8 核心修复收口后，由主开发以独立设计和 PR 实现；dogfooding 方负责真实用例与验收。

源码：`capability.rs:78`、`context.rs:2094`、`lifecycle.rs:780`、`app.rs:3827`。固定源码散列见 `evidence/review-baseline.json`。

## #93：本轮应完成的小型文档与诊断修复

第四轮已独立复现：非空 map 的普通 value 字面量报 `Output type incorrect: string (expecting Map)`，递归 `{type,value}` 形式成功。详见 [已完成探针](../2026-10-01-interaction-0.1.8-round4/schema/report.zh-CN.md)。

建议作者或主开发用一个小 PR 补齐：

1. USER_GUIDE 的 state defaults 明确使用 durable UiValue 编码，提供非空 map、非空 array、嵌套 optional 示例；空数组不能充当递归格式的说明。
2. root state_schema 与 formal component schema 两条 decode 入口提供字段路径和期望形态，避免让应用只能二分排查。
3. 将 issue 的失败例与递归格式成功例加入测试。

不必在 0.1.8 中引入两种默认值语法；如果要改成更符合 Rhai 直觉的普通 literal，另行设计一次完整的状态/schema 编码迁移即可，用户已明确无需历史兼容。
