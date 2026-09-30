# 0.1.8 整改后第二轮对抗审查

日期：2026-09-30。基线：合入 main 的 `d77b4b49a667da618fe9b9583d331d8c325cf180`；对照整改前 `0b9b887c`。

## 结论

**上一轮原始 GPUI 探针已独立复跑，25/25 通过；但当前仍不建议发布 crates.io 或创建正式 v0.1.8 release/tag。** 新一轮确认 **7 组 P1、9 组 P2**，包括正常暂停使 PanZoom View 被销毁、异步更新绕过取消、合法 RangeSlider 配置无法呈现、虚拟排序跨可视区失败，以及新 live-register 的子组件失效遗漏。

这不是否定已完成的整改：owner 增加 retained identity、跨 View capture、普通卸载取消、clip/occlusion、caller wrapper、Tree 全图验证、PR #88 和异步结果交付都有可核验的进展。主要剩余问题是**同一能力的不同输入、生命周期和提交入口，尚未共同遵守全部约束**；另有数值和多边形修复引入的新边界错误。

本轮按修订后的 ADR 0022 审查，不继续要求已经收缩掉的 awaiting-control 协议，不把 Dock／OS 跨窗口 drag／任意 GPUI 子树变换列为缺失，也不将 pre-1.0 break 作为 bug。

主审和六个并行方向只新增本目录材料，没有修改产品、正式测试、上一轮审计文件或 GitHub 状态。PR #81 的 merge commit、当前 open/closed issues 已通过 GitHub 只读查询核对。代码合并不等于下面的发布门槛已满足。

## 1. 上一轮修复复核

| 范围 | 独立结果 | 证据 |
|---|---|---|
| 原 capture / source unmount | 4/4 | [runtime-core.log](baseline/runtime-core.log) |
| 原 drop / Sortable | 5/5 | [drag-sort.log](baseline/drag-sort.log) |
| 原 resize / range / composition | 10/10 | [resize-controls.log](baseline/resize-controls.log) |
| 原 affine / selection | 6/6 | [transforms-selection.log](baseline/transforms-selection.log) |
| #86 新交付边界 | 6 个正向与资源边界探针通过；正确性闭环成立 | [Rhai 报告](rhai-async/report.zh-CN.md) |
| Tree 与 PR #88 | 默认／空／折叠挂载、全图 depth 和重复遍历已修；map move 静态核验正确 | [集合报告](collections-tree/report.zh-CN.md) |
| #90 原最小复现 | 已通过，并准确提示 Host lifecycle 未执行 | [CLI 对照](tooling/cli-results.json) |

原探针的 repo source 未改；[results.json](baseline/results.json) 记录实际 HEAD、退出码与源文件 SHA256。为避免并行临时 workspace 同名 test target 的缓存干扰，最终 core 复跑使用唯一 `round2_original_core`，复制前后校验 source 字节相同；本表只引用最终隔离运行。

## 2. P1 发布前应解决

### R2-F01：PanZoom 正常暂停仍失败，失败补偿使整个 View 变为 Disposed

只需挂载默认 PanZoom，不进行任何手势，再调用 `view.suspend`。先触发 `cannot read ScriptHostView while it is already being updated`；补偿再触发 PanZoomEntity 更新重入，最后返回 Suspend error，View 状态为 `Disposed`。

新增 direct signal lane 只处理 write，而且在 `primitives.suspend_mounted` 之后才打开。PanZoom 的 suspend hook 在此之前先读取 wheel-generation signal，再恢复 source。单纯把原 bool flag 提前仍不能覆盖 read 路径。

位置：`app.rs:4383/3351/4425`，`pan_zoom.rs:368/619`。这是上一轮生命周期整改的剩余覆盖缺口；准确后果是正常暂停失败并丢弃 View，不夸大为整个进程必然退出。

整改应给 native lifecycle hooks 明确、不会重复 lease owner Entity 的 signal 访问上下文，覆盖 prepare、cancel、补偿和 resume；不要在单个 callback 忽略错误。验收 idle/pointer/wheel/pending commit 下的 suspend/resume 以及注入失败补偿。

证据：[runtime-core RC2-1](runtime-core/report.zh-CN.md)。同组正常 pan、Rhai source 替换、native source 替换三项通过；未证实的来源替换重入猜测已排除。

### R2-F02：取消只覆盖节点回调，timer/async 更新仍允许旧手势提交

Draggable 从 `{20,20}, axes=horizontal` 开始拖动；声明式 timeout 把 axes 改成 vertical，松手前已确认新状态；松手后仍提交旧水平结果 `{60,20}`。

取消位于 `handle_node_event` 的成功 rerender 尾部（`app.rs:4752–4753`），异步 delivery 的 `render_dirty`（`:4994–5005`）没有同样处理；Draggable 本地失效键也没有包含 axes 等完整约束。

这违反当前 ADR 的 source/constraint rerender 取消契约，和旧异步 acknowledgement 要求无关。应在共同成功提交／契约失效边界产生并安全消费取消，而不是为每一种调用者继续补一次 cancel。

证据：[尺寸 R2-RC2](resize-controls/report.zh-CN.md)。验收需交叉 timer、task/subscription、Host 数据／主题、普通事件及 native handler，且未改来源的正常手势继续可用。

### R2-F03：Table 拒绝提议只修了 pointer，keyboard/autofit 仍留下错误宽度

同一个受控 width=160、callback 不采纳提议的 Table：keyboard 后分隔条 x 从149留在157；双击 autofit 后留在37。两者 callback 都执行一次，但未恢复受控宽度。旧 pointer probe 已通过。

位置：`table.rhai:186–196`、`column_resize.rs:232–245/287–292`。应把三种入口的提议派发和 preview 清理统一起来，并同时验证接受、钳制、拒绝和 callback 缺失。它是旧根因修复覆盖不完整，不是另一个独立宽度模型需求。

证据：[尺寸 R2-RC3](resize-controls/report.zh-CN.md)。

### R2-F04：RangeSlider 新归一化逻辑对合法端点配置产生反向 clamp

`min=0,max=100,step=10,minimum_gap=5,values={low:95,high:100}` 有合法网格解 `[90,100]`，且通过现有 schema。但分别 snap 后得到 `[100,100]`，再向上扩展 high，将 `105..100` 交给 clamp，发生 native render panic。

primitive guard 捕获并替换为错误元素，控件不能正常呈现和交互。**不是进程必然崩溃**，但合法公开配置不能使用。

位置：`range_slider.rs:630–644/663`。需要联合求可行 pair，保证每个中间步骤也是合法区间，补端点、非整倍数 step/gap、非零 min 和归一化幂等性检查。不能只 clamp 到100后留下 gap 违约。

证据：[尺寸 R2-RC1](resize-controls/report.zh-CN.md)。本项由整改后的求解顺序引入。

### R2-F05：Rotatable 用 handle 几何初始化 pivot，窗口 resize 后又不重新计算

两条新变体：

- 300×220 Rotatable，90°、pivot=(0,0)，增加独立 handle 后，补偿按约26×26的操作柄计算为 `(-26,0)`；真实 Canvas 298×218 需要约 `(-258,40)`。
- 不提供 handle，实际改变窗口大小，Canvas width 从1918变598，补偿仍保持 `tx=-1066`。

位置：`rotatable.rs:101–107/432–439`、`rotatable.rhai:72–75`。首次 token 同步不等于当前 Canvas 布局下的变换有效；控制柄和内容 bounds 不能互换。

证据：[变换 R2-TS-1](transforms-selection/report.zh-CN.md)。统一以实际 Canvas 的已呈现几何与 layout revision 求 pivot，并验收有／无 handle、边框、首次值和 resize。

### R2-F06：虚拟 Sortable 源滚出可视区后仍逻辑存在，却被错误取消

100项列表、180px视口，开始拖 Item0，保持鼠标按下并真实 wheel 滚动，使窗口从 Item0..4 变成 Item2..6；业务数据、order、key、disabled、source index 均未变。再松手，没有 reorder。

新逻辑 `interaction.rs:462–472` 以本帧 presented 集合判断源存活；pin 只维持 realized，并不让离屏行经过 GPUI paint。真实卸载取消已修，但 presentation 和逻辑源身份仍未正确分离。

这不是要求继续使用被删除的源；应以具体集合成员、挂载世代与有效拖动租约保活合法虚拟源，目标仍按实际 hitbox 命中。不能恢复上一版无条件豁免，也不能用“内部 realization 也算业务修改”消解虚拟拖动 pin 的承诺。

证据：[拖放 R2-DS-1](drag-sort/report.zh-CN.md)。验收必须证明源确实离过屏，并与源删除／同 key 重建的取消对照一起通过。

### R2-F07：live-register 成功，但缓存的正式子组件不会更新

正式 child 读取未创建的 `late` 集合，catch 后展示 missing。Host 动态注册成功，再执行 dirty render，返回 `Ok(true)`，画面仍为 missing；强制不复用的 full render 才得到 ready:1。

`context.rs:253–254` 只将 root 标 dirty；`native_collection.rs:1682–1690` 在登记 reader 前就因不存在而返回；`engine.rs:2627–2629` 因而继续复用干净 child。

应追踪“读取当前不存在的名字”这一依赖，并在注册时精确失效。不能通过关闭全部组件复用来掩盖问题；缺失依赖也要支持事务回滚和卸载清理。

证据：[集合 CT2-1](collections-tree/report.zh-CN.md)。#87 的 API 已有，但数据接入到呈现的闭环仍有此缺口，建议在该 issue 的跟进中记录。

## 3. P2 确认问题

| 编号 | 问题与实际结果 | 分类／定位及证据 |
|---|---|---|
| R2-F08 | 虚拟 Sortable 边缘移动30次仍停在 Item0..4，不能自动向后滚动 | 缺少 ListState 到 auto-scroll 的适配；`interaction.rs:880–915`。既有测试只检查 source key，未证明窗口移动。[DS-2](drag-sort/report.zh-CN.md) |
| R2-F09 | SplitPane 可见 start 被约束为312，Left却从原始320减8，分隔条不动 | 键盘和 native solver 的起点／handle占用不同；`split_pane.rhai:72/77–83`。[RC4](resize-controls/report.zh-CN.md) |
| R2-F10 | 水平 marquee 仅从x5拖到50，选中x100及180的远处对象 | 新 polygon 的退化边界错误，cross全0时把共线点当内部；`selection_area.rs:507–522/556–561`。[TS-2](transforms-selection/report.zh-CN.md) |
| R2-F11 | `multiple=false, selected=[a], active=b`，Space提交[a,b] | 单选约束只修marquee，未覆盖Space；`selection_area.rs:178–185`。[TS-3](transforms-selection/report.zh-CN.md) |
| R2-F12 | PanZoom wheel开始后Escape不清preview，后续Ended仍提交 | Cancelled事件修好了，但wheel会话未进入Escape取消；`pan_zoom.rs:357–364`。[TS-4](transforms-selection/report.zh-CN.md) |
| R2-F13 | 合法1000项／depth128的Tree，仅将祖先标disabled就耗尽1,000,001 operations、无法挂载 | 新Rhai `eligible_parent` 每级祖先扫描全表；`tree.rhai:105–110/125`。enabled同数据对照通过。[CT2-2](collections-tree/report.zh-CN.md) |
| R2-F14 | 明知900k项结果超限，仍先创建900k SchemaIssue和33,188,888 bytes错误文本，再截到约1KB | 配额检查晚于全量schema遍历／格式化；`async_runtime.rs:1199–1203`。[async报告](rhai-async/report.zh-CN.md) |
| R2-F15 | 相同合法脚本只因声明capability，跳过init后执行view而错误退出 | #90剩余边界；`lib.rs:447–450` / `lifecycle.rs:181–200`。[工具 T2-1](tooling/report.zh-CN.md) |
| R2-F16 | native Automation返回Ok/invoked=1，实际callback随后throw | #85新增执行结果不一致；`app.rs:3919–3945` / `primitive.rs:906–918`。[工具 T2-2](tooling/report.zh-CN.md) |

F14 的 debug 时间约483ms只用于定位，不当作release延迟承诺；更强证据是拒绝前实际构造的错误数量和33MB中间文本。应前置无分配配额检查并限制在线schema诊断，而不是只让最终错误看起来短。

F15 不能用“捕获任意 view 错误并忽略”修复。缺 Host context 时应明确静态检查与执行验证等级，不将缺初始化状态认定为应用无效，不伪造业务返回值。

F16 不能靠第二套 resize 算法解决。native policy 应产生一个可由正确调用边界执行的语义提议；同步 Automation 必须得到执行结果，或者显式引入有完成/失败反馈的异步命令契约，不能继续把“已排队”返回为“执行成功”。

## 4. 哪些结论不应过度推导

- #86 的交付正确性已通过：限额边界、递归累计、UTF-8有界错误、无success副作用、error一次投递、订阅收尾、取消／旧generation都正常。F14是拒绝路径成本问题，不是旧loading/error routing仍未修。
- 普通布局中的 SplitPane wrapper 嵌套百分比几何对照通过。增加wrapper是合理的身份修复，不因结构多一层就要求回退。
- 同 Host 跨 View pointer与显式 keyboard drop通过；显式 `occlude()` 的普通前景能阻止背景drop。
- 无 `occlude()` 的 Normal hitbox允许后景参与，是当前GPUI和项目显式遮挡契约；模态中的显式 keyboard_target 可以指定业务对象。两者作为边界说明，不包装成未经承诺的安全漏洞。
- 负缩放、非均匀缩放与旋转的正常 enclose 对照通过。未确认invalid affine fallback导致paint/hit分裂，不将最初猜测写入finding。
- Tree原先的二次Rust全图验证已修，depth独立于expanded。F13是新增导航逻辑在Rhai中重新引入重复扫描，不能把两者混为同一个未修改算法。
- 暂不把“没有revision字段”或“没有旧awaiting-control枚举”直接判为缺陷；结论基于现行契约与实际行为反例。

## 5. 这轮对核心模型的判断

整改已有结构进展，但公共边界仍常落在某一个使用路径，而非所有生产者／消费者共同经过的位置：

| 应成为共同边界的事实 | 当前遗漏 |
|---|---|
| native生命周期访问Runtime状态的合法阶段 | 直接写lane覆盖coordinator cancel，没覆盖primitive suspend hook的读写及补偿 |
| 有效交互契约变化后的失效 | 节点事件处理取消了，timer/async提交仍可绕过 |
| 受控提议派发后的清理 | Table pointer结束清理，keyboard/autofit未共享 |
| 内容几何决定变换 | pivot初始化读了handle bounds，layout resize没有失效 |
| 逻辑源存活与呈现目标可命中 | 真卸载和源仅离屏仍被相同presented条件处理 |
| 所有选择提议满足single约束 | marquee与Space分别实现，修复没有覆盖同一出口 |
| 数据源可见性的依赖 | 成功读取有reader，缺失读取没有；register只刷新root |
| 命令执行结果 | 声明式Automation等待callback结果，native路径只确认排队 |

建议下一轮按以下顺序修，而非再按16条各加局部条件：

1. native lifecycle的可借用上下文、共同成功提交后的交互失效、统一提议清理和Automation结果。
2. 虚拟排序的逻辑源租约、实际ListState自动滚动与结束后pin释放。
3. Canvas内容几何的版本化、pivot解析和几何退化策略；共享selection约束出口。
4. RangeSlider完整可行域求解、SplitPane共用步进solver。
5. 缺失数据依赖、Tree索引化导航、异步拒绝路径早期有界检查、CLI静态与运行验证分层。

修订ADR允许“成功script rerender取消旧手势”，本轮没有把无关状态变化导致取消简单判为bug。但它有体验代价：将来后台刷新频繁时，应评估是否收窄到owner/有效契约实际变化。虚拟内部realization与业务修改也必须明确区分，否则虚拟跨屏排序的承诺无法实现。

## 6. Issue 与发布状态

本轮只读核对：#80、#82、#84、#85、#86、#87、#90 已关闭；#83、#89、#91 保持open，符合用户说明。见 [issues.json](evidence/issues.json)。没有擅自重开、评论或新建issue。

- #80保持已修结论；#84原组件ref复现与新增wrapper布局对照都通过。
- #86正确性保持已修，F14可独立记录为有界校验顺序问题。
- #85、#87、#90虽然最小复现/API已修，分别还存在F16、F07、F15的闭环问题，建议关联原issue追踪后续整改。
- #82覆盖的完整交互承诺仍受当前P1阻塞；主分支已合并，后续应以明确修复提交补齐，当前不宜据原CI/25探针通过直接发包。
- #83/#89/#91继续后续设计，未因本次审查扩大0.1.8 scope。

本报告没有声称撤销已有合并，也没有操作tag、release或crates.io。

## 7. 验证材料与下次门槛

| 组 | 新增验证结果 | 报告 |
|---|---|---|
| Runtime core | 4项原生：3通过、1失败 | [runtime-core](runtime-core/report.zh-CN.md) |
| Drag/Sortable | 5项原生：3通过、2失败 | [drag-sort](drag-sort/report.zh-CN.md) |
| Resize/Range | 6项原生：1通过、5失败 | [resize-controls](resize-controls/report.zh-CN.md) |
| Transforms/Selection | 6项原生：1通过、5失败 | [transforms-selection](transforms-selection/report.zh-CN.md) |
| Tooling native | 2项原生：1通过、1失败；另有CLI对照 | [tooling](tooling/report.zh-CN.md) |
| Collections/Tree | 真实ScriptLifecycle与operations对照；表征式输出 | [collections-tree](collections-tree/report.zh-CN.md) |
| Rhai async | 6项正向／资源边界探针通过，含新成本表征 | [rhai-async](rhai-async/report.zh-CN.md) |

**旧25项全部通过；新增GPUI原生23项为9通过、14失败。** 失败数不是独立根因数，Rhai表征测试通过也不意味着F14资源路径已经优化。每组源码、锁文件／运行器与日志均在对应目录；主报告不把源码疑点、协议选择或未实测平台当作确认缺陷。

下一次复核要求保留旧25项并加入本轮实际反例与正向对照，至少覆盖：

- 同一操作的mouse/keyboard/Automation/autofit，以及node/timer/task/subscription/Host更新。
- idle/active/pending的suspend→resume及失败补偿；wheel Escape和迟到终结事件。
- 虚拟源真正滚离可视区，逻辑删除／重建另作相反对照；实际visible range持续变化。
- pivot有／无独立handle、真正改变Canvas尺寸、退化选区、active与selected分离。
- RangeSlider可行但临近端点的联合约束、合法Tree复杂disabled祖先、动态数据源嵌套正式组件。
- 工具成功返回意味着约定的操作完成，不能只测试“invoked大于0”或“静态文字view通过”。

本轮没有重跑全部workspace/发布平台矩阵，未声称新增探针等于真实设备GUI认证。原全量门禁已由用户和整改记录提供，本轮新增失败需要在最终修复commit上与这些门禁共同验证。尚未在真实桌面完成的新交互组合必须单独记为未验证，不能由旧视觉截图代替。
