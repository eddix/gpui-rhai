# 图表整改验收与二次审计

日期：2026-09-23。整改提交：`16cf8e51 fix(charts): enforce runtime contracts`。
原始审计基线：`e09b5b2f`；原报告见 [上一轮审计](../report.zh-CN.md)，实施说明见 [remediation.md](../remediation.md)。

**结论：有实质进步，但暂不通过整轮验收。** 多个原始复现已经修好，尤其是堆叠数学、transform 输出绑定、PNG 字体、重复注册原子性、图例角色和基础语义投影。然而，缩放被修成了没有有效变化，联动图出现空闲重绘循环；新轴计划与严格 key 校验还会拒绝一些合法组合。不能将「12 组都有代码响应」等同于「12 组全部关闭」。

本轮新增 **6 组必须整改的问题：R01–R05 为 P1，R06 为 P2**，另列出几个仍需收口的一致性与验收缺口。R＝独立复现，S＝源码确认，G＝尚未验收；不把三者混称为已经发生的故障。

审计开始时图表实现已提交且干净；工作树中保留上一轮 UI 规格文档。审计期间另一会话开始修改 Tabs/Button/Badge 等 UI，本轮未触碰它们。结束前 chart 源码及 registry/charts 仍与整改提交一致。本轮只新增此验收目录，没有修改产品代码、提交或操作 GitHub issue；接受当前阶段的 breaking API，不将新字段/新 key 格式作为兼容性 bug。

## 验证结果与证据解释

| 检查 | 结果 | 说明 |
|---|---|---|
| Workspace 全目标、全 feature、locked/offline | **548 通过** | 包含新 chart_contract 10 项 |
| 原生独立包 | **70 通过** | 现在明确启用 charts，包含新增的 5 项 |
| performance 默认检查 | **4 通过，3 ignored** | benchmark 仍需显式运行 |
| fmt / 严格 Clippy | **通过** | 无产品静态检查失败 |
| 原审计原生 5 项 | **5/5 通过** | 其中缩放用例存在假通过，见 R01 |
| 本轮新增原生断言 | **4 项均失败** | 有效缩放、跨帧手势、无 phase 滚轮、Gauge 业务 key |
| 两图联动独立探针 | **无法空闲，命中保护终止** | 宿主 render 达 64 次；非联动对照布局始终 2 次 |
| public 特征探针 | 执行完成 | 同时记录修好与未修好结果；exit 0 不等于验收通过 |
| v2 100k benchmark | debug 单样本通过 | 本次确实执行了目标数据 revision 断言 |

v2 benchmark 记录 prepare≈974ms、mount≈451ms、resize≈201ms、streaming≈350ms，Rhai operations=0。单个 debug 样本不具有分位数统计意义，也不能与上一轮不同工作负载下的 debug 数字直接做性能退化判定；这里只确认新断言能执行且通过。

复现入口：[reproduce.md](reproduce.md)。[公共探针输出](evidence/public.log)、[新增原生输出](evidence/new-native.log)、[联动循环](evidence/linked-idle.log)、[旧原生输出](evidence/original-native.log) 随报告保存。PNG 已重新目视检查，当前标题、坐标文字及图例文字可见：[本轮 PNG](evidence/export.png)。

## 上一轮 12 组问题的验收状态

| 原编号 | 已验证的进展 | 本轮判断 |
|---|---|---|
| C01 坐标/轴 | 显式共享 X 跨不同 Y 的位置正确；正值自动 log 可用；数字 category 可画；显式 category 本地化保留名称；投影拒绝点不再回退到经纬度 | **部分通过**：默认轴归一化、多轴 annotation、显式域与 viewport、自动类别格式仍失败 |
| C02 堆叠 | 10+10 落在域内，正负累计分开，stack group 有独立槽位 | **原主复现通过**；未据此宣称所有多轴 stack 组合完成 |
| C03 数据管线 | aggregate 输出可绑定；select_rows 保留列类型和空 schema；普通 null gap 保留；5k null 数据可准备；整数分组仍为 Integer | **部分通过**：空类别布局失败、空 aggregate 丢 schema、采样改变轴域 |
| C04 Polar 数值 | Gauge 25/100 不再画满；Radar 十倍值在同域下不再重合 | **原主复现通过**；Gauge/Map 的交互身份另见 R06 |
| C05 受控 viewport | 引入本地 preview 和 props 回写 | **不通过**：旧测试因所有 proposal 都是 1 而通过；R01/R03 |
| C06 身份/颜色 | `legend` 数据键正常触发 select；多 region 隐式轴 key 不再重复；隐藏 A 后 B 与图例同色 | **部分通过**：角色已类型化，mark key 仍是可碰撞字符串；R04 |
| C07 手势/联动 | brush 不再吞图例；brush/select 附结构字段；结束阶段提交 wheel | **不通过**：phase-less wheel 无提交，联动重复广播循环；R01/R02 |
| C08 导出/呈现 | PNG 字体有效；symbol 已进入真实 scene geometry；增加 clip 和 export viewport/selection 参数 | **核心复现通过，整体未关闭**：Geo viewport 无效；native stroke-only circle、selected 视觉仍与 SVG 不一致 |
| C09 A11y/诊断 | 原生 figure 能返回 active/selected 信息、revision/count；语义数据与绘制采样分开；焦点不再存数组下标 | **基础投影通过**：完整可浏览数据语义、Inspector、真实 VoiceOver 仍未证明；Gauge/Map 引用也不一致 |
| C10 性能/生命周期 | cache 在转换前判等；忽略 origin-only resize；布局转后台；v2 benchmark 断言 revision | **部分通过**：R02 引入重复工作；cache 命中仍深克隆；release/长期/Motion 生命周期验收仍缺 |
| C11 扩展 | duplicate Err 不再覆盖原实现；无 Value 的 Custom Polar 真正被调用；增加坐标 context 和 options | **原主复现通过**：不同坐标系的错误策略不一致，扩展布局输出仍限 marks |
| C12 API/适配器 | full-spec BarChart 能填 kind；axis title、color encode、tooltip tokens、funnel 顺序等有实现 | **部分通过**：自动类别轴格式仍错，完整公开字段行为矩阵及真实布局证据仍不全 |

文档对下采样承诺做了调整：现在强调保留 transformed semantic dataset、stable keys 和选中语义。这个收敛可以接受；本轮不会继续把「每个被采样掉的点必须可鼠标命中」当成未修复 bug。但 `draw-only` 必须至少保证坐标域、数据含义与空值有效性不被绘制采样改变。

## R01 · P1 · 原生绘制被误判为 Host acknowledgement，滚轮缩放失效

位置：[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `update_config` 268–311、`wheel` 614–627、`render` 1240；固定依赖 GPUI 0.2.2 的 macOS `platform/mac/events.rs` 217–240。

**R：** 原始 5 项现在通过，但原受控缩放探针打印的是：

```text
controlled zoom=1, wheel proposals=3|1.0|1.0
```

此前输入 40px 滚轮，第一次 proposal 应约为 `exp(-0.1)=0.904837`。旧断言只比较 first==last，因此「每次正确从受控值重新提案」和「每次完全没有变化」都会通过。

本轮增加有效变化断言以及跨帧测试：Host 接受回调值，发送 Started(+40px)，实际绘制/等待后台布局，再发送 Ended(0px)。仍得到 `callbacks=1, zoom=1.0`。普通 mouse wheel 的 Lines/Moved 事件等待多个帧后得到 `callbacks=0, zoom=1.0`。

**根因：** `ChartPrimitiveHandler::render` 会在正常绘制路径调用 `update_config`；后者无条件执行 `self.zoom=config.zoom; self.pan=config.pan`。本地 preview 导致布局完成和 notify，接下来的绘制把 preview 自己重置了，尚未到达手势结束。所谓“下一次 Host render 是 acknowledgement”在此实现中没有区分外部受控提交与 native draw。

此外，`wheel` 只在 `TouchPhase::Ended` emit。固定版本 GPUI 将 macOS 的 phase-none 等其他阶段映射为 Moved，非精确滚轮为 Lines；这里没有 phase-less 提交或静默截止策略。Ended 分支还没有检查是否真的有待提交手势。

**修复边界：** 为 props commit 引入明确 generation/revision 或等价的外部提交标识；普通 native render 只读取配置，不确认/拒绝正在进行的手势。手势 preview、pending proposal、committed value 分离；Host 相同值但新一次明确提交可以拒绝 proposal，不能用“任何一帧”替代它。phase-less wheel 使用有界合并/结束策略；空 Ended 不提交伪事件。

**验收：** 保留“不漂移”测试，同时断言第一次 proposal 确实为预期非 1 值；Moved/Ended 之间绘制若干帧仍正确；Host 接受、拒绝、钳制、延迟提交均成立；真实 mouse wheel 和 trackpad 两条路径都能提交。中键 pan 使用同一状态协议一并检查。

## R02 · P1 · 联动组在没有业务变化时形成重绘与布局循环

位置：[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `update_config` 291–299、`broadcast_selection_set` 170–189、`rebuild_scene_with_motion` 372–436。

**R：** 相同的两个 Custom Cartesian 图表，不设置 link 时，预热后 layout count=2，再刷新 8 次仍为 2。只添加相同 `link_group/link_domain` 后，窗口不能进入空闲；探针在**同一宿主第 64 次 render**主动退出。此保护是审计程序增加的退出，不是声称产品自己 panic。无保护的初次尝试持续运行，已手动终止，没有让后台进程遗留。

**根因闭环：** 每次 `update_config` 都 defer 广播 `config.selected`，不判断 selection 是否变化。接收方无条件赋值 `linked_selected` 并重建布局；完成后 notify，再走 primitive render/update_config，重新广播。甚至双方都选择空集也触发。

弱引用解决的是所有权环，不会自动解决这种通知环。每次广播都开新 layout job，旧任务不断被替换，既浪费工作也会妨碍实际最新候选稳定呈现。

**修复边界：** 在明确的业务提交边界、实际 selection/group 变化时传播；接收端先做有效状态相等比较，不变即返回。携带来源/版本防回流；更新“联动投影”不能被当成本地图表的再次提交。销毁、离组、清空要撤回相应来源的状态；多来源不能靠不断广播各自的空集互相覆盖。

**验收：** 本轮 linked 用例正常结束，稳定后的数据准备/布局计数不增长；强制窗口 redraw 不产生业务事件；一次 select/clear 仅导致有界传播；两个与三个图表、不同选择、换组与卸载均无反馈循环。

## R03 · P1 · viewport 没有完整参与坐标计划

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `layout_chart_scene_with_viewport` 694、`layout_cartesian_group` 1240–1300、`build_continuous_scale` 1790、`layout_geo` 2728、`viewport_domain` 3089；[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) paint 调用 906–918。

**R：**

| 合法场景 | 当前结果 |
|---|---|
| X/Y 显式 min=0,max=100，zoom 从 1 改 2 | marks 和 labels 完全相同；axis_domains.visible 却写为 25…75 |
| Geo 地图 zoom=2、pan=(30,10) | marks 与默认 viewport 完全相同 |
| 正值 log `[1,10]`，zoom=0.5 | `Scale(LogDomain)` |
| 字符类别数据、命名 X 轴 scale=Auto、NumberMetadata 有效 | Alpha/Beta 仍格式化成 `188.5/461.5` |

**根因：** Cartesian 在 inferred domain 上先应用 viewport，随后 `build_continuous_scale` 又拿 axis.min/max 覆盖可见域；公开 domain metadata 与实际 mapper 分离。Geo 路径未接收 viewport，同时 native paint 已统一传 zoom=1/pan=0，原来的地图几何变换被移除。Log 的窗口计算仍在线性数值空间缩放，缩小后下限可能跨过零。类别格式修复只检查声明的 `axis.scale==Category`，没有依据实际编译出来的 scale kind。

这也是联动目前仍不能按“完整域窗口”验收的原因：`linked_viewport` 取的 full/visible 可能不是实际 mapper 的域；代码只挑第一组 X/Y 域，并用单个 zoom 同时表示两个轴，log/reversed/多 region 的含义没有统一。

**修复边界：** 先确定完整域（显式 min/max 在这里生效），再在 scale 的正确空间计算 visible domain，所有 ticks、marks、invert、link metadata 使用同一个计划。Log 在对数空间处理；Geo 明确 camera 变换并共享给 paint/hit/export；实际 category tick 不走数字格式化。不要通过对显式域图表禁用手势或将 log 下限硬塞为 1 来绕过问题。

**验收：** zoom 后显式范围图确实改变可见域；metadata 与实际 mark/tick 一致；Geo 在屏幕、命中和导出上同步移动；log `[0.01,100]` 缩放/平移有合法策略；Auto category 与显式 category 标签一致；linked 不同尺寸/不同完整域图对齐同一数据窗口。

## R04 · P1 · mark role 类型化之后，身份仍会碰撞并拒绝合法图表

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `layout_cartesian` 1074、`layout_cartesian_annotations` 1585、`add_cartesian_axes` 1823、Line mark 生成约 1460、`add_circle_mark` 3145、全局去重 774。

**R：**

- 两个不同 Y 轴的系列共处 main region，增加一条 baseline：`DuplicateMarkIdentity("main:annotation:limit")`。
- 定义命名 X 轴 `x`，A 省略 x_axis 采用默认轴，B 显式绑定 `x`：`DuplicateMarkIdentity("main:grid:x:0")`。
- 普通 Line 的业务 id 合法取值 `"line:0"`：`DuplicateMarkIdentity("main:s:line:0")`。

现在 strict uniqueness 能检测到碰撞，但碰撞仍由内建 renderer 自己生成。annotation 仍在每个轴对 group 中重复创建；默认/显式轴的 grouping identity 不同，而输出 key 使用同一实际 axis key；业务 datum key 与 `line:{segment}` decoration key 仍共享字符串拼接空间。

**修复边界：** 默认轴绑定先归一到真实 axis ID，再分组；annotation 明确轴绑定并按其身份布局一次；MarkId 必须编码角色、region、series、datum、part，且编码无歧义，不能只加一个 region 前缀。禁止通过保留业务字符串 `line:0` 等规避内部身份错误。

**验收：** 上述三种配置正常布局；对 axis 显式/隐式写法等价变换不改变结果；多 Y 轴 annotation 位置可解释且只生成一次；任意合法 datum key 不与轴、图例、线段和 area 碰撞；动画身份不跨角色。

## R05 · P1 · 数据语义与绘制优化仍没有真正分离

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `PreparedSeries`、domain/category 收集 2981–3087；[transform.rs](../../../../crates/gpui-rhai/src/chart/transform.rs) `aggregate` 269–315、分段 LTTB 549 起。

**R：**

1. 两行带 id 的类别 Bar 过滤为零行，prepare 现在成功，但 layout 仍返回 `Scale(EmptyCategory)`。应用会保留上一张非空图，合法“无结果”仍不能呈现。
2. 空过滤结果继续做 aggregate，输出 `rows=0, columns=0`。`select_rows` 保型已修，但 aggregate 仍通过零行重新推断 schema，把分组列和输出 total 一起丢掉。
3. Line 数据 `[(0,0),(1,-500),(2,1000),(3,0)]` 显式 draw sample threshold=3。语义数据保留 -500，但生成的 Y full domain 为 **0…1000**。domain 收集读取的是 draw dataset，而非 semantic dataset；绘制采样改变了数值范围。

**S：** null-aware LTTB 给每个连续段至少保留若干点、另外保留全部 null separator，多个短段或很多 null 时结果可以远超 threshold。它目前更像采样目标而不是总预算，需要区分“保留 gap”与“保留每一行缺失记录”。不能以丢 gap 的方式重新修回旧缺陷。

**修复边界：** 空语义数据有明确可布局的 empty scene；category 不要求空集也生成一个非法空 scale。每个 transform 明确输出 schema，即使零行也保留；axis/color/indicator 域从完整 transformed semantic data 计算，draw sample 只缩减几何。采样预算按连续段分配，并保留 gap 边界而非盲目复制全部空行。

**验收：** 非空→空→非空时清空并恢复图像，不显示陈旧业务数据；filter→aggregate→encode 全链条零行保型；更改采样阈值不改变 domain/tick/业务量值；大量短段/缺失段仍保持有界准备与绘制成本。

## R06 · P2 · Gauge/Map 的 DatumRef 与回调、选择和语义树不一致

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) Gauge 2569–2638、Map 2743–2804；[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `mark_payload` 1523、`apply_selection` 1503。

**R：** Gauge 唯一数据行 id=`measurement-1`。scene 的 `datum.key` 是 measurement-1，但 `mark.datum_key` 仍是内部字符串 value。原生 Home/Enter 激活后，回调实际得到 **`datum_key="value"`**。因此业务侧依据事件回写 selected_keys 无法与原始数据、语义记录建立一致关系。

**R：** Map 源数据 id=`measurement-1`，name=`feature` 对应地图 feature。mark 的 datum_key/DatumRef 都是 feature，但 semantic dataset 中的 key 是 measurement-1。地图图形身份被当成业务数据身份，程序选择源数据 key 不会选中对应地图区域，焦点语义也无法自然对齐。

**S：** Radar 的 Data mark 仍 `datum=None`；`apply_selection` 仍遍历所有 role 按裸 datum_key 匹配。选择合法业务 key `legend` 时还会影响 legend mark 的 selected flag。结构类型已加入公共 API，但各消费方没有以它为唯一依据。

**修复边界：** MarkId/part 表示 gauge value/track、map geometry 等绘制身份；DatumRef 表示业务数据或显式定义的聚合实体。事件、selection、focus、semantics 使用同一个引用。Map 的 feature join 保存源行引用；一对多/无匹配区域明确规则。系列级 Radar 激活若不能对应单行，应公开系列/聚合事件，不能伪装成有完整数据引用的行事件。

**验收：** Gauge 回调为 measurement-1 且受控选择高亮同一项；Map 源 key≠feature key 时仍闭环；选择不改变 legend/control role；复杂系列与自定义系列使用同一数据身份规则。

## 其他仍需收口的问题与验证缺口

以下不作为“又新增几个 P1”计数，但不能在总体完成说明中省略。

- **R / C11 错误策略不一致**：同一注册 renderer 返回 `Err("candidate rejected")`，Cartesian 的 layout 返回 Err，Polar/Geo 却返回 Ok 并附 Error diagnostic。见 public 输出 CUSTOM_FAILURE 三行。后者会被 ChartEntity 当作可安装场景，并清空顶层 error。需要明确哪些故障允许局部缺系列、哪些必须保留完整 last-good；不能因坐标分支不同而隐式改变语义。现有 trait 文档本身同时允许“rejected or reports series error”，应统一公开契约并补原生故障注入，而非仅凭辅助函数测试宣布候选原子性完成。
- **S / C08 绘制仍有分叉**：`paint_circle` 在 stroke 存在时先画一个半径 radius+width 的实心圆，再用 fill 覆盖中心。fill=None 的合法空心圆不会挖空中心，native 仍是实心盘，SVG 是空心圆。selected/hover 时 native 还在 painter 临时 lighten fill；export with_view_state 只写 stroke，没有同样的 selected 填充规则。应将静态选中样式编译入统一 scene，并真正绘制环形 stroke。
- **S / C10 cache 仍有 O(N) 命中成本**：`ChartSourceCache::matches` 深比较 data，命中后 `cached.clone()` 又克隆包含 inline UiValue rows 的 source cache；`same_source` 还可能再次比较数据。转换次数确实减少，但“缓存命中就是常数成本”尚不成立。没有重新测量就不报具体毫秒收益；建议共享不可变 props/snapshot 并记录实际转换/克隆计数。
- **S/G / C09 语义投影边界**：当前是 figure 描述及 value Map，active 与最多 16 个 selected 文本有用，但不是完整可浏览语义数据窗口。Inspector 也没有呈现 source/presented revision、候选错误、transform provenance 等完整接入证据。文档删除部分旧承诺不自动等于原 ADR 的统一 scene 消费已经实现。
- **G / Motion 与生命周期**：Chart 仍有独立 scene transition / animation-frame 路径。全局 suspend/resume、预算拒绝、卸载后后台候选、跨窗口/换 link domain，以及持续 stream 的峰值资源没有本轮新增验收证据。不能沿用旧 MotionRuntime 测试成绩替代新 chart 路径。
- **G / 发布矩阵**：真实 VoiceOver、全部主题/RTL/字体的实体窗口截图、30 样本 release 性能仍是待办。实施说明对此有如实保留，本轮认可该边界，没有将“未跑”改写成“已失败”。

## 下一轮实施顺序

1. **先修 R02、R01**：让联动图能够空闲、手势能够产生有效 proposal。绘制和配置提交必须分开。这两项不应靠 debounce 一个错误循环来掩盖。
2. **再完成统一坐标计划（R03/R04）**：轴默认值归一、显式完整域、visible domain、scale kind、annotation 归属一次编译；所有下游复用真实 mapper。
3. **补数据与身份闭环（R05/R06）**：零行 schema/empty scene、semantic domain、DatumRef 贯通事件/选择/语义。
4. **最后收口跨后端、扩展与性能证据**：失败候选策略、空心圆与静态选中样式、cache 命中成本、Motion/生命周期与 release 矩阵。

关键测试应从“个别正例”升级成**等价变换与组合不变量**：

- 同一个手势在输入之间插入绘制帧，提案不变；
- 同一个页面插入无状态变化 redraw，准备/布局次数不变；
- 隐式轴改为显式引用同一轴，结果不变；
- 调整 draw sample 阈值，domain 和语义数据不变；
- 数据 key 从普通文本改为 `line:0` / `legend`，图元角色和合法性不变；
- 选择事件回写给对应受控属性，可以定位回同一个业务对象。

这些测试直接覆盖本轮遗漏的原因，比继续增加“配置能 parse”“输出非空”“first 等于 last”的断言更能防止下一次假通过。

## 交接附件

- [public-probes.rs](public-probes.rs)：旧输入复验与本轮组合探针。
- [native-probes.rs](native-probes.rs)：4 个失败行为断言和独立有界联动探针。
- [reproduce.py](reproduce.py) / [复现说明](reproduce.md)：不修改产品清单的独立运行入口。
- [metadata.json](evidence/metadata.json)：提交、范围、测试数量与边界。
- [PNG](evidence/export.png) / [SVG](evidence/export.svg)：确认字体修复的真实产物。

原报告和原始证据保持不变。后续应以本目录中的行为验收转绿、以及明确关闭剩余验证缺口作为完成依据。
