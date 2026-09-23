# 第三轮图表整改验收

日期：2026-09-23。基线：`a8d2ef0f8b90df4915d3150941ee49baa9dd23bd`（`fix(charts): close second acceptance gaps`）。
本轮复验 [第二轮报告](../acceptance-16cf8e51/report.zh-CN.md) 的 R01–R06，并检查 [本次整改说明](../acceptance-16cf8e51/remediation.md) 与实际行为是否一致。

**结论：上一轮的主要复现已修复，整体仍暂不通过验收。** 这轮确实解决了无效缩放、空闲联动循环、Geo viewport、重复 mark identity、空聚合 schema 和 Gauge 业务键等问题。独立测试也确认了这些进展，不再重复将其作为原样未修的缺陷。

新的组合测试确认 **4 组 P1、2 组 P2**：类别数据越过自动采样阈值会消失；晚到的确认会清掉新手势；正式适配器缺失新版受控字段；多轴计划仍受空组和轴名字排序影响；反向轴 pan 方向错误；导出 selection 仍会误改图例。这里包含新引入的回归、先前未覆盖的组合，以及跨后端残留，不声称全部由本提交首次引入。

R＝独立复现；S＝源码确认；G＝验证缺口。接受 `viewport_revision` 和结构 key 的有意 breaking change；本轮问题不是要求历史兼容。

审计未修改产品实现。工作树中此前的 UI 文档归并、临时规格/参考图片删除保持原样；本轮只新增此目录。图表源文件与 registry/charts 在验收期间均保持基线提交内容。

## 1. 验证概况

| 检查 | 结果 | 边界 |
|---|---|---|
| Workspace 全目标、全 feature、locked/offline | **555 通过** | 包含新增 transform 与 chart_contract 回归 |
| 原生独立包 | **80 通过** | 其中图表原生测试 11 项 |
| performance 默认检查 | **4 通过，3 ignored** | 显式 benchmark 另行执行 |
| fmt / 严格 Clippy | **通过** | 没有产品静态检查失败 |
| 独立原生探针 | **6 通过、3 失败** | 旧主复现通过，新组合仍失败 |
| public 探针 | 程序执行完成 | 记录正反结果，exit 0 不等于行为通过 |
| 100k v2 benchmark | debug 单样本通过 | 含目标 revision 断言，不是 release 性能认证 |

独立原生通过项：有效缩放 proposal、跨帧 preview、phase-less 滚轮、Gauge 源数据 key、联动空闲稳定、来源卸载撤销 linked selection。
失败项：正式 BarChart 的 viewport_revision、旧 acknowledgement 与新手势交错、Started 手势在 Ended 前提前提交。

本机 debug 单样本 prepare≈912ms、mount≈464ms、resize≈187ms、streaming≈345ms，stream 后 Rhai operations=0。此处不据单样本做分位数或跨版本性能结论。完整 release smoke、30 样本 release、真实窗口主题/RTL 与 VoiceOver 本轮未重跑。

附件：[复现命令](reproduce.md)、[public 输出](evidence/public.log)、[原生输出](evidence/native-probes.log)、[常规测试](evidence/workspace-tests.log)、[原生套件](evidence/native-tests.log)。本目录不新增参考截图。

## 2. 第二轮 R01–R06 的验收状态

| 原编号 | 已通过的实测 | 状态 |
|---|---|---|
| R01 手势与确认 | 40px 滚轮 proposal≈0.904837；重复拒绝后仍从受控值提案；跨帧保持 preview；Lines/Moved 可以提交 | **主复现通过，协议仍有交错漏洞**：T02；正式入口未齐：T03 |
| R02 联动循环 | linked=false/true 均是 layout count 2→2；64 次保护不再触发；移除选中来源后目标高亮撤销 | **本轮覆盖通过**，不再列为未修复 |
| R03 viewport/轴 | 显式 min/max 的 zoom 改变 marks/ticks；正值 log zoom-out 合法；Geo 几何随 viewport 改变；Auto category 保留 Alpha/Beta | **主复现通过**；反向 pan 另见 T05，多轴计划另见 T04 |
| R04 mark identity | 多 Y 轴 annotation 不再因重复 key 报错；默认/显式轴引用归一；业务 key `line:0` 可正常绘制 | **碰撞主复现通过，布局含义未完全闭合**：T04 |
| R05 empty/schema/sample | 空过滤可布局；空 aggregate 保留 2 列；threshold=3 的折线仍保留语义域 -500…1000 | **主复现通过，类别输入新回归**：T01 |
| R06 业务身份 | Gauge 原生回调返回 measurement-1；Map 的几何 mark 引用与源数据 key 一致；runtime selection 过滤 Data role | **主复现通过**；export 的 role 过滤遗漏见 T06 |

同轮收口也有实际进展：Cartesian/Polar/Geo 自定义 renderer 返回错误时现在一致返回 Err；circle stroke 使用实际 border 绘制；cache 命中不再深克隆整份 inline rows，snapshot 使用 Arc 身份判等。

这里的“本轮覆盖通过”不等于全主题、所有组合和真实辅助技术全部认证。例如 source cache 仍深比较 props；性能收益需实际计数/测量，而非从 Arc 的存在直接推断常数成本。

## 3. T01 · P1：类别折线从 4000 行增至 4001 行后完全消失

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `prepare_chart_data` 自动采样分支（约 612–638）；[transform.rs](../../../../crates/gpui-rhai/src/chart/transform.rs) `downsample_lttb`（565 起）。

**R：** 相同结构的合法数据：`id="rN"`、`x="CN"`、`y=N % 7`，Line 自动使用类别 X 轴。

```text
4000 rows: data_marks=4000, line_parts=1, semantics=4000
4001 rows: data_marks=0,    line_parts=0, semantics=4001
diagnostic: draws 0 of 4001 rows
```

数据规模仍在公开 inline 10k 上限内，没有非法值。超过 4000 后进入自动 LTTB；该算法用 `as_number()` 同时读取 X/Y，把全部 String X 当成缺失段。此次修改将“没有有效段”改为返回空 dataset，于是业务数据全被抹掉。

这不是需要连接 null gap 的特殊策略，而是**采样器没有按坐标类型选择合法的输入表示**。数值 X 的 LTTB 修复不能直接泛化到类别 X。

**修复：** 先在已解析坐标计划中判断采样适用性。类别折线可以按稳定类别序号采样，同时保留原始 category/key，也可以使用明确的类别采样策略；未支持的绘制优化应跳过或提供明确诊断，不能把有效类别解释成 null。null gap 与不支持的类型必须区分。

**验收：** 3999/4000/4001/10000 行类别 Line/Area 的存在性与量值语义一致；包含 null 时仍保留 gap；改变采样策略不改变 category 身份和语义域；全部被丢弃必须是实际无有效数据的结果。

## 4. T02 · P1：旧提案确认会擦除正在进行的新手势

位置：[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `update_config`（338–421）、`wheel` / timer（736–770）、`emit_viewport_change`（772–803）。

显式 revision 是正确方向，但它目前只标记 pending proposal，没有标记该 proposal 之后产生的 preview。

**R：** 用正式 Rhai Chart 入口确定性复现以下顺序：

1. 普通滚轮产生 proposal 1，zoom≈0.960789；Host 暂不确认。
2. 新手势 Started 再输入 40px，产生新的未提交 preview。
3. Host 此时接受 proposal 1，回写该 zoom 与 revision=1。
4. 新手势 Ended。

实际最终仍只有 **1 次回调，zoom≈0.960789**，第二次输入完全丢失。测试中的公开状态动作只用于确定性模拟迟到的 Host 回写，不要求产品提供“确认按钮”。

根因是 acknowledge 后无条件重设 `zoom/pan`、清空 `viewport_preview_dirty`、取消 wheel timer；没有判断当前 preview 是否已经包含 proposal 1 之后的新输入。正确接受旧 proposal 不应同时接受或拒绝尚未提交的新手势。

**另一个 R 级提交时机问题：** 明确发出 `TouchPhase::Started`，短暂停顿、尚未 Ended，就得到 `callbacks=1`。80ms fallback timer 对显式分阶段手势也生效。当前正式回归测试在 Started/Ended 之间 pump，却只检查最终数值，没有检查提前提交，因此继续通过。

**修复：** proposal snapshot 与 active gesture/preview generation 分开保存。确认只结算对应提案；较新的 preview 应保留或在新 committed base 上重算其 delta。记录是否处于明确 Started 的手势，Ended/取消走明确协议；静默合并仅用于无可靠结束事件的输入，不能把短暂停顿等同于所有手势结束。

**验收：** proposal 1 pending→gesture 2 preview→ack 1→end 2 的两次输入都生效；接受、拒绝、钳制、延迟和旧回复次序有明确结果；显式 gesture 的 Ended 前无 commit；无 phase 输入仍有有界提交，空 Ended 不产生事件。

## 5. T03 · P1：正式 Bar/Line/Pie/Map 适配器无法使用新的确认协议

位置：[bar_chart.rhai](../../../../registry/charts/bar_chart.rhai)、[line_chart.rhai](../../../../registry/charts/line_chart.rhai)、[pie_chart.rhai](../../../../registry/charts/pie_chart.rhai)、[map_chart.rhai](../../../../registry/charts/map_chart.rhai) 的 props schema；新字段仅加入 [chart.rhai](../../../../registry/charts/chart.rhai)。

**R：**

```rhai
import "charts/bar_chart" as bar;
bar::BarChart(#{
    key: "c", key_dimension: "id", viewport_revision: 1, zoom: 1.0,
    data: [#{ id: "a", x: "A", y: 1 }], encode: #{ x: "x", y: "y" }
})
```

挂载失败：`props for component charts/bar_chart are invalid: $.viewport_revision: unknown field`。

**S：** 另外三个正式适配器也没有此字段。`chart::BarChart` 等通用模块便捷函数会转发 revision，但 CLI 安装得到的正式 `charts/bar_chart` 入口先经过更外层严格 schema，两者行为不同。上轮适配器测试仍只覆盖便捷函数路径。

因此调用者可以接收 proposal，却无法在正式组件上按新文档写回确认；这不是老版本兼容问题，而是**本版本两种官方入口的公共契约没有一起升级**。

**修复：** 同步四个正式 schema 和默认值/转发路径，核对 callback payload；尽量共享或生成共同字段，避免再次手工漏抄。不要新增静默丢弃 revision 的兼容分支。

**验收：** 对四个正式组件以及通用 Chart/便捷函数分别测试省略、初始化、接受、同值拒绝和程序写入 revision；不能只验证解析或 export 函数名存在，要通过真实 mount 和 callback 回写完成闭环。

## 6. T04 · P1：轴/标注的绘制仍由系列分组的遍历顺序决定

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `layout_cartesian`（1104）、`layout_cartesian_group` 空组提前返回（1189 起）、`layout_cartesian_annotations`（1642）。

**R：两个独立复现。**

第一组使用相同 X、两个 Y：左轴数据 0…10，右轴 0…1000，baseline value=5。只重命名轴 ID，并同步对应 series 引用：

```text
left=a_left, right=z_right → baseline y=185.00
left=z_left, right=a_right → baseline y=344.39
```

数据、位置、轴声明顺序和 annotation 值均未改变，但标注从左轴中点移到右轴底部附近。原因是 BTreeMap 轴对排序后的第一个 group 获得 `render_annotations=true`，annotation 没有明确坐标轴绑定。

第二组有一个空数据系列绑定共享 X/左 Y，另一个非空系列绑定相同 X/右 Y。结果仍有 **2 个数据 mark，但 X 轴标签为 0，annotation 为 0**。`drawn_x.insert` 和 `drew_annotations=true` 在确认 group 真正产出轴/标注之前就更新；第一个空组提前 return，把后续绘制资格消耗了。

上次修复消除了 duplicate key 异常，但“只在第一个组画一次”没有形成正确的坐标所有权。

**修复：** 轴与 annotation 从 series-pair 绘制循环中解耦。先解析唯一轴计划、明确主轴或 annotation 的 axis reference，再独立生成它们。若暂不支持某种多轴标注，应明确拒绝歧义，而非让内部名字排序决定数学含义。空 group 不得消耗其他可见系列所需的共享轴。

**验收：** 重命名轴 ID、交换 series 声明顺序、将某一 series 过滤为空，都不改变其他系列轴/标注的含义和存在性；无数据、部分空、全部空场景分别有确定策略；除了计数还要断言标注的数值位置。

## 7. T05 · P2：反向坐标轴的 pan 与指针移动方向相反

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) visible domain 计算及 `viewport_domain`（3227）；[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) 中键 drag（640–649）。

**R：** zoom=1、同一组数据、pan.x 从 0 到 +20：普通 X 轴的数据点向右移动约 20px；`direction="reversed"` 时向左移动 20px。

runtime 将鼠标屏幕位移直接累加到 pan，但 domain 计算只考虑水平/垂直 range 的符号，没有考虑 axis.direction。结果是反向轴拖动时图像背离指针。同理需检查 reversed Y 和 linked viewport 的方向转换。

**修复：** 将 screen delta 经过实际 mapper/inverter（包含 axis direction、log 空间及 region）转成 domain window 位移，不在旁路手写只有 width/height 符号的公式。

**验收：** normal/reversed X/Y 的 20px drag 都使对应内容在屏幕同向移动 20px；ticks 与 data 对齐；log、不同 plot 尺寸、联动反向轴也保持窗口语义。

## 8. T06 · P2：导出 selection 仍会把业务键当作图例内部键

位置：[export.rs](../../../../crates/gpui-rhai/src/chart/export.rs) `export_scene` 的 selected_keys 循环（146–155）；对照 [primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `apply_selection`（1699）。

**R：** Bar 数据 id=`legend`，导出请求 selected_keys={`legend`}。输出的图例方块 `data-key="legend:s"` 也增加 `stroke-width="2"` 和 selection 颜色。

runtime 现在正确要求 role=Data 并使用 DatumRef；export 仍对所有 marks 按裸 `mark.datum_key` 匹配，保留了上一轮旧逻辑。因此同一个合法业务 selection 在 native 与导出产生不同作用范围。

**修复：** 共用 scene 的 selection 编译步骤，统一 role/DatumRef 匹配，不在 export 维护另一段近似循环。

**验收：** 选择业务键 legend、line、track 等值只影响对应 Data；图例/轴/annotation 不变；同一 acknowledged selection 的 native scene 与 export scene 静态样式一致。

## 9. 仍未覆盖的承诺

这些是范围说明，不计入新的 P1/P2 数量，也不作为无证据的故障声明：

- **G：** Chart 的 Motion suspend/resume、预算拒绝、卸载时后台任务与跨窗口隔离，仍没有本轮完整验收矩阵。现有普通输入和 idle 测试不能替代这些情形。
- **G：** 全主题/RTL/字体的真实窗口视觉、VoiceOver、30 样本 release 性能未验证。继续保留 release checklist，不因 debug 全绿而取消。
- **S/G：** 当前 native accessibility 投影提供 figure、active/selected 描述和计数；不能据此宣称已实现完整可浏览的数据语义窗口、Inspector provenance 和全部复杂系列语义导航。
- **S：** cache 的深克隆已去掉，但 `ChartSourceCache::matches` 仍做深相等比较。无复测数据时只确认实现改动，不宣称输入热路径已经达到常数成本。

## 10. 下一轮最小实施顺序

1. 修 T01，补不同坐标类型在采样阈值两侧的行为测试，避免数据静默消失。
2. 将 T02/T03 一起处理：完整的手势 generation / proposal / acknowledgement 协议，以及所有正式组件入口共同的 schema。
3. 修 T04/T05，把轴、标注和坐标转换的所有权从遍历顺序及旁路公式中抽出。
4. 修 T06，消除 selection 在 native/export 两条路径中的重复解释。

需要迁入产品的关键性质测试：

- 同一类别数据增加一行跨过采样阈值，不会从可画变成完全无数据；
- 确认某个旧 proposal，不会确认或取消它之后才产生的输入；
- 便捷函数与正式 source component 可以表达相同受控状态；
- 重命名内部 ID、交换声明顺序、让一组数据变空，不改变其他坐标事实；
- 反向轴改变数据排列方向，不改变指针拖动与屏幕内容移动的方向对应；
- 更换渲染/导出后端不会扩大 selection 的作用对象。

本轮代码响应比上一轮更完整；剩余问题集中在类型分派、状态代际和多个入口/后端的共享契约。下一轮不需要继续增加图表种类，应先让这些不变量得到明确实现与测试。

附件：[public-probes.rs](public-probes.rs)、[native-probes.rs](native-probes.rs)、[运行脚本](reproduce.py)、[复现说明](reproduce.md)、[验证元数据](evidence/metadata.json)。原有两轮审计及其证据未改动。
