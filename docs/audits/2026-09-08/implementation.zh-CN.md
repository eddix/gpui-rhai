# 实施任务书

适用基线：`d4a7901b26fa91311f3228d768037e1d52531e16`。先读[审计报告](report.zh-CN.md)、[第一性原理复审](first-principles.zh-CN.md)和原始证据，不要把本文件当作无需验证的修复结论。

## 工作规则

1. 先复现，再修改。R 类探针是当前缺陷的特性化证据，打印了缺陷不代表测试通过了正确性要求；修复时改成断言正确契约的回归测试。
2. S 类发现先建立完整触发用例。如果实际路径与报告不同，记录反证、修正结论，不要为了符合审计而制造改动。
3. 保留 UiNode/GPUI 隔离、generation、正式组件作用域、node-prop owned snapshot、native data plane、view suspend/resume 和现有正确交互。无证据时不做全量重写。
4. 维护者已明确：0.1.1 允许破坏性变更，不做历史兼容。不要补 alias、旧 parser、双运行时、旧版本支持矩阵，也不要因此要求更改发布编号或再次确认策略。原 A07 已撤回。
5. 不把 Rust 外部副作用宣传为可回滚。区分“逻辑状态恢复”“runtime 管理资源的提交/取消”“宿主外部效果”。
6. 不把 `catch_unwind`、提高限额、强制 full render、关闭虚拟化、扩大 timeout、吞掉错误用作通用修复。
7. 不删除公共 API 或扩大公开可变面来方便测试；测试内部机制可放 crate 内，验证下游契约用独立 consumer fixture。
8. 不使用墙钟阈值作为共享 CI 的稳定 oracle；结构计数、bounded work、正确交付与确定性时钟优先。
9. 每个 PR 必须记录问题、改变后的行为、失败用例与运行命令、剩余限制。审计表的状态应从证据生成或链接证据，不能仅凭文字标记完成。

## 建议 PR 顺序

第二轮调整：先用PR01建立跨域/时间身份测试，PR11明确所有权与身份后再实施PR04；PR12和PR13可以分小提交执行。主依赖关系：`PR01 → PR02/PR03/PR11 → PR04 → PR05`；`PR07` 在 `PR04` 后；`PR08` 可独立进行；`PR09` 依赖 `PR04/PR05` 的结果模型；`PR10` 汇总全部验证。一次只完成可独立审查的范围。

### PR01 — 建立故障与契约测试底座

对应 A01–A06、A09、A13、A14，以及 B01–B07 的失败用例。

- 将随报告交付的公开 API 探针整理成最小 regression fixtures，尤其是 const Map、nested operations、delayed callback 和 failed preload。
- 为事务测试定义独立 committed fingerprint：root/retained/report、state/store、callback/provenance、invocation、refs/signals、effect activation、timer 状态、async pending/active。
- 增加阶段故障注入入口，限定 `cfg(test)`，不把调试钩子加入公开 API。
- 验证当前版本 runtime、registry、CLI、生成的 Host 和应用配置组成一致的工程，不要求旧版本通过。

验收：在原始基线上能够可靠证明报告中的缺陷；修复分支中的正确性测试能区分缺陷与修复。禁止提交依赖现有 bug 永久成立的“回归测试”。

### PR02 — 修复执行会话预算

对应 A01。

主要范围：`engine.rs`、`invocation.rs`，必要时建立 crate-private execution session 模块。

- 一个外层执行会话具有预算/计量 ID；同步嵌套复用，异步延迟调用重新开始。
- 使用 pinned Rhai 的真实 progress 调用消费共享预算，不能把 parent absolute count 重复计入，也不能用最终最大/最后计数代替总量。
- 解耦 stored context 的来源数据与过期消费状态。保持 imports/private helpers/curry 同名隔离。
- 所有生产执行入口明确归属：root/state_schema/init/render、component、callback、effect、timer、virtual、theme/locale/styles loader。
- 为 operation 数据标记新的语义版本；旧数据不与新数据直接比较。

验收：四个重组件超预算失败；相同延迟 callback 不受创建前已消耗操作数影响；嵌套总量单调；失败后没有额度泄漏；纯 Rust callback 的阻塞限制仍明确。

### PR03 — 默认限制与脚本异常边界

对应 A02、A09、A13、A14。

这四项可拆成更小提交，但不要在错误处理尚不完整时宣传统一安全入口已完成。

- 默认拒绝未声明 import；所有配置 loader 采用显式来源策略。
- 处理 pinned Rhai const Map panic，选择上游修复或被特性化证明可靠的防护。恢复策略必须保证 Engine 后续可用，或者明确重建。
- import 提取与 Rhai lexer/AST 对齐；模板插值中的真实表达式仍需审计。
- durable UiValue 的有限数、递归深度、总字节/项数契约集中验证，确保 Rust/serde 与 Rhai 路径一致。

验收：跨入口测试矩阵通过；未调用用户目录之外的资源；panic 探针使用独立进程验证宿主不被误杀；Node/HostCallback/任意 Dynamic 拒绝规则不倒退。

### PR04 — 唯一的候选状态提交点

对应 A03、A04、A06。

主要范围：`app.rs`、`lifecycle.rs`、`context.rs`、Engine program/checkpoint 相关代码。

- program preload 不修改活动 exports/renderers/resolver；整个准备阶段失败也能保留旧调用能力。
- 将可组合的 candidate render 与 publish 分开，确保外层事务恢复 lifecycle root、retained 和 report。
- cancellation 在提交前保持逻辑状态，producer 终态只在提交后发布；不将快照中的 ID 集合等同于完整资源恢复。
- 明确每批 reload 涉及 script/theme/style/locale/assets/manifest 的原子边界。
- 梳理 initialize/start/dispose/suspend/resume 的所有 `?` 提前退出；清理失败要有明确状态与重试/终止策略。

验收：每个 prepare/validate/reconcile/effect/commit 阶段都能注入失败；失败后旧页面继续交互；虚拟投影先成功、普通 render 后失败时全部 committed 指纹一致；新资源 rollback 与旧资源 cancel rollback 同时成立。

### PR05 — 异步交付可靠性与 producer 终态

对应 A05、A10、B07，依赖 PR04。

- 选择并文档化 delivery 语义：独立消息独立事务，或明确的原子批次 + commit 后 ack。
- 错误消息有隔离/终态，未处理消息保持可达和有序；无无限自动重放外部效果。
- 暂停容量在 admission/dequeue 前约束；不能先丢弃再报 backpressure。
- 有界任务/订阅数量、执行并发、结果队列项数与字节数。
- worker panic/spawn 失败转成可观察终态；合作取消；内置 receiver adapter 可被取消唤醒。
- receiver adapter区分Backpressure与Closed，保留未成功入队的值并等待容量；65→64容量探针不得断流或丢值。
- 计数区分活跃回调注册、运行中的 worker、已取消但尚未退出的 worker。

验收：A/失败B/C、暂停第257项、resume失败、旧generation、producer idle、panic、dispose 都有确定性测试；资源反复挂载后回到基线。

### PR07 — 去除重复全量状态复制

对应 A08，依赖 PR04 稳定的事务契约。

- 在 viewport 没变、没有虚拟请求等情况下，先快速返回再申请事务快照。
- 一次正式 render 共享状态 staging；单实例挂载不克隆全表。
- 对固定改动集测量无关状态放大影响，再选择 entry COW 或写日志。
- NativeHandler/signal 仍必须保持规定的事务语义；不要以 bypass rollback 换取低耗时。
- 检查空闲轮询/活跃 view 数量与刷新调度，避免把每 view 的 16 ms timer 当作真实120Hz frame scheduling。

验收：原有所有 node-prop、callback provenance、虚拟投影、suspend/resume 测试通过；组件数翻倍不再出现全表复制引出的四倍成本；归因包括 allocation、state stage、Rhai、reconcile、GPUI。

### PR08 — CLI 项目计划的一致性

对应 A11、A12。

- 共享 add/update 的目标依赖闭包、资产与本地清单计划逻辑。
- staging、唯一临时文件、项目锁、journal/恢复协议；源码、manifest、baseline 为同一提交单元。
- 同时改好 conflict 后的继续处理说明与机器可见状态。
- 将 registry 源码变化但版本未变纳入 release validation。

验收：新增传递依赖/资产、已有本地修改、当前目标图不完整、任意第k次I/O失败、并发修改、进程终止恢复；dry-run 零写入；成功后 fresh consumer 校验通过。

### PR09 — 诊断与 automation 的结果契约

对应 A15、A16。

- 对外错误保留 code/source/component/generation/operation/transaction outcome。
- 将 EventResponse 的传播控制与 callback/render 的成功失败分离。
- 原始用户输入不默认进入普通 trace；提供独立脱敏输出模型。
- 保留最初错误与 rollback 错误，禁止后者无说明地覆盖前者。

验收：正常 stop、script throw、render failure、stale callback、AdvanceTime失败可区分；敏感输入在安全诊断链路不可见；普通问题仍有足够定位信息。

### PR10 — 可追溯认证与维护流程

对应 A17 和报告的架构/治理建议。PR06 随 A07 撤回，编号保留空位。

- 一个 verification manifest 管理 examples、特性、平台、自动/人工 gate。
- CI：快速格式/严格检查、default/开发/实验特性、原生行为、性能结构门槛、当前版本源码一致性、package/consumer；避免无意义重复构建。
- 受控 Mac：fresh截图对比、IME/AX/焦点/指针、文档viewer启动、120Hz帧时间和空闲能耗；为未满足项如实保留状态。
- 每次 release 证据绑定 exact commit；CLI 依赖入库后 clean install，从 `--no-verify` 过渡条件不能永久停留在注释。
- 增加依赖漏洞/许可证审计和版本升级 characterization 流程；独立 workspace lockfile 一并更新验证。
- 编写精简的维护者/agent 工作说明，链接规范与 gate，不复制整份历史计划。

验收：新加产品 target 缺 gate 会失败；证据与实际覆盖一致；维护者可从一份索引重跑，而不需猜多个文档的计数哪一个正确。

### PR11 — 明确作用域与运行时身份

对应B01、B02、B03。先设计身份关系，再分三个提交验证，不把它们合成无法审查的大重构。

- scene/view局部资源从app共享状态中分离；NodeId、geometry、presented epoch、pointer capture带所属域，retain/clear只作用于当前域。
- component incarnation与逻辑key/path分离；callback、async delivery及绑定实例的写句柄在实际使用时检查存活。ElementRef的逻辑追踪语义单独保留，不机械禁止同一活组件内的目标替换。
- program preparation session分配唯一候选身份；独立候选不能别名，但同一程序的module preload和entry仍共享正确环境。

验收：两个standalone窗口交替绘制/resize/capture不干扰；unmount/remount后旧callback/signal不能写新实例；两候选只提交B时A回调拒绝；现有keyed move、node-prop、ref迁移和合法热重载行为保持。

### PR12 — timer暂停原因与执行签名

对应B04。

- 用独立原因表示declaration、interaction和view暂停，移除某一个原因不影响其他原因。
- 明确timer签名包含哪些实际行为数据；改变curry不能被函数名相同掩盖，未变声明不能重复触发。
- 用ManualRuntimeClock测试全部相关顺序，区分连续实例和重挂载实例。

验收：原本手动暂停的timer经历view suspend/resume仍暂停；完成timer改变curry按新声明重启；原有倒计时冻结、cancel、热迁移和one-shot测试通过。

### PR13 — 所有写入入口的拒绝后置条件

对应B05、B06，联动PR03的A14。

- AssetRegistry重复register先校验再插入，拒绝后旧provider和缓存来源一致。
- StoreRegistry声明验证完整schema及default，包括serde路径；不能只依赖调用者使用new。
- Host signal写入执行与Rhai同一有限数规则；公开构造路径均检查，不引入绕过正常性能路径的反复深复制。

验收：拒绝duplicate之后加载未缓存旧资产仍成功；非法integer默认值在component/store路径一致拒绝；NaN/Infinity在Host/Rhai signal写入路径一致拒绝且旧值不变。

## 每个 PR 的最小交付格式

```text
对应审计 ID：
真实触发输入/顺序：
修复前观察：
改变后的契约：
主要实现边界：
新增正确性/故障测试及其独立 oracle：
运行命令与结果：
性能/内存变化（仅在测过时填写）：
版本和下游影响：
尚未验证或仍需平台认证的事项：
```

最终验收不能只看测试数量或报告完成勾选。维护者应能重跑当前源码、错误重载、延迟回调、异步失败、虚拟与普通同帧更新这些跨边界场景，并看到系统持续可用。
