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
| #99 | 3209039e→c37459d4，修订222bae61，PR #107；最终真实外观 ingress53a62e3f 在RC #108 | 初次+真正idle外观、projection/scope/deps/生命周期实机门槛，必须含53修复的最终候选实际合入后关闭 |
| #14 | 不纳入；保留6427de47及原研究/分支 | 授权后关闭主线集成申请；无整应用证据不引入fork/JIT |
| #98 | Host桥文档/fixture f9cb8843，stableID/shared repaint2/2 | 替代方案独立核对、合入后可关闭诉求；强revision仍独立设计 |
| #95 | 0.1.9，theme/typography维护者统一mono/列字体/测量/Studio | 两数据源与round-trip一致；span另列设计，保持open |
| #91 | 0.1.9，runtime/capability维护者定义可信origin/归属/异步资格 | 区分input/timer/task/subscription/automation，不猜最近输入，保持open |
| #89 | 0.1.9，Table维护者定义row/column/anchor与pointer/keyboard请求 | 两数据源、选择语义及已有Menu组合通过，保持open |
| #83 | 0.1.9，interaction/registry维护者定义grip hit/paint/state/token | 不仅添加slot；overhang/热路径/keyboard通过，保持open |

#93保持既有closed状态。disktree-rhai排除。新增stacked PR与最终RC由精确head/
verification清单关联；C检查点仅允许继续D，不是发布批准。

最终整合候选为 PR #108；产品source53a62e3f，测试/文档be7a8eb8及后续纯证据归档。
stacked #105/#106/#107 是可单独审阅的工作包，不是可独立宣称最终发布通过的包。
RC还包含Table测试表示迁移4785703f、CLI生成源/checklist修正b1aa8c7d、
native外观53a62e3f以及实机生命周期测试be7a8eb8。后续若按工作包合入，必须包含
这些收口提交并核对与批准RC的生产树相等；不能只合入旧#107然后遗漏空闲外观修复。
仅合并最终RC或其他获准等价集成方式由用户精确授权决定，当前都不执行。
