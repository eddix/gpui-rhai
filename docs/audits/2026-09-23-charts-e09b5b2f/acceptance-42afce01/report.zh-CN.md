# 第八轮：生命周期闭环验收与联动状态审查

日期：2026-09-24（本轮开始于 09-23）。基线：`42afce01139f73d6cb921f43f036e36adc2e0b9c`，`fix(charts): close lifecycle and viewport state loops`。
参照：[第七轮报告](../release-review-145320d1/report.zh-CN.md)、[整改说明](../release-review-145320d1/remediation.md)。

## 1. 结论

**上一轮 12 个独立探针全部通过，原 R01–R03 的具体复现关闭。生命周期补偿可以按已验证范围验收；完整 Chart Runtime 暂不通过出货验收。**

本轮新增 6 个组合测试：3 个正向控制通过，3 个失败，确认 **1 个 P1、2 个 P2**。三个问题均位于本次变更的 viewport 路径：

| 编号 | 严重度 | 问题 | 可观察后果 |
|---|---|---|---|
| R01 | P1 | 接收联动后的本地手势仍使用过时的 scalar 状态，轴 override 又覆盖 preview | 在目标图放大，preview 不动，确认后反而缩小，并传播给原源图 |
| R02 | P2 | 去重只比较组内 version，丢失 version 所属的组身份 | 从 A 组切到 B 组后持续显示 A 组的旧窗口 |
| R03 | P2 | resize 用“恢复基准视口”覆盖当前有效 preview | 普通布局变更取消正在进行的缩放，Ended 最后提交初始值 |

这次应收敛到 **一个工作包：带完整身份、可组合手势的有效 viewport 模型**。不需要重新打开已经通过的生命周期、数据准备或单场景坐标编译工作包，也不要求扩散成全项目重构。

证据标记：**R** 独立复现；**S** 源码确认；**D** 设计建议；**G** 未认证范围。本轮仅新增本目录中的文字、代码和日志；未改产品代码，未新增图片，未改动已有 UI 文档归并和参考图删除。

## 2. 已经修好的内容

| 内容 | 验证结果 |
|---|---|
| native suspend 失败后的 Chart 更新 | 公开 Active 状态下，后续数据 revision 能正常呈现 |
| Rhai suspend 抛错后的 Chart 更新 | 同样恢复订阅与帧提交 |
| Geo 联动跨 suspend/resume | 源、目标都保留已确认视口 |
| 不同完整域的 Cartesian X/Y 对齐 | 两根实际 mapper 均对齐，不再压缩成一个 zoom |
| 上上轮 frame、时间冻结及双故障对照 | 全部保持通过 |

以上是上一轮原附件 **12/12** 的结果：[日志](evidence/previous-native.log)。

另外三个新增正向控制：

1. **补偿 commit 失败的处置通过。** Rhai suspend 抛错后，在后置 native primitive 的 `commit_resume` 再注入失败；View 进入 Disposed，primitive 卸载，继续发布数据也不再执行 Chart layout，恢复调用返回 DisposedView。实测 `layouts_before=1, layouts_after=1`。
2. **不同尺寸 Geo pan 通过。** Source plot 为 `304×192`，target 为 `184×132`；按 plot 比例计算的目标位置能真实命中。目标 resize 后 plot 为 `324×132`，重新投影的位置仍命中。确认复用了布局层实际 plot，而非整个组件 bounds。
3. **源实例卸载撤回投影通过。** 目标从联动窗口恢复其本地 `[0,100]`，没有永久保留已卸载来源的状态。本项只认证卸载场景，不能代替换组与新来源重建测试。

本轮总计 **18 项：15 通过（12 个旧对照＋3 个新对照），3 失败**。见 [探针](native-probes.rs)、[完整输出](evidence/native-probes.log)、[复现说明](reproduce.md)。

## 3. R01 · P1：联动目标的放大手势被忽略，提交后反而缩小

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 919–939、1105–1114、1158 起；[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) 1222–1233。

**R：** 两个 Cartesian 图表具有相同的 region、axis ID、完整 X/Y 域 `[0,100]`、尺寸、linear/normal 方向。两图都由 Host 接受 `zoom_change` 并写回 proposal revision。无需不同数据域、复杂坐标或异常输入：

1. 在源图放大并确认，目标正确收到联动。
2. 在目标图执行新的 Started 放大手势，观察实际 mapper。
3. 发送 Ended，让目标 Host 接受，再观察两图。

```text
初始已联动窗口： [31.6060279414, 68.3939720586]
目标放大 preview：[31.6060279414, 68.3939720586]  // 完全没动
目标确认后窗口： [19.6734670144, 80.3265329856]  // 跨度反而扩大
原源图最终窗口： [19.6734670144, 80.3265329856]  // 错误扩散到组
```

数据来自 `HostChartSeries` context 的真实 `x_scale.invert`，没有自行重建轴数学。手势 factor 大于 1；相同方向在未联动图表中能正常放大。

**S：** Cartesian 接收端现在只写 `linked_axis_windows`，自己的 `self.zoom` 仍来自原本为 1 的 Host config。后续 wheel 在这个过时 scalar 上累乘；scene 又优先使用 `visible_override`，所以 native preview 被完全覆盖。Ended 发出的 `zoom` 不代表用户刚看到的逻辑窗口；Host 确认后目标成为新来源，以本地 full domain 重新解释 scalar，最终造成反向跳变。

**根因：每轴窗口已成为呈现权威，但输入和 proposal 仍以另一份 scalar 为权威。** 同一时刻出现了两个 viewport 表达，缺少手势开始时的基准快照与统一提交协议。

**修复边界：** 从当前已生效的逻辑窗口／Geo 相机建立 gesture base；将 native delta 作用到该 base，形成显式 preview。Preview、proposal、Host acknowledgement、组投影与最终 scene 必须能够表达同一结果。涉及不同 X/Y full domain 时，一个 zoom/pan 元组未必有能力表达这个结果；必要时直接调整 typed viewport 事件/API，不需要为 pre-1.0 保留历史兼容。若保留 zoom 便利写法，应在输入边界一次转换成统一视口。

**验收：** 本轮探针通过；目标的 preview 真正缩窄，接受后不反向跳变，源图收到同一窗口。覆盖两图交替成为来源、拖动 pan、Host 接受／拒绝／延迟确认，以及不同 X/Y full domain。不能仅在 Ended 清空 override，因为手势期间的 preview 与提交基准同样需要正确。

## 4. R02 · P2：不同组的同号 version 被误认为同一个投影

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 114–118、238–246、257–267、903–912。

**R：** 同一个 View 中放三个 Chart：A 属于 `group_a`，B 属于 `group_b`，目标初始属于 A。A/B 分别确认一次不同缩放，因此各自注册表记录都是 version 1。保持同一个目标组件 key，仅通过正常 Rhai 状态更新把它改到 B 组：

```text
A 组窗口：       [19.6734670144, 80.3265329856]
B 组窗口：       [31.6060279414, 68.3939720586]
目标换组前窗口： [19.6734670144, 80.3265329856]
目标换组后窗口： [19.6734670144, 80.3265329856]  // 仍是 A
```

两组已各自生效、目标确实更新了 spec，数据准备和重新布局也已运行完；等待更多普通刷新不会自动纠正。

**S：** 注册表 version 在每个 `ChartLinkKey` 内独立递增。送给目标的 `ChartLinkedProjection` 只保留 `version + viewport`，没有携带所属 group/domain 或不可混淆的提交身份。`apply_linked_projection` 比较到 `Some(1)==Some(1)` 就提前返回，连新 payload 都不保存。目标的新 spec 和旧投影由此混用。

**修复边界：** 版本相等判断需要带作用域。可使用 group/domain＋组代际＋提交序号，或等价的全局唯一提交身份；选择清晰的身份规则即可。处理删除／重建组、替换来源、换组时的去重，并把目标坐标绑定变化纳入重新投影条件。不要依赖用户给目标换 key、先退出一帧再入组或再滚一次来源来刷新。

**验收：** A→B 同号 version 当帧取得 B 投影；相同投影的普通 redraw 仍不重建；来源卸载／重建后，旧身份不能吞掉新提交。源卸载正向控制继续通过。

## 5. R03 · P2：普通 resize 清除活动 preview，Ended 提交错误基准

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 834–842、919–925。

**R：** 使用单张**未加入联动组**的 Cartesian 图，Host 正常接受 proposal。Started 手势产生放大 preview；未发送 Ended 时，由 Host 状态把组件宽度从 280 改为 420，随后才发送 Ended：

```text
initial  = [0,100]
preview  = [19.6734670144,80.3265329856]
resized  = [0,100]
committed= [0,100]
```

这是单纯的布局尺寸变化，没有 suspend、Host 拒绝或 link 状态变更。测试通过公开 UI 控件改变尺寸，不接触 Chart 私有状态；实际应用中可由侧栏、分栏、响应式布局等触发。

**S：** 为让 Geo camera 在新 plot 下重投影，本轮在所有 `set_bounds` 尺寸变化时调用 `restore_effective_linked_viewport`。该 helper 首先无条件执行 `self.zoom=config.zoom; self.pan=config.pan`，然后才判断有没有 linked projection。非联动图表也因此丢掉当前 preview；手势 dirty/generation 仍保留，Ended 最后把重置后的值当作用户 proposal 提交。

**修复边界：** resize 应重新投影当前有效逻辑视口，而不是借用取消手势／恢复 committed 基准的状态迁移。把“丢弃 preview”和“为新尺寸生成几何”分开。只对 `linked_projection.is_some()` 加条件能够修当前非联动例子，但不能证明 Geo 手势、联动目标 preview 和待确认 proposal 在 resize 后正确。

**验收：** 非联动 Cartesian、Geo 和联动目标均覆盖 Started→resize→Moved→Ended；连续 preview 不丢失，proposal 与当前有效结果一致；前述不同尺寸 Geo 正向控制继续通过。

## 6. 结构判断与有限整改方案

上一轮要求的两项基础设施已落地：注册表持久化了 viewport，每根 Cartesian 轴也能直接接收逻辑窗口。此次三个失败说明还缺少它们与输入状态的组合规则：

| 事实 | 当前情况 | 下一步 |
|---|---|---|
| 呈现帧身份与后台提交 | 原 frame 探针保持通过 | 保留 |
| 生命周期恢复／补偿 | 原失败路径及新增 commit 失败处置通过 | 按本轮范围验收，不继续扩大重构 |
| 已确认组状态 | 有持久化 source/version/payload | 给投影保留完整提交身份，关闭 R02 |
| 逻辑轴窗口 | 可独立驱动 X/Y mapper | 将其作为手势、proposal 的同一基准，关闭 R01 |
| Geo 布局投影 | 使用实际 plot，变尺寸验证通过 | 保留几何方法，分离尺寸变化与取消 preview，关闭 R03 |

**D：建议集中定义一个 effective viewport 的计算／迁移入口。** 可实现为小型纯状态 reducer 或现有 controller 内集中方法，名称和文件拆分不是验收要求。输入应明确区分 Host committed viewport、带身份的 link projection、gesture base/preview、pending proposal/ack、当前 coordinate plan 与 bounds；输出统一的有效逻辑视口及需要失效的 FrameKey。

建议先把以下迁移写成规则，再实现。它们足以覆盖本轮风险，不要求枚举所有组件组合：

| 事件 | 必须保持的性质 |
|---|---|
| 首次收到 link / 换组 / 换来源 | 依据完整身份选择投影，不能仅比较数字 version |
| 在 linked target 上开始手势 | 从当前已呈现或已明确生效的窗口开始；不能回到过时 Host scalar |
| 手势中收到新 link、Host ack | 明确抢占、排队或 rebase 规则；不能由 render 顺序偶然决定 |
| Host 接受／钳制／拒绝／延迟 ack | proposal 身份与后续 preview 分开，保持此前已通过的受控协议性质 |
| bounds 变化 | 重新投影有效视口；不隐式取消或确认手势 |
| suspend | 明确取消 transient 手势，保留 committed/link；本轮旧对照保持通过 |
| 来源卸载、离组 | 按契约撤回其状态；目标本地 committed 状态仍可恢复 |

不要分别为三个复现加互不相关的条件分支。R01/R03 共享“有效视口来源和转换时机”的根因，R02 是同一状态对象的身份缺失。关闭这一工作包即可进入下一次有限验收，不需要重新设计 NativeChartData、Rhai 执行或整个 Chart scene。

## 7. 自动验证与 release 证据

| 检查 | 本轮结果 | 日志 |
|---|---|---|
| Workspace 全目标、全 feature、locked/offline | 561 通过 | [workspace](evidence/workspace.log) |
| 原生独立包 | 94 通过 | [native](evidence/native.log) |
| Performance 默认套件 | 4 通过、3 ignored | [performance](evidence/performance.log) |
| 上轮原探针 | 12 通过 | [previous](evidence/previous-native.log) |
| 本轮组合探针 | 15 通过、3 失败 | [probes](evidence/native-probes.log) |
| fmt、workspace 严格 Clippy、native 严格 Clippy | 通过 | [fmt](evidence/fmt.log)、[clippy](evidence/clippy.log)、[native-clippy](evidence/native-clippy.log) |
| Target inventory | 通过 | [targets](evidence/target-manifest.log) |
| 最终 commit 的完整 release smoke | 通过 | [smoke](evidence/release-smoke.log) |
| release artifact 路径／inspector 审查 | 通过 | [artifacts](evidence/release-artifacts.log) |

环境仍为 Macmini9,1、macOS 26.6.2、rustc 1.94.0、GPUI 0.2.2、Rhai 1.26.0；完整身份和命令结果见 [metadata](evidence/metadata.json)。本轮没有产品 dirty files，已有 UI 文档清理仍未改动。

### 性能：记录当前候选，不沿用旧 commit 的采样身份

对 `42afce01` 执行了 100k release 基线，每次 5 次预热、30 次测量。首轮 p95 高于上一轮记录，因此追加一次同参数复测；两个结果均保留：

| 指标 | 本轮第一次 | 本轮复测 | 145320d1 旧参考 |
|---|---:|---:|---:|
| Resize p50 / p95 | 27.001 / 28.161 ms | 29.443 / 36.932 ms | 25.940 / 27.384 ms |
| 128 行 sliding update p50 / p95 | 83.803 / 96.240 ms | 85.574 / 103.737 ms | 80.864 / 82.230 ms |
| Streaming 后 Rhai operations | 0 | 0 | 0 |

[第一次日志](evidence/chart-release-30.log)、[复测日志](evidence/chart-release-30-repeat.log)。两个 run 都通过目标 data revision 已呈现的断言。

这些是 TestAppContext 的完整 settle 测量，不是实际显示器延迟。**本轮观测值较旧参考慢且有波动；没有同一时段、相同电源／热状态的双 commit A/B，不能把差异归因为此次补丁，也不能据单样本宣称无性能回退。** 不另列未经归因的性能缺陷。若发布需要延迟门槛，应在受控环境做候选与参考的交替 A/B，并预先定义阈值。整改说明中上一轮 30 样本可以作为旧参考，不能当作已经在本 commit 执行的测量。

## 8. 验收终点与边界

当前发布阻断项是 R01–R03；下一轮以三个失败探针转绿、上述有效视口状态迁移矩阵有正式回归覆盖，以及已有对照保持通过为关闭条件。实机主题／RTL／键鼠／VoiceOver 等发布体验矩阵仍按原清单执行；本轮未重复认证这些 **G** 范围，也未将它们计作新 bug。

本轮对 core 模型的判断已收窄：**生命周期与每轴坐标表达有可复验的进步，剩余核心问题集中在有效视口的身份与交互转换。** 修复应围绕这一事实的唯一权威和迁移规则展开。
