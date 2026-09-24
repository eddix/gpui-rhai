# 第五轮图表与生命周期验收

日期：2026-09-23。基线：`30c29929 fix(charts): complete axis and lifecycle contracts`。
参照 [第四轮报告](../acceptance-7d66a14d/report.zh-CN.md) 与 [本次整改说明](../acceptance-7d66a14d/remediation.md)。

**结论：第四轮的原始复现已经通过，但整体验收仍有 3 组问题：2 个 P1、1 个 P2。**
正常路径确实改善：交叉主轴映射正确，挂起后流数据不再逐次布局，恢复后的普通滚轮正常。
本轮新失败集中在原生生命周期的异常事务、空 schema 对轴编译的影响，以及正常动效的冻结进度。

R＝独立复现，S＝源码确认，G＝未验证。本轮不要求历史兼容，不将新 hook 或 API 本身视为问题。
只新增本文目录中的文字、代码与日志；未修改产品实现、提交或操作 issue。此前 UI 文档整理和参考图删除保持原样。

## 1. 旧问题复验与验证结果

| 检查 | 结果 |
|---|---|
| 原交叉主轴用例 | annotation 正确落在 `(325,185)` |
| 原挂起流数据用例 | 挂起期间布局计数保持不变 |
| 原挂起中断手势用例 | 有/无中断手势两组恢复后都能提交普通滚轮 |
| 第四轮独立 native 探针 | **2/2 通过** |
| Workspace 全目标、全 feature、locked/offline | **560 通过** |
| 原生独立包 | **84 通过，其中 chart 15** |
| performance 默认检查 | **4 通过，3 ignored** |
| fmt / 严格 Clippy | **通过** |
| 本轮新增 native 探针 | **3 项均失败** |
| 100k v2 benchmark | debug 单样本通过，包含目标 revision 断言 |

产品新增测试还验证了挂起期间三次 data replace 合并为恢复时的一次布局；它在完整原生套件中通过。
因此本文不会把上一轮“正常挂起仍逐次布局”和“旧 Explicit 手势吞新输入”重新列为未修复。

debug 单样本 prepare≈1349ms、mount≈495ms、resize≈186ms、streaming≈336ms，stream 后 Rhai operations=0。
这里只确认流程和断言通过，不据单样本作帧预算或性能退化判断。完整 release smoke、30 样本 release、
实体屏幕主题/RTL 与 VoiceOver 本轮没有重跑。

## 2. V01 · P1：原生生命周期 hook 失败后，视图进入半恢复/半挂起状态，且不能正常重试

位置：[app.rs](../../../../crates/gpui-rhai/src/app.rs) `suspend_view`（约 4090–4113）、`resume_view` / `resume_native_primitives`（约 4180–4220）；
[primitive.rs](../../../../crates/gpui-rhai/src/primitive.rs) `suspend_mounted` / `resume_mounted`（1123–1154）。
契约依据：[Embedding](../../../embedding.md#retained-suspension-and-host-managed-tombstones) 的原子恢复与失败保留 Suspended，以及 `ScriptViewHandle::resume` 的失败状态保证。

**R：** 通过正式扩展 API 注册 A、B、C 三个生命周期 primitive，记录各自 active 状态和调用次数。
B 的 hook 主动失败一次，A/C 是健康对照。由于 hook 当前是无 Result 的签名，探针使用一次可控 panic，
由产品已有的 `guard_primitive_panic` 捕获成 Err。

| 操作 | 公开返回 | 公开视图状态 | A/B/C 的 active 标志 | 再次调用 |
|---|---|---|---|---|
| B.resume 失败 | `Err(Resume(...))` | **Active** | `[true,false,false]` | `Ok(false)`，B/C 未重试 |
| B.suspend 失败 | `Err(Suspend(...))` | **Suspended** | `[false,true,true]` | `Ok(false)`，B/C 未完成清理 |

两种操作的 hook 调用次数都是 `[1,1,0]`，说明中途失败直接跳过 C。这里不是把“故意注入的 panic”
当作缺陷，也不是声称发生了随机崩溃；缺陷是框架已经捕获并返回 Err 以后，仍暴露不一致状态并抑制重试。

**根因：**

- suspend 在通知原生实例前，脚本 lifecycle 和公开 state 已提交 Suspended；registry 遇到第一个错误就提前返回。
- resume 先完成脚本恢复事务，然后才调用原生 hook；`resume_native_primitives` 的错误分支更显式地把 state 设成 Active。
- 公开 handle 的幂等判断只看 state，导致后续请求返回 `Ok(false)`，无法完成第一次遗漏的操作。
- 已经恢复的 A 没有被重新 quiesce，尚未恢复的 B/C 却与 A 一起被当成同一个 Active view。

这是新增通用 primitive hook 的共享基础设施问题，影响范围不限于 Chart。Chart 的正常 hook 不报错，
不能证明这个新边界已经满足已有的视图失败契约。

**修复边界：** 让脚本状态、公开 view state、原生实例活动状态共同跨越提交边界。可以用明确的准备/提交阶段，
或在失败时执行有界补偿/清理，但必须保证失败后仍有一致、可观察、可重试的状态。
一个清理失败不应阻止其他健康实例清理。原生 resume 失败时应 quiesce 已启动的同伴，并保留可恢复的 Suspended 状态。

不能只将错误分支的 `state=Active` 改成 Suspended：此时内部 ScriptLifecycle 已经恢复为 Running，
下一次 resume 仍可能无法正确执行。需要同时验证内部状态和原生资源，而不只修改外部枚举。

另外，trait 注释称 resume hook 在 owner becomes active 之后执行，实际成功路径是在公开 Active 标记之前调用。
应明确准备阶段允许的操作与实际激活时机，避免扩展按注释在错误阶段交付事件或请求交互。

**验收：**

- 第一、中间、最后一个实例分别失败；健康后续实例仍获得必要清理，不能暴露混合活动状态；
- 原生 resume 失败后 `view.state()==Suspended`，已恢复同伴已 quiesce；故障解除后再次 resume 真正成功；
- 重试成功后全部实例 Active，不能只返回 `Ok(true)`；
- 已排队任务/事件不能在失败恢复与实际激活之间越界交付；
- 明确 panic、registry 借用错误、重复调用与 dispose 的处理，不要求回滚任意外部副作用，但必须守住内部状态和资源活动边界。

证据：[new-native.log](evidence/new-native.log) 的 RESUME_HOOK_FAILURE / SUSPEND_HOOK_FAILURE；
[native-probes.rs](native-probes.rs) 中两个 hook failure 测试。

## 3. V02 · P1：轴与 annotation 使用不同的贡献数据，仍能编译出两种 scale

位置：[scene.rs](../../../../crates/gpui-rhai/src/chart/scene.rs) `compile_cartesian_axis`（1129 起）、
series group 的 `x_domain_series/y_domain_series` 收集（约 1298–1310）、annotation 数据收集（1334 起）。

**R：** 两个可见 series 共享 Auto X 轴：

- main 数据集已经过滤为空，保留合法的 String X schema；
- numeric 数据集有两行，X/Y 分别为 0/0 和 10/10；
- 两个系列绑定不同 Y 轴，声明同 region 的 `mark_point(x=10,y=10)`。

结果：

```text
data point for X=10: x=461.5
annotation X=10:    x=598.0
public X axis kind: Category
```

这是合法空集/保型与 Auto 轴推断的组合，没有向同一个列塞入混合类型。无论最终决定空列是否参与 Auto 推断，
轴、数据与标注都必须服从同一个结果；不能成功返回一张同时采用两个尺度的图。

**根因：** 数据系列的轴编译包含同轴的空 schema，String 类型使 X 变成 Category。
annotation 调用同一个 helper 前过滤掉了所有空 dataset，X 又被编译为 Linear。
本轮消除了两份实现，但仍在不同调用点以不同输入重新编译同一个 axis ID。

**修复边界：** 每次场景编译先按 `(region, axis ID)` 确定统一的数据贡献集合和 Auto/type 策略，
产出唯一的 scale/domain 计划；series、ticks、annotation、custom context 都引用它。
若类型组合不支持，应一致拒绝/诊断，而不是让调用点自行筛选数据后各推断一遍。
“调用同一个函数”不等于“共享同一个坐标事实”。

**验收：** 空 String/Bool/Timestamp/Number schema 与非空系列混合、过滤前后、显隐切换及换序时，
同一 axis ID 的 mapper/domain/type 始终一致。断言数据点、tick 和 annotation 对同一值重合；
同时保留第四轮交叉主轴用例，避免为统一贡献集合重新要求某个 series 承载整个轴对。

证据：[public.log](evidence/public.log) 的 EMPTY_SCHEMA_AXIS_FACT，代码见 [public-probes.rs](public-probes.rs)。

## 4. V03 · P2：挂起后恢复的 Chart 动画会追赶墙钟，直接跳到终点

位置：[chart/primitive.rs](../../../../crates/gpui-rhai/src/chart/primitive.rs) `ChartEntity::suspend/resume`（367–399）、
`displayed_scene/current_scene_sample`（1099 起）、`rebuild_scene_with_motion`。
契约依据：[Embedding](../../../embedding.md#retained-suspension-and-host-managed-tombstones) 规定 timers/Motion 从冻结进度恢复，不追赶挂起墙钟。

**R：** 使用 Normal motion、ManualRuntimeClock 和稳定 key 的自定义矩形；初始动画先完成，随后让矩形
从中心 `(92,102)` 移向 `(292,102)`。新过渡刚开始时，旧位置能命中，确认 fixture 正常。
挂起视图，只在 Suspended 阶段推进手动时钟 60 秒；恢复后不推进任何活动时间：

```text
hits before suspend                  = 1  (old position works)
hits after resume, click old position = 1  (no hit)
hits after resume, click terminal     = 2  (terminal position works)
```

使用的是正式 Chart 输入路由和同一场景采样的命中结果，不依赖肉眼估计动画帧。终点能够命中也排除了
恢复后整个输入失效这一替代原因。本轮没有声称截图像素或所有 easing 都已检查。

**根因：** 新 suspend hook 取消了任务/手势，但没有保存动画采样、挂起时刻或剩余时长，也没有调整
`transition_started`。恢复的 prepare/layout 又调用 `current_scene_sample` 取得 previous；它按当前墙钟
减去旧 start，认为过渡早已完成，从错误终态继续重建。通用 MotionRuntime 的暂停偏移不会自动作用于这份独立计时状态。

**修复边界：** chart 聚合动画应进入受 view 生命周期约束的时间模型。保留冻结 sample 和剩余时长，
或正确偏移该资源的时间原点；无数据变化时连续恢复，挂起期间有数据变化时从冻结 sample 过渡到最新 target。
还需避免 resume 的无变化重建无条件重启动画。

**验收：** 在开始、中间、临近结束三个进度挂起，分别推进长短墙钟；恢复第一帧等于冻结帧，
后续只消耗剩余活动时间，既不追赶也不从头重放。与 burst 数据、resize/主题变化、Reduced/None、重复 suspend/resume 组合验证。

证据：[new-native.log](evidence/new-native.log) 的 FROZEN_MOTION 与 `chart_motion_freezes_across_suspension`。

## 5. 下一步与验收范围

建议先处理 V01 的共享生命周期事务，再把 V02 做成真正单次编译的轴计划，最后补 V03 的冻结时间模型。
这三处都应维护一个权威状态：视图活动状态、轴计划、动画时钟，避免各个调用方重新推断。

发布验证中仍未覆盖的项保持 G：全主题/RTL/字体的实体窗口、真实 VoiceOver、30 样本 release 性能、
chart Motion 的全部预算与多窗口隔离组合。它们不计入本轮新增缺陷数，也不因当前回归全绿就宣称已通过。

本轮正常路径的进展可以确认；待修复的是异常路径及共享状态一致性。原报告、前四轮证据和产品实现均未改动。

附件：[复现说明](reproduce.md)、[运行脚本](reproduce.py)、[public-probes.rs](public-probes.rs)、
[native-probes.rs](native-probes.rs)、[元数据](evidence/metadata.json)、[前轮重跑](evidence/previous-native.log)。
