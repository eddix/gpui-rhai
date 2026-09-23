# 第七轮：Chart Runtime 出货审计

日期：2026-09-23。产品基线：`145320d17d43fd10cfe5150495fd43cfa7d90381`，`refactor(charts): unify frame lifecycle and link models`。
参照：[上一轮核心审查](../core-review-a31da5a8/report.zh-CN.md)、[本轮整改说明](../core-review-a31da5a8/remediation.md)。

## 1. 出货结论

**暂不通过完整 0.1.5 Chart Runtime 出货验收。** 上一轮的 8 个独立原生探针全部通过，本次重构确实修复了原报告的具体复现；常规测试与 release smoke 也全部通过。但组合验收又确认了 **3 类正确性问题：1 个 P1、2 个 P2**，分别涉及失败补偿、联动状态保留和联动坐标表达。

这次不是要求继续无边界地重构。建议把整改收敛为两个工作包：

1. **生命周期闭环**：恢复提交必须覆盖正常 resume 和失败 suspend 的补偿；成功返回到 Active 必须意味着 Chart 也恢复活动。
2. **联动视口闭环**：联动结果作为有来源和版本的状态保留，以逻辑窗口驱动目标坐标计划，不能继续只修改一组临时 zoom/pan。

通过这两个工作包及文末的有限验收矩阵后，再完成已有 ADR 要求的发布体验矩阵，即可重新判断出货。没有证据要求推翻 Rust/Rhai 分工、替换 GPUI、增加兼容层或拆成更多 crate。

本轮仅增加审计文档、探针和文本日志。未改产品实现，未提交代码，未新增图片；原有 UI 文档归并和三张参考图删除保持不变。以下不把 pre-1.0 breaking change 当作 bug。

证据标记：**R** 独立行为复现；**S** 源码确认；**D** 改进建议；**G** 本轮未认证的范围。

## 2. 上一轮整改是否有效

| 上轮问题 | 本轮独立复验 | 判定 |
|---|---|---|
| Prepared 已完成、presented 尚未安装时暂停 | 原 presented=1，源 revision=2；恢复后 presented=2，重新完成布局 | 原复现关闭 |
| 取消 preview 后仍保留 preview 图像 | 恢复后 committed 位置命中，preview 位置不再命中 | 原复现关闭 |
| 恢复失败且补偿再次失败 | View 进入 Disposed，已登记测试资源全部卸载 | 原复现关闭 |
| 失败 resume prepare 推进动画时间 | 失败准备推进 60 秒后重试，仍能命中冻结位置 | 原复现关闭 |
| 同图域、同 map/projection 的 Geo 联动无效 | 源 zoom=2.71828，源图与目标图都离开旧几何 | 原复现关闭 |
| 单 hook 失败重试、正常动画冻结等原对照 | 全部通过 | 保持通过 |

完整原附件重跑：**8/8**，见 [日志](evidence/previous-core-native.log)。没有把这些已通过的原始复现重新记为“未修复”。

本轮新增 4 个失败断言，加上原 8 个对照，共 **8 通过、4 失败**；4 个断言归并为以下 3 类问题。见 [探针](native-probes.rs)、[最终日志](evidence/release-native.log)、[复现说明](reproduce.md)。

## 3. R01 · P1：暂停失败的补偿没有执行恢复提交，Active 图表持续停止更新

位置：[app.rs](../../../../crates/gpui-rhai/src/app.rs) 4108–4123；[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 477–505。

**R：两个独立入口都复现。**

- 原生 primitive 的 suspend 发生一次受控失败，其余资源成功补偿。
- 普通 Rhai `suspend(ctx)` 抛出错误，不依赖原生扩展 panic。

两者都先正常呈现 revision 1，再请求暂停。暂停返回错误、公开 View 保持 Active，随后 Host 发布 revision 2 并充分运行原生 executor：

```text
native suspend failure: view=Active before=1 wanted=2 presented=1 layouts=1
script suspend failure: view=Active before=1 wanted=2 presented=1 layouts=1
```

测试 primitive 的 active flags 已全部恢复，但真实 Chart 不再订阅和呈现后续更新。Host 此时调用 `view.resume()` 也只会因 View 已为 Active 而返回 `Ok(false)`，不会修复 Chart 的活动状态。

**S：根因是本轮两阶段恢复没有覆盖反向补偿。** Chart 的 `resume()` 现在只把状态改为 `PreparingResume`；订阅、时间恢复和待处理帧启动都移到了 `commit_resume()`。正常 `resume_view` 增加了 commit，`suspend_view` 的 native 失败和 script 失败分支仍然只调用 `resume_mounted`。因此公开 Active 与 Chart 的 PreparingResume 分裂。

**归因：本轮引入的回归。** 旧的“一阶段 resume”测试 primitive 在 `resume()` 内直接 active=true，无法发现采用新契约的 Chart 卡住。新接口的正常路径通过，不等于所有回滚路径已经迁移。

**修复边界：** 由生命周期协调者提供唯一的“恢复到已提交 Active”操作，让正常恢复及暂停补偿复用完整 prepare/commit 序列；任何必要阶段失败都进入明确的补偿或处置策略。不要让某个错误分支直接调用 Chart 私有函数，也不要在 Chart.render 中偷偷激活来掩盖状态错误。顺带核对 `!changed` 分支，避免它继续绕过同一协议。

**验收：** 本轮两个断言通过；增加真实两阶段 primitive 的补偿测试、补偿 commit 失败后的处置测试。失败暂停返回 Active 后，原生数据更新、输入和活动时钟都可继续工作；普通失败恢复仍不能消耗隐藏时间。

## 4. R02 · P2：已确认的联动视口在暂停／恢复后只在目标图丢失

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 213–229、467–470、830–861。

**R：** 直接复用已通过的 Geo 联动 fixture：两图位于同一个 View、同一个 group/domain、同一个 region/map/projection；源图 Host 明确接受 zoom 和 proposal revision。先证明两图都已移动，然后对整个 View 做一次成功的 suspend/resume：

```text
暂停前：source_hits|target_hits|zoom = 1|1|2.718281828459045
恢复后：source_hits|target_hits|zoom = 1|2|2.718281828459045
```

计数表示点击各自最初的旧位置：源图保持已缩放状态，所以旧位置不命中；目标图恢复后却重新命中旧位置。不是 Host 拒绝 proposal、不同 projection，也不是仅仅没有重绘。

**S：根因是 linked 状态仍与本地 preview 共用 `self.zoom/self.pan`。** 接收联动直接覆写这两个字段，但不保留逻辑联动状态。暂停无条件把它们还原到该图自己的 `config.zoom/config.pan`。目标 Host 从未被要求写入联动值，因而仍是 1/0/0；源图则有已确认的 Host 状态。LinkRegistry 只广播 viewport，没有可恢复的 viewport 记录；恢复也没有重新投影来源。

FrameKey 正确检测到了差异并重建了目标图，但重建使用的是丢失联动后的输入。**正确的帧失效机制不能弥补错误的有效视口来源。**

**归因：上轮要求分离 committed/preview/linked 状态的结构工作尚未完成；本轮新增 Geo 支持使该组合可以被独立验证。** 原来的“Geo 完全不联动”已经关闭。

**修复边界：** 保存带来源/版本的 linked viewport 投影，与 Host 本地 committed viewport、当前 gesture preview 分离；统一计算 effective viewport。Suspend 只取消未提交手势，不应撤销已生效的组视口。为来源离组/卸载、目标重入、Host 显式覆盖及多来源竞争规定一致策略，防止重新广播形成循环。

**验收：** Geo 和 Cartesian 都覆盖“确认联动→普通 redraw→暂停／恢复→数据更新”；目标保持与组状态一致，且不要求向每个目标重新发送 Rhai 回调。退出组和 Host 覆盖后的行为也必须可预测。

## 5. R03 · P2：Cartesian 两个逻辑窗口被压缩成单个 zoom，不同完整域时 Y 轴错误

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 868–918；现有单轴 helper 测试在 2704 起。

**R：** 两图同 region/axis ID，全部 linear/normal，相同尺寸且没有 pan。初始完整域：

| | X full | Y full |
|---|---|---|
| 源图 | 0…100 | 0…100 |
| 目标图 | 0…200 | 0…100 |

通过真实滚轮和 Host acknowledgement 把源图 zoom 改为 2.71828。纯 Rust custom series 只记录框架交给它的真实 `x_scale/y_scale`，使用公开 `invert` 读取 plot 两端的逻辑值，不参与坐标计算：

```text
source X = [31.6060279414, 68.3939720586]
source Y = [31.6060279414, 68.3939720586]
target X = [31.6060279414, 68.3939720586]  // 正确
target Y = [40.8030139707, 59.1969860293]  // 错误：窗口宽度减半
```

**S：** payload 虽然区分 X/Y，但 `apply_linked_viewport` 优先用 X 算唯一 zoom：目标 X 全域是源图两倍，需要 zoom≈5.43656。这个 zoom 又用于目标 Y，尽管目标 Y 的全域与源图相同。pan 只能改变中心，无法恢复被压缩的 Y 跨度。现有 helper 测试分别证明单根轴可 round-trip，没有证明两个轴可以同时兑现窗口。

**归因：旧的单 scalar 接收模型仍被保留，本轮类型枚举没有解决它的表达能力。** 不是 Rust enum 本身有错，也不涉及 API 历史兼容。

**修复边界：** 目标坐标编译应直接消费每根绑定轴的逻辑 visible window，或使用足以独立表示每轴窗口的内部模型。不要先接收两个区间，再降维成单一缩放比例。若产品决定只联动 X，必须在 API/绑定契约中明确选择，并保留目标未绑定轴；当前默认发送并接收 X/Y 的行为不能被解释为仅联动 X。

**验收：** 同时比较 X/Y 的真实 mapper，覆盖不同 full domain、不同尺寸、normal/reversed、linear/log 和单轴绑定。除了区间相等，还要验证 marks、ticks、hit testing 与 mapper 一致。源数据或目标数据改变 full domain 后，应重新兑现逻辑窗口。

## 6. 核心模型复评

| 模型 | 当前判断 | 本轮出货影响 |
|---|---|---|
| NativeChartData、语义数据／绘制采样 | 延续原有结构；正式测试通过 | 没有新增阻断证据 |
| 单 scene 坐标编译 | series/ticks/annotations/custom scale 继续共享计划 | 保留；问题在 scene 外的联动输入 |
| Requested/Prepared/Presented key | source epoch、data revision、frame epoch 及安装 guard 是实质改进；两项原 frame 探针通过 | 不要求为类名或文件拆分再重构 |
| 生命周期失败处置 | 补偿双故障进入 Disposed，原复现关闭 | 保留终止策略；R01 要补完整反向迁移 |
| 活动时间 | commit_resume 使失败 prepare 不再消耗 Chart 时间 | 原 C03 关闭；没有必要仅为统一命名强制换时钟实现 |
| 有效 viewport 所有权 | Host、preview、linked 仍共用局部变量 | R02，属于结构缺口 |
| 坐标类型与联动 | Geo/Cartesian 已分类并校验部分身份；目标仍用全局 scalar，发送端仍挑首 region/首匹配轴 | R03；能力边界需跟真实表达能力一致 |

**D：最需要改进的是状态序列测试。** 每个模型单独成功的测试越来越多，但跨模型序列仍缺：`Active→暂停失败→补偿→数据更新`、`联动提交→Suspend→Resume`、`不同 X/Y full domain→联动提交`。下一轮应把这些性质迁入正式测试，不应只增加又一批局部 helper 断言。

**D：FrameKey 与性能观测应继续接通。** 公开 AX 已提供 source/frame epoch，但性能 harness 对 streaming 只断言数据 revision，对 resize 仍用固定 settle 流程，没有断言期望尺寸对应的新 frame 已安装。建议将 FrameKey/布局结果作为终点条件。当前 30 样本结果可以作为现有 v2 基线，不应据此宣称所有 resize 失效都被测试识别。

**S/D：文档小项，可与修复一起完成：** `release-checklist.md:63` 仍写 chart-e2e-v1，实际 harness/性能文档为 v2；`ScriptViewHandle::resume` 的 rustdoc 仍绝对承诺失败后 Suspended，漏掉本轮新增的 Disposed 故障策略。这些不另列产品严重缺陷。

## 7. 已执行的发布检查

环境：Macmini9,1、macOS 26.6.2、rustc 1.94.0、GPUI 0.2.2、Rhai 1.26.0。详情见 [metadata](evidence/metadata.json)。

| 检查 | 本轮结果 | 证据 |
|---|---|---|
| Workspace 全目标、全 feature、locked/offline tests | 561 通过 | [workspace](evidence/workspace.log) |
| 独立原生包 | 92 通过 | [native](evidence/native.log) |
| Performance 默认套件 | 4 通过，3 ignored | [performance](evidence/performance.log) |
| 上轮核心原附件 | 8 通过 | [previous](evidence/previous-core-native.log) |
| 本轮组合探针 | 8 对照通过，4 新断言失败，归并为 3 类 | [new](evidence/release-native.log) |
| Workspace fmt、严格 Clippy、原生独立包严格 Clippy | 通过 | [fmt](evidence/fmt.log)、[clippy](evidence/clippy.log)、[native-clippy](evidence/native-clippy.log) |
| 默认 feature 的 core all-target check | 通过 | [default](evidence/default-feature-check.log) |
| target inventory、workspace rustdoc | 通过 | [targets](evidence/target-manifest.log)、[rustdoc](evidence/rustdoc.log) |
| 完整 scripts/release-smoke.sh | 通过，含 Chart Gallery、所有示例、Theme Studio、Table 状态 | [release smoke](evidence/release-smoke.log) |
| release 二进制路径／inspector 审查 | 通过 | [artifacts](evidence/release-artifacts.log) |
| 已有视觉 baseline 文件审查 | 38 张 PNG 格式／尺寸／数量有效 | [inventory](evidence/visual-baseline-inventory.log) |
| 三个 crate 的 cargo package --list | 成功，列表已检查；本轮审计附件／参考图不进入 crate 列表 | [core](evidence/package-core-list.log)、[registry](evidence/package-registry-list.log)、[CLI](evidence/package-cli-list.log) |

`cargo package --list` 不是 clean package rebuild；视觉 baseline 脚本没有 Chart 用例，也不比较当前渲染；release smoke 检查启动和存活，不证明每个画面与交互正确。这些结果按各自真实覆盖范围计入。

## 8. 100k release 性能证据

执行实际 release harness：5 次 warmup、30 次 measured sample、100,000 点、每次滑动更新 128 行；streaming 每次断言目标 data revision 已呈现。

| 指标 | 结果 |
|---|---:|
| Gallery prepare（一次冷启动） | 129.610 ms |
| Mount＋terminal first frame（一次冷启动） | 122.404 ms |
| Resize p50 / p95 | 25.940 / 27.384 ms |
| 128 行 sliding update p50 / p95 | 80.864 / 82.230 ms |
| Streaming 后 Rhai operations | 0 |

[完整日志](evidence/chart-release-30.log)、[JSON](evidence/chart-release-30.json)。Prepare/first frame 不是 30 次冷启动的分位数；30 次采样适用于 resize 和 streaming。

这是 GPUI TestAppContext 下的端到端 settle 测量，不是物理显示器 input-to-photon 或 GPU frame-time。没有给项目自行发明一个“超过 16 ms 就不能发布”的阈值，也没有把 Rhai operations=0 宣称为所有前后台开销为零。测试与仓库原短样本参考量级相近，未发现足以另列性能回归的问题。

## 9. 有限整改与发布门槛

按顺序完成以下工作；此处没有将所有 D 建议升级为强制重构。

1. **关闭 R01**：统一恢复提交入口；两个失败 suspend 行为探针通过，补偿 commit 失败可处置；正常 suspend/resume、失败 resume 时间冻结和双故障处置对照保持通过。
2. **关闭 R02/R03**：保留逻辑联动状态并直接进入每轴坐标计划；当前 Geo 恢复和 Cartesian 双轴探针通过，并补组生命周期／不同域的正式性质测试。
3. **在最终候选 commit 重跑必要自动门禁**：workspace/native/performance 结构测试、fmt/Clippy、release smoke；涉及几何或执行成本时重跑对应 release benchmark。无需为文档措辞重复所有性能测试。
4. **完成原有发布契约中的体验和分发验收**，记录精确 commit、环境、矩阵结果和已知平台限制。

第 4 项的 **G** 边界：本轮未做完整 Chart 各系列×主题/Host override×locale/RTL×窗口尺寸×Normal/Reduced/None 的实机视觉与交互签收，也未做实际 VoiceOver 验收、持续高频 producer 的资源峰值认证；不能以 scene 单测或 38 张旧 baseline 代替。完整 release checklist 的第二工具链、全依赖 license matrix、clean package rebuild／发布后的 crates.io 安装等也没有在本轮执行。发布后的步骤应由正式发布流程完成，不是要求现在发布验证。

这些 G 项是既有契约及本轮证据边界，不算新发现的 bug。若决定缩小首次公开能力，应明确修改产品契约并拒绝不支持的绑定；当前文档承诺下，不能把错误联动或停止更新当作已知限制直接出货。

**可出货判定条件：3 类 R 问题关闭，自动门禁保持通过，既有发布矩阵有候选版本的可追溯签收。** 不要求开发 agent 把本报告所有建议类名逐字落地，也不以“继续没有新发现”为唯一、不可终止的标准。
