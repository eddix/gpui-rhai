# 第四轮图表整改验收

日期：2026-09-23。图表修复：`7d66a14d fix(charts): close third acceptance gaps`。
审计 HEAD：`243b0098`，额外包含 Gallery 滚动背景修复；该提交没有再改图表实现。

**结论：上一轮 T01–T06 的原始复现全部通过，本轮仍确认 2 组 P1，需要修复后再作整体验收。**
一组是主 X/Y 轴未被同一系列同时引用时的标注绑定；另一组是 Chart 原生实体未完整接入视图挂起/恢复边界，导致隐藏时继续布局、恢复后普通滚轮不提交。

这是已修问题之外的组合与生命周期验收，不将已经通过的旧复现重新写成未修复。
生命周期此前多轮明确列为 G（未验证），本轮已补实际测试并得到 R（复现）证据。
全文 R＝独立复现，S＝源码确认，G＝未完成验证。

本轮只新增验收文档、代码探针和文本日志，未修改产品代码。已有 UI 文档归并和参考图删除保持原样，未复制新的图片进仓库。

## 1. 上一轮整改验收

| 原编号 | 独立复验结果 | 判断 |
|---|---|---|
| T01 类别数据跨采样阈值消失 | 4000/4001 行都有数据 mark 和折线；原始类别、key、semantic rows 保留 | 原复现通过 |
| T02 旧确认擦掉新手势 / 提前提交 | 旧确认后第二次提案仍包含新输入；显式 Started 在 Ended 前回调数为 0 | 原复现通过 |
| T03 正式适配器缺失 revision | 原 BarChart mount/转发探针通过；产品原生用例覆盖四个正式适配器 | 原复现通过 |
| T04 轴名字及空组改变绘制 | 重命名 Y 轴后 baseline 均为 y=185；空组不再吃掉 X 轴和标注 | 原复现通过；更一般的交叉轴组合见 U01 |
| T05 reversed pan | normal/reversed 的 +20px pan 都使数据点向右约 20px | 原复现通过 |
| T06 export 误选图例 | 业务 key=legend 时图例保持 stroke=none、width=0 | 原复现通过 |

上一轮独立原生探针 **9/9 通过**，其中包括有效缩放数值、phase-less 输入、联动空闲、来源卸载撤销高亮、Gauge 业务 key 和正式适配器的真实 mount。
原证据和用例保持不变，见 [本轮重跑输出](evidence/previous-native.log) 与 [public 复验](evidence/previous-public.log)。

## 2. 常规验证

| 检查 | 结果 |
|---|---|
| Workspace 全目标、全 feature、locked/offline | **559 通过** |
| 原生独立包 | **82 通过，其中 chart 13** |
| performance 默认检查 | **4 通过，3 ignored** |
| fmt / 严格 Clippy | **通过** |
| 上一轮独立原生探针 | **9 通过，0 失败** |
| 本轮新增原生探针 | **0 通过，2 失败** |
| 100k v2 benchmark | debug 单样本通过，目标 revision 断言通过 |

本机 debug 单样本：prepare≈919ms、mount≈529ms、resize≈191ms、streaming≈346ms，stream 后 Rhai operations=0。
这些只证明冒烟及断言执行成功，不是 release 分位数或帧预算认证。

Gallery 的额外修复将 surface 背景从滚动内容转移到固定 scroll viewport；代码归属合理。本轮未重新打开实体窗口目视认证，实施方记录的目视结果与本轮代码检查应区分。

## 3. U01 · P1：主 X/Y 轴来自不同系列组合时，标注仍绑定错误轴

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `layout_cartesian` 的 `primary_axis` / `annotation_group`（约 1178–1214）及 `layout_cartesian_group` 中的 annotation 调用。
相关公开承诺：[charts.md](../../../charts.md) 规定未显式限定的 annotation，每个 channel 分别绑定该 region 第一个有非空可见系列的已声明轴。

**R：** 合法同 region 配置有四根轴，声明顺序如下：

| 轴 | 位置 | 数据范围 |
|---|---|---|
| primary_x | bottom | 0…10 |
| primary_y | left | 0…10 |
| secondary_x | top | 0…1000 |
| secondary_y | right | 0…1000 |

两个系列分别绑定：

- A：primary_x + secondary_y；
- B：secondary_x + primary_y。

两个主轴都有效，只是没有一个系列同时引用它们。声明 `mark_point(x=5,y=5)`：

```text
plot center / expected annotation: (325.00, 185.00)
actual annotation:                 (54.73, 185.00)
```

公开 axis_domains 确认 primary_x、primary_y 都是 0…10，而实际 annotation 的 X 却使用 secondary_x 的 0…1000。仅仅缺少一个引用主轴组合的系列，就把标注从中心画到左边缘附近。

**根因：** 实现先正确独立选择 primary_x/primary_y，然后要求在既有系列 group 中找到该轴对；找不到就回退到「包含 primary_y 的 group」。这个 fallback 静默替换了 primary_x。上一轮把“第一个 group”换成“优先包含主轴的 group”，仍未建立独立的 annotation 坐标计划。

**修复边界：** 按 region/axis ID 编译可复用 scale，将 annotation 绑定到两个独立解析出的 scale；轴对不需要已有系列作为承载者。若未来支持显式 annotation axis reference，也应使用同一入口。不能要求用户添加虚假的空系列来制造主轴组合。

**验收：**

- 交叉轴用例严格落在 `(325,185)`，同时断言两个 channel；
- 不同 series-pair 组合、series 换序、轴 ID 重命名不改变主轴意义；
- 主轴有有效数据但没有直接相连的系列，annotation 仍存在且映射正确；
- empty/hidden、reversed、log/time 的主轴解析复用相同计划；
- 所有 mark_point/mark_line/mark_area/baseline/threshold_band 使用一致的绑定规则。

证据：[public.log](evidence/public.log) 的 `CROSSED_PRIMARY_AXES`，复现代码位于 [public-probes.rs](public-probes.rs) 的 `crossed_primary_axes`。

## 4. U02 · P1：Chart 原生订阅和手势未完整进入 suspend/resume 生命周期

位置：[app.rs](../../../../crates/gpui-rhai/src/app.rs) `ScriptHostView::suspend_view` / `resume_view`；[primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `restart_data_listener`（462 起）、`start_prepare`、`rebuild_scene_with_motion`、`wheel`（775 起）。
契约依据：[Embedding 的 retained suspension](../../../embedding.md#retained-suspension-and-host-managed-tombstones)：Host 从活动布局移除视图，释放 focus/pointer capture、清理长期订阅所属 effects，挂起期间积累失效，恢复时重新提交；允许已在途的一次性工作完成，不等于继续为每次新数据启动布局。

### 4.1 新数据在挂起期间仍逐次启动布局

**R：** 正式 Chart 绑定 NativeChartData，并注册只计数、不产生额外图元的 trusted custom renderer。初次布局后调用公开 `view.suspend`；Host 按文档将该视图从布局中移除。确认 executor 空闲后，连续发布三个新的 data revision：

```text
state=Suspended
layout count before=1
layout count after =4
```

这三个调用都由挂起后才产生的数据触发，不是把已有在途工作算成缺陷。测试设 MotionPreference::None，没有动画帧干扰。

**S：** ChartEntity 的 data listener 直接调用 `start_prepare`，随后安装 prepared data、启动布局并更新 scene；没有 view suspension guard。ScriptHostView 在挂起时保留 native entities，但没有通知这个订阅/布局状态机进入暂停。被隐藏的实时仪表盘因此仍为更新计算几何，而非只保留最新失效/revision。

### 4.2 活动手势跨挂起后，恢复的普通滚轮不再提交

**R：** 同一 fixture 有正常对照组，恢复后都确认 `view.state()==Active`：

| 挂起前状态 | suspend → resume → 普通 Lines/Moved 滚轮 | 结果 |
|---|---|---|
| 无活动手势 | 1 个滚轮输入 | callback=1，zoom≈0.960789 |
| 先收到 Started，随后视图被挂起，Ended 无法再交付 | 相同输入 | **callback=0，zoom=1.0** |

**根因：** `wheel_gesture=Explicit` 只由 Ended 重置。挂起清理了视图展示和捕获，却保留这个不可能自然结束的活动手势；恢复后的普通鼠标事件仍命中 Explicit 分支，既不 emit，也不安排 phase-less timer。

暂停时移出布局是公开支持的正常 Host 操作，不能要求 Host 给隐藏的图表补发虚构 Ended 才恢复可用。

### 修复边界

为 retained native primitive 建立明确的 view lifecycle 通知/检查，保证 Chart 与 Host 对“活动、挂起、恢复、卸载”的认识一致，而非仅在顶层不渲染。

- 挂起：停止/门控新的布局与订阅交付，取消 timer 和活动 wheel/pan/brush；保留已提交的配置、选择与必要原生状态。已经启动的计算可按取消能力结束，但不能越过挂起边界安装新呈现场景。
- 数据继续到达：记录最新 snapshot/revision 或有界失效；不为每个更新重建几何。
- 恢复：订阅重新建立后读取最新快照，按恢复事务呈现当前数据；避免重新监听与读 revision 之间丢更新。
- 手势：对挂起打断的未提交 preview/proposal 定义取消或恢复策略；新的独立输入必须从可用的手势状态开始，不能被旧 Explicit 标志吞掉。
- 卸载与窗口关闭：共享同一资源清理边界，丢弃晚到的候选与失效回调，不能为了修 suspend 改成每次销毁所有保留状态。

**验收：** 两个新增原生断言转绿；挂起后 burst 更新只在恢复时按最新版本产生有界工作；对每种输入在 idle/Started/dirty preview/pending proposal 时分别 suspend/resume；重复挂起、恢复失败、卸载、窗口关闭和后台 job 完成不会越界。另补 Motion freeze/resume 的实际 chart 路径测试。

证据：[new-native.log](evidence/new-native.log)，代码见 [native-probes.rs](native-probes.rs)。本轮没有据这两个用例宣称已测试所有动画或资源泄漏情形。

## 5. 后续工作与发布边界

建议先修 U02 的统一生命周期，再修 U01 的独立轴计划。两处都应落在共享契约上：不能用 renderer 内部计数限流掩盖挂起后仍做工作，也不能通过另一条 annotation group fallback 修补坐标组合。

仍未完成的发布验证保持 G：实体屏幕全主题/RTL/字体、真实 VoiceOver、30 样本 release 性能、chart Motion 的全局预算/suspend/窗口隔离。实现方报告的 release smoke 通过有价值，但不替代这些不同维度的验收。

本轮常规回归和上一轮探针全绿，说明修复在收敛；新的失败集中在两个尚未完全共享的边界：annotation 对独立轴的使用，以及 ChartEntity 对 Host 生命周期的服从。

## 6. 交接材料

- [reproduce.md](reproduce.md) / [reproduce.py](reproduce.py)：独立临时包运行方式。
- [public-probes.rs](public-probes.rs)：前轮特征复验与交叉主轴用例。
- [native-probes.rs](native-probes.rs)：暂停流布局、恢复手势的原生测试和对照。
- [metadata.json](evidence/metadata.json)：提交与验证数量。
- [workspace 输出](evidence/workspace-tests.log)、[原生套件](evidence/native-tests.log)、[debug 冒烟](evidence/chart-smoke.log)。

原有三轮审计与其原始证据未改动。
