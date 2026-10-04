# 0.1.8 R8：收敛候选独立审查

日期：2026-10-04。候选：[PR #108](https://github.com/eddix/gpui-rhai/pull/108)，HEAD **`736025a8b2b1ac44d41d893bc9a929c990b97532`**；对照上轮 `cda11ce7d80f817e5675543cc7a95f577b977320` 与[收敛实施计划](../../plans/2026-10-02-0.1.8-release-convergence.zh-CN.md)。

## 结论

**本轮确认 2 个局部 P2，未确认新的 P0/P1。整体候选仍未达到发布条件。** 上轮三个核心反例已全部关闭，窗口命令、虚拟贡献、Ref 订阅、Host policy、主题和按键的主要模型已明显收敛。两个剩余问题都在这次 Table 布局接入的边界上，不支持再启动一次大规模 Runtime 重构。

代码问题之外，实际 OS 输入/辅助功能与物理 120Hz 门槛仍待完成。五张 Table 图和精确 HEAD CI 已通过，不能再列为未完成。审查与发布状态分别记录如下。

## 新发现

### R8-T1 / P2：切换语言方向时，Table 单向丢失逻辑滚动位置

位置：[`table_layout.rs:422`](../../../crates/gpui-rhai/src/table_layout.rs#L422)。完整材料见 [Table 子报告](table/report.zh-CN.md)、[探针](table/probe.rs)与[原始日志](table/probe.log)。

同一张宽表先水平滚动到距逻辑起点 80px，再通过公开 Host `select_locale` 切换方向，列、数据与 viewport 均不变：

| 操作 | 切换前 | 切换后 |
| --- | ---: | ---: |
| LTR → RTL | 80px | **0px** |
| RTL → LTR | 80px | 80px |

当前进入 RTL 的分支仅恢复“之前也是 RTL”的位置，把之前为 LTR 和首次初始化一并按 0 处理；反向分支却会转换旧位置。因此语言切换会让用户在一个方向上丢失当前浏览位置。

建议统一先解码旧的逻辑距离，再对新范围 clamp，最后按新方向编码：未初始化为 0；旧 LTR 为 `-old_offset`；旧 RTL 为 `old_maximum + old_offset`。保留目前已通过的真实 extent、列计划和有限跟进帧机制。

这是局部一致性问题，不是旧“隐藏列不可达”P1 复发。公开指南承诺 viewport/四态变化时保留逻辑位置，未逐字承诺跨语言方向；严重性按 P2。若产品选择方向切换时回到起点，应明确规则并统一两向行为，不能留下当前的不对称结果。

验收：保持非零 offset，覆盖 LTR↔RTL 两向及范围缩小时的 clamp；原 R7 四项与当前混合列 resize 对照继续通过。

### R8-T2 / P2：Table modifiers 未进入正式组件的 presentation replay

位置：[`node.rs:1934`](../../../crates/gpui-rhai/src/node.rs#L1934) 与 [`node.rs:1962`](../../../crates/gpui-rhai/src/node.rs#L1962)。见 [虚拟/组件子报告](virtual-runtime/report.zh-CN.md)、[最小探针](virtual-runtime/probe/src/main.rs)和[记录](virtual-runtime/table-modifiers.log)。

对正式组件返回的 UiNode 使用 `.with_table_track(...)` 或 `.with_table_column(0)`，首次渲染具有正确标记；仅该子组件的 state 变脏并增量更新后，标记消失。同一返回节点外侧的 `.with_style(width=123)` 仍然保留，子组件文本也确实从 0 更新到 1。把相同 table modifier 放在组件自身 render 内，则更新前后均保留：

| 标记 | 组件 render 内设置 | 正式组件返回值外侧设置 |
| --- | --- | --- |
| table layout/track | Some → Some | **Some → None** |
| table column index | Some(0) → Some(0) | **Some(0) → None** |
| 外层 with_style 正对照 | 123 → 123 | 123 → 123 |

根因是两个新方法直接 clone 后改字段，没有使用已有 `NodePresentationMutation`。`replace_component_subtree` 替换的是组件自有快照，再重放 caller presentation；未记录的 Table 标记不会被恢复。renderer/列计划依赖这些标记，自定义组件化的 table track/cell 因而不能稳定保留其列布局身份。

修复应把这两类 metadata 纳入既有 mutation/apply/replay 机制，保持组件内部快照与调用方装饰的边界；不需要再增加第二套恢复路径。回归必须同时覆盖 track/column、内部设置/外部装饰、子组件增量更新，保留普通 with_style 正对照。

**影响限定：**这些 helper 已注册为 Rhai 可调用方法，但当前 USER_GUIDE 没有介绍它们；官方 `registry/components/table.rhai` 都在自身构造 raw box/row 时使用，本轮没有证明官方 Table 现有场景因此失效。这是新 helper 与既有正式组件组合模型的一致性缺口，按 P2 处理，建议同轮补齐，不上升为普遍 Table 故障或 P1。日志是四组 characterization 输出，进程 exit 0 不表示两个外部装饰场景达到了保留标记的期望。

## 上轮修复与本轮交叉验收

| 契约 / 工作包 | 本轮结论与证据 |
| --- | --- |
| 窗口命令来源及目标身份 | **原 R7 P1 关闭**。入队来源不会被共享 pump 改写；失效 Open 的 reservation 可被存活 peer 重用；链式 Open→Focus→Close、可信 Host origin、secondary init 与主题/预算组合 5/5 通过。[详情](runtime-core/report.zh-CN.md) |
| Table 真实内容范围 | **原 R7 P1 关闭**。原四项探针保持原字节，LTR/RTL 隐藏列均可到达，四态保持宽度、offset 和高度；新增中间 percent 列拖动后扩大 viewport，Array/LTR 与 Native/RTL 均正确，拖动 Rhai operations 为 0。[详情](table/report.zh-CN.md) |
| ElementRef 长期订阅 | **原 R7 P2 关闭**。pending→120、NodeId 4→6 后更新为 180、解除绑定→pending 及两个正对照全通过。逻辑 Ref reader 与直接 NodeId reader 分离，reconcile 会通知真实 owner。[详情](theme-geometry/report.zh-CN.md) |
| 虚拟批次、正式组件及读取贡献 | Custom Node/Nodes slot × raw/formal virtual row 四组通过：延迟实现、state/callback、精确 theme owner、Event 非订阅、prune 与失败候选回滚；另两组正式 row Ref retained/new 对照通过。[详情](virtual-runtime/report.zh-CN.md) |
| D1 Host 执行策略 | 6 项正式 policy + 2 项新增探针通过。真实 200 跳任务链累计 1,207,787 operations，各回合额度独立；降额后终止与状态回滚正确，历史诊断不被后续配置改写。[详情](policy/report.zh-CN.md) |
| D2 键名与原生 phase | 4 项新增交叉通过：同 phase Stop/StopImmediate、规范化入口共享 payload、disabled 祖先、Host chord/raw capture 对照。未把实际 GPUI action 顺序误报为 D2 缺陷。[详情](keys-cli/report.zh-CN.md) |
| D3 主题 | 本轮组合中 5 项 core + 6 项 native scope 对照通过，和 5 项 Ref 对照合计 16/16。首次 init、scope、effect deps、fixed theme、secondary 失败边界符合契约。真实 Mac idle appearance 证据按源码等价复核，未冒称重新执行。[详情](theme-geometry/report.zh-CN.md) |
| CLI 生成代码 | 当前 CLI 源码重新生成空/非空 capability consumer；均在 `#![deny(warnings)]` 下实际编译并通过 manifest 断言，手写 main 保持不变。这是本地 SDK consumer，不是已发布 crates.io 的干净安装。[详情](keys-cli/report.zh-CN.md) |
| 五张 Table 基线 | **G 已完成**。逐张查看，尺寸/hash/binary 对应一致，未发现新的静态视觉阻断。[详情](visual-review.zh-CN.md) |

此前收敛阶段另外发现的 C-TABLE-01（首次测量后缺有效跟进帧）、H-THEME-01（真正 idle 后无 appearance 入口）、E-DOC-01（维护索引缺失），当前实现与交付材料已覆盖；本轮未将旧失败重新计为当前发现。

## 测试、来源与可信边界

主审实际重新执行完整独立 native workspace，取消 `RUST_MIN_STACK` 覆盖，使用 locked/offline、单测试线程：**208 passed，0 failed，0 ignored**。见[日志](evidence/native-default-stack.log)和[汇总](evidence/native-summary.json)。各子审查另有原始输出；其中 Table 组为 **5 passed / 1 failed**，失败即 R8-T1，不能用正式 208 项全绿覆盖它。重叠执行的正式用例不相加包装成新的覆盖总数。

精确 HEAD `736025a8` 的 [GitHub CI](https://github.com/eddix/gpui-rhai/actions/runs/37134855574) 已 **completed / success**，包括 MSRV、stable、Clippy、workspace/default feature、native、性能结构、rustdoc、package、release build、Linux X11/Wayland smoke 和 artifact audit。见[本轮查询的原始 JSON](evidence/ci-current.json)。这项状态取代旧报告中的“最终 HEAD CI pending”；不是借用较早提交的绿色结果。

完整 workspace 656、默认 core 473、release smoke、package/license/performance 的细目来自现有交付记录与当前远端 CI，不冒称主审本轮在本机重跑全部。CLI package 尚有依赖发布后才能完成的真实 crates.io 安装验证，不能把当前 local-path/no-verify 证据写成已完成发行验证。

产品冻结源 `53a62e3f`、实机证据提交 `be7a8eb8` 与当前 HEAD 的 core/CLI/registry 产品树及 Cargo.lock 一致，见 [baseline.json](evidence/baseline.json)。既有 Mac appearance 探针是实际 App 的外观 override，包含真正 idle、暂停恢复、同 ID remount 与 token-only 路径；它不等于系统全局主题切换、IME 或辅助功能验收。本轮逐项核对 JSON 断言、cleanup 与来源，没有只看进程 exit 0。

## 架构判断与修复边界

这轮已经能看到原先“身份来源混用”的问题被统一模型替代：命令 origin 不等于 drainer；组件调用图不等于可见节点树；reader owner 不等于单项 contribution；逻辑 Ref 不等于当前 NodeId；Table viewport 不等于内容 extent。跨入口对照及正向功能均通过，不再只是原失败点变绿。

R8-T1 是方向转换的一个分支遗漏。R8-T2 则提示新增 UiNode metadata 仍需遵循既有 caller presentation 协议：新字段不能只接 renderer，遗漏 snapshot/replay。这值得补一个很小的同类回归，并在维护说明中明确新 node modifier 的核对要求；没有证据要求推翻现在的 retained/事务/依赖架构。

下一轮应限制为这两个修复及已有发布门槛，避免为了“架构干净”另起通用框架，也不要把 #83/#89/#91/#95 等新功能倒灌进 0.1.8。

键盘文档有一项**非阻断澄清建议**：`docs/actions-and-keybindings.md:34–35` 的“native input 然后 action”概述过宽。锁定的 GPUI 0.3.7 可先由匹配的 Host action 消费，再决定是否进入 raw key capture/bubble；本轮有原生正反对照。应解释已经被消费的 chord 不进入节点 capture，不要为迎合错误的测试假设改变运行时优先级。这不列为新的运行时 P1/P2。

## 发布门槛与 GitHub 处置

| 项目 | 当前状态 / 下一步 |
| --- | --- |
| 本轮代码发现 | R8-T1、R8-T2 待修正与定向复验；无新增 P1 |
| 五张最终 Table 图 | **已完成**，不再沿用早期 pending |
| 精确 HEAD 远端 CI | **已通过**；后续产品修复提交需要对应 CI |
| 实际 OS 输入与辅助功能 | **pending**：按计划重验适用的真实焦点/键盘、clipboard、IME preedit、VoiceOver、窗口交互。TestPlatform 与已提交文本测试不能替代 |
| 物理 120Hz | **pending**：现有实机记录为 60Hz。场景耗时、模拟 clock 与 native ops=0 都不是物理帧率验收 |
| crates.io 干净消费 | 依赖实际发布后的阶段门槛；当前未发布，不要求伪造已完成结果 |
| 合入 / 发布 / tag | 仍无执行授权，本轮没有执行 |

本轮重新查询到 **12 个开放 PR、5 个开放 issue**，见 [PR 快照](evidence/open-prs.json)和 [issue 快照](evidence/open-issues.json)。没有新的事项改变既定分流：

- **#108** 是完整整合候选；#104/#105/#106/#107 是分包材料。最终若采用分包合入，必须包含 Table 测试修订、CLI 修正及 `53a62e3f` 的真实 appearance 入口等后续提交，并核对产品树。不能只合旧 #107 而漏掉最终修复。旧包的红 CI 不自动等于已集成 HEAD 的失败。
- **#100/#103** 由完整 mount_window 接入取代；**#101** 原作者修订已纳入；**#96/#97/#99** 的修订版在 RC。待完整候选验收、获准并实际合入后，引用真实提交处理原 PR。
- **#98** 已有 Host 资产刷新桥、文档及 fixture；候选实际合入后可据此处理原诉求。更强的 per-asset revision 语义仍是独立设计，不夸称本轮交付。
- **#83/#89/#91/#95** 按计划留到后续设计；**#14** 不进入 0.1.8，保留研究与来源，授权后处理主线集成申请。它们不是这一轮需要额外实现的功能。

后续执行以[事项处置表](../2026-10-02-release-convergence/dispositions.zh-CN.md)为准。本轮未留言、关闭或合并这些事项。

## 后续验收范围

1. 用最小产品改动关闭 R8-T1/R8-T2；把独立反例转为正式有断言的回归，既有输入与期望不放宽。R8-T1 不要重做已通过的 extent 求解；R8-T2 使用现有 presentation 管道。
2. 复跑受影响 Table/组件增量测试、R7 Table 四项和本轮交叉，再按发布清单完成规定的 workspace/native/CI；如只增加归档文件，不为了追逐审计材料 SHA 无限重建相同产品。
3. 补齐真实 OS 人工证据和物理 120Hz 缺口；环境暂不可用就保留 pending，不能把已有 60Hz 数据改称通过。
4. 更新精确候选与 disposition。Table 实际绘制若受影响，核对/重拍受影响基线；不改动的其他组件不需要重复整套截图。
5. 再做针对这两处的独立复验和发布清单核验；在没有新证据指向其他问题前，不主动增加下一轮功能或新验收契约。

本轮仅新增此审计目录，未改产品、正式测试、历史报告、依赖或 disktree-rhai。未将 pre-1.0 的有意 breaking change 计为缺陷；未把不合法的 raw-row ElementRef 声明当成运行中的漏更新。最终 HEAD 与工作区证据见本目录归档。

机器可读结果：[verification.json](verification.json)。归档完整性：[SHA256SUMS](SHA256SUMS)。
