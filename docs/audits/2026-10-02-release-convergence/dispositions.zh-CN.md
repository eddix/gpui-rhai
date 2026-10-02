# 0.1.8 事项处置表

这是候选交付记录，不是已合入/已关闭声明。合入、发布和tag禁令保持。
原8个开放PR与5个issue保持真实状态；须精确候选获批并实际合入后才关闭。

| 原事项 | 候选实现/处置 | 完成/关闭条件 |
| --- | --- | --- |
| #104 | 核心Source99f77b1a独立通过，docs6327deac | 最终RC完整门槛+独立验收，再获授权合入 |
| #100/#103 | 完整mount_window替代部分接口；普通mount禁用 | #104实际合入后引用真实commit关闭，不保留双轨 |
| #101 | 原作者32b7efae保留；RTL/末列/sole/extent统一 | 最终native及五张视觉门槛，合入后关闭 |
| #96 | befbe926→b2f7eef4，修订d8a3f969，PR #105 | 配额/parser/candidate/累计诊断通过且实际集成后关闭原PR |
| #97 | 31df8387→2b9caeae，修订69a7f1a2，PR #106；RC250b09c7/68962c84 | key/phase/focus/IME及完整回归，实际合入后关闭 |
| #99 | 3209039e→c37459d4，修订222bae61，PR #107 | 初次真实外观/轻量projection/scope/deps/实机门槛，合入后关闭 |
| #14 | 不纳入；保留6427de47及原研究/分支 | 授权后关闭主线集成申请；无整应用证据不引入fork/JIT |
| #98 | Host桥文档/fixture f9cb8843，stableID/shared repaint2/2 | 替代方案独立核对、合入后可关闭诉求；强revision仍独立设计 |
| #95 | 0.1.9，theme/typography维护者统一mono/列字体/测量/Studio | 两数据源与round-trip一致；span另列设计，保持open |
| #91 | 0.1.9，runtime/capability维护者定义可信origin/归属/异步资格 | 区分input/timer/task/subscription/automation，不猜最近输入，保持open |
| #89 | 0.1.9，Table维护者定义row/column/anchor与pointer/keyboard请求 | 两数据源、选择语义及已有Menu组合通过，保持open |
| #83 | 0.1.9，interaction/registry维护者定义grip hit/paint/state/token | 不仅添加slot；overhang/热路径/keyboard通过，保持open |

#93保持既有closed状态。disktree-rhai排除。新增stacked PR与最终RC由精确head/
verification清单关联；C检查点仅允许继续D，不是发布批准。
