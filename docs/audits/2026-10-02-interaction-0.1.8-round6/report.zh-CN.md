# 0.1.8 第六轮验收、开放事项接手及公开dogfood审查

日期：2026-10-02。框架head：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`，对应 [PR #104](https://github.com/eddix/gpui-rhai/pull/104)；main基础是已合入的PR #102 / `c088e96f`。

**结论：PR #104 暂不建议合入或据此发布0.1.8。本轮在验收头确认3个P1、2个P2。** 其中窗口授权问题属于新adapter；两个多scope提交问题位于继承的虚拟化机制；Table状态外壳是本次视觉检查暴露的既有问题。没有把公开dogfood应用自己的缺陷混进框架计数。

本轮同时完成了公开仓库的clone和只读分析：

- 仓库：[eddix/disktree-rhai](https://github.com/eddix/disktree-rhai)。
- 本地：`/Users/eddix/Codes/github.com/eddix/disktree-rhai`，符合现有owner/repository目录习惯。
- 固定head：`e33a61c4ebc8a6cd6de7ca55b36896452ee92685`，克隆后产品源码未修改。

所有开放PR/issue的owner、排期、最小修订、关闭条件和dogfood迁移顺序见 **[本机实施Agent接手清单](handoff.zh-CN.md)**。按用户已有实现engineer的分工，本轮产出验收与可执行交接，没有把未答复的范围澄清当作直接改产品/批量关闭GitHub事项的授权。

## 1. 已通过的部分与发布状态

| 项目 | 本轮证据 |
| --- | --- |
| PR #104远端CI | **SUCCESS，精确对应83b19b1d**；[快照](evidence/ci-104-current.json) |
| 完整产品native workspace | **155项通过**，显式移除RUST_MIN_STACK覆盖；[日志](evidence/native-default-stack.log) |
| Gallery默认线程栈 | **15项通过**，包含原viewport/locale matrix、完整shell和所有story；也包含在上述155项中，不重复累加 |
| Table边界/RTL/原生操作 | **12项通过**（7正式+5独立交叉）；前#101错列、误排序问题已关闭 |
| Style Rc/COW | **3项独立公开API对照通过**：输入隔离、默认共享/clone隔离、formal局部重绘与caller style/旧树隔离 |
| Table五状态图片 | 五张现场候选已采集，**正式视觉验收尚未通过**；三张规范尺寸、两张1点偏差，并确认loading/empty越界 |

完整native suite不是新反例的替代品。作者的616项workspace、Clippy、package及完整release smoke没有在本轮全部重跑；远端CI与作者材料分别说明来源，不混成独立验收数量。此前历史suite也没有在本轮全量重复执行；本轮重点是新批次/授权边界及已经build的完整产品native suite。

Table正向证据：[resize-controls报告](resize-controls/report.zh-CN.md)。Style证据：[style-geometry报告](style-geometry/report.zh-CN.md)。

## 2. P1：释放最后Handle后，旧授权的关闭请求仍能关闭新owner窗口

位置：`app.rs:695` 的 `ScriptViewHandleInner::drop`，`:346` 的unregister，以及`:4882–4903`的deferred native operation。

在真实已呈现的Host中，同一次Host update内执行：

1. 旧command owner通过正常Automation/脚本将close排队。
2. pop并drop最后一个ScriptViewHandle，没有显式调用dispose。
3. 新Host成功以mount_window接管同一个真实GPUI窗口。
4. 处理已排队的native operation。

实际结果：**窗口被旧请求关闭，新owner进入Disposed**。同构的显式dispose对照则保持窗口存在、新owner Active。

最后Handle释放会撤掉Host持有的authority lease，因此新owner可以获权；但旧Entity还可能由已绘制frame等合法对象持有，source_state仍是Active。执行闭包只检查这个state及旧registry中的WindowId，没验证当前mount的授权lease。Entity仍活着、WindowId仍相同都不能证明旧请求仍有权执行。

整改：请求必须携带发起mount的不可混淆lease/epoch，执行时确认当前权限仍有效；或把claim生命周期与真正Entity销毁/撤权保持一致。不能仅增加weak Entity是否存在的判断。source、target、替换owner和同ID重挂载均应有明确身份。

验收保留：最后Handle drop、显式dispose、仍有一个Handle clone、同Host/新Host replacement、旧focus与旧close、确认策略与force-close路径。只检查命令成功入队不能算关闭语义验收。

[原生探针及报告](runtime-core/report.zh-CN.md)：最终3项，2个正对照通过、1个产品期望断言失败。此项是PR #104合入前的明确阻断。

## 3. P1：父子虚拟target同批提交会撤销刚创建的内层owner

位置：`engine.rs:1062–1069`、`:1416–1422`，`context.rs:731–732`。

outer保留row0并新增row10，同时row0中的inner转到正式Counter row10。顺序提交outer再inner时，Counter state=7、自身callback返回7；将相同两个请求放在一次realization batch中，整体返回Ok、state仍为7，**新Counter自己的callback却立即StaleComponentCallback**。

outer pending commit的active集合在inner新行建立之前已经冻结；祖先提交清掉后来建立的inner incarnation，后续子提交没有恢复它。raw text或归外层Panel的callback不足以覆盖该路径，所以已有nested测试可以通过。

整改：根据所有target完成后的最终候选树，合并重叠scope的存活关系，统一提交incarnation、事件、状态与资源。不能通过放宽stale检查或交换提交顺序来隐藏问题。

这是当前已合main的单target修复未覆盖的批次组合；不表示PR104的窗口代码制造了该问题。

## 4. P1：并列集合同批提交会复活已卸载状态

位置：`state.rs:186–202` 的全量snapshot提交，调用于 `engine.rs:1416–1422`。

A/B两个独立集合各有Counter，先把A-row0从默认7改成9，再使A/B都切到row10，最后让A重挂row0：

| 提交方式 | A-row0移出后的state | 重挂后的state |
| --- | --- | --- |
| 分别提交A、B | None | 默认7 |
| A、B同批 | **仍为9** | **仍为9** |

scope事务保存了整份StateStore；B的旧全局snapshot在提交时覆盖A刚完成的删除。组件再次出现后继承应已卸载的状态，行为取决于调度器是否合批。

整改：scope内candidate delta只能合入其负责部分，或整个batch共用一次最终候选提交。必须保留其他scope最新的修改和删除，失败时原子回滚。该根因与上一项incarnation清理不同，修一个不会自动修另一个；两者应在同一批次模型里处理。

第3/4项证据：[collections-tree报告](collections-tree/report.zh-CN.md)、[真实RuntimeEngine/ScriptLifecycle输出](collections-tree/probe.log)。数据为20行级别的public API请求，均有顺序提交正控。

## 5. P2：raw row的依赖按滚动历史累积，当前行最终无法订阅

位置：`context.rs:1760–1763`、`:1166–1184`，`native_collection.rs:1787–1804`。

66条合法数据，target始终只有一条。每行读取自己名下的缺失collection并显示占位：raw row滑到index63时已保留64个历史名字；index65触发missing dependency预算，即使此时只实现一行。Host随后成功注册当前source65，**没有dirty，仍显示预算错误**。

把同一读取放入正式Loader行组件，整个序列只保留一个名字，注册source65后正常显示ready。

真实root作为失效调度owner是正确方向，但它不能替代每个item产生的依赖贡献。整改需要同时记录“失效谁”和“谁的存活贡献了这条订阅”。释放移出item的贡献，共享名字仍由其他存活贡献保留；root自身依赖、其他集合以及失败候选不受连带清理。不能提高64上限或每次滚动清空root全部reader来掩盖问题。

证据同collections-tree报告。这是普通有界滚动，不是内存耗尽测试。

## 6. P2：Table loading/empty绕开横向viewport，标题画出边框

位置：`registry/components/table.rhai:499–504`、`:509–514`、`:609–614` 的三个early return；正常数据分支`:631–636`具有min_width0与overflow_x_scroll。

本轮精确980×752的loading/empty截图中，Table右边框约x903，Action标题却画在x913以后。相同列声明的数据态被正确限制在viewport中。宽表合法，状态切换不应把横向滚动区域变成默认可见overflow。

整改：共用Table外壳、边界和横向viewport，只切换内部data/loading/empty内容；覆盖全部三个early return。不能只把当前demo改窄来让问题不显现。

Score只剩省略号则由当前示例的固定730px+Joined25%列声明解释，属于demo可读性改进，不报告另一个核心flex算法P1。建议默认示例给Score可读宽度，或把超宽组合单列为overflow fixture。

证据：[视觉补充](resize-controls/visual-followup.zh-CN.md)、[五状态候选说明](visual/report.zh-CN.md)。该缺陷早于logical inset修改，本轮实际截图使它显现。

## 7. 开放PR/issue已形成完整本机接手安排

全部8个开放PR、5个开放issue已读取并保存快照，具体执行与关闭条件见[handoff.zh-CN.md](handoff.zh-CN.md)：

- #104先修本报告阻断，再基于最终commit验收。
- #100/#103在#104完整能力验收并合入后关闭为已覆盖；不再合入第二套policy/registry入口。
- #101作者提交已保留，RTL部分已独立通过；#104合入及视觉验收后关闭为已纳入。
- #96/#97/#99由本机engineer按上次评审修订后集成，不依赖原作者机器；#96表达式深度现在有真实skin的64通过证据，建议单独Host配置。
- #14仍建议结束当前集成目标、保留实验数据，不作为0.1.8功能。
- #98先文档/Host桥；#95/#91/#89/#83保持明确的后续核心设计/增强安排，避免为清空列表关闭未完成需求。

“处理”在本轮表示完成逐项评审、复现、归属和可执行交接。GitHub的合并、关闭和评论尚未执行，未声称已经关闭这些事项。

## 8. 公开dogfood的价值与独立问题

这个项目确实验证了可替换Rhai布局算法：Host持有扫描树，脚本按层读取受限数据，两个skin可替换tiling。保留这一路线，先修正确性与缓存边界，不必将全部布局搬回Rust。

但当前checkout不能直接用已验收main运行：它使用未合#96的operation_limit、#97的标点、#99的theme_variant，并沿用已被新adapter取代的new_with_policy。path dependency也没有固定相邻框架的Git版本。应先完成受控集成并记录两仓SHA。

应用自身的重点整改：

1. **P1旧扫描发布竞态**：取消消费者后，旧ScanHandle完成仍先覆盖SharedTree，再尝试发送done。纯内存fixture已复现，未调用scanner、未扫描用户目录。发布必须受当前request/activation版本约束，不能只靠100ms progress发送发现取消。
2. 原始路径经lossy展示名再重建会失真；invalid crumbs静默回到祖先。保存原生路径身份，展示名与动作目标分开。
3. `--theme system`被Host归一成dark；即使修正，paint cache/effect deps也未随主题重建。geometry与palette应分开缓存。
4. by_key尚为空时就写入cache，之后回填不会改变已存Map副本；viewport整数cache key与浮点deps还不一致。
5. `/`未绑定、`k`重复注册；过滤模式会把left/tab等named key拼成文本。两套skin都需修并使用真实输入对照。
6. 1200项小fixture已超累计数组限额，当前“2000 tiles+单槽cache”不能保证安全；先去重复数据持有、按整份值预算，而非直接抬高框架全局限制。
7. README默认`cargo run -- --dev`未启用dev feature；raw rhai_check的depth1000也不是实际RuntimeEngine验收。完成声明需要与验证commit一致。

新解析证据：两个实际main.rhai在32/48失败、64/128通过，core文件在32通过；这支持Host可配置深度，不证明必须全局128。具体来源、最小修法和本机实施顺序见[脚本侧报告](dogfood-script/report.zh-CN.md)、[Host侧报告](dogfood-host/report.zh-CN.md)。

这些结论来源于源码和有界fixture，未声称完整dogfood桌面应用已编译运行、45项作者测试已重跑或真实文件扫描已验收。

## 9. 截图与环境收尾

五张候选保留在本轮visual目录。三张实际1960×1504经过2×到1×密度归一，得到980×752；另外两张实际1962×1504，保留原图，未缩放伪装成规范viewport。加上已确认的Table状态溢出，**正式五张baseline仍未批准更新**，旧PNG保持不变。

截图过程中只给本轮精确bundle ID增加临时floating规则，结束后核对runtime app_rules完全恢复；本轮创建的六个测试进程及临时bundle已清理，其他会话原有窗口保留。没有保存用户窗口管理器配置变更。

开始时磁盘仅6.7GiB，回收了本助手历轮命名的审计可执行文件/incremental/fingerprint缓存，保留源码、日志、shared libraries和产品release产物，空间恢复到约16GiB后再运行小探针。清理范围记录于[evidence/audit-build-cleanup.json](evidence/audit-build-cleanup.json)，不等于修改产品代码。

最终应按本轮阻断修复后，在同一最终commit重跑release gates并重拍Table状态。CI与155项native绿灯都是真实进展，但当前新反例仍不允许给出发布通过结论。
