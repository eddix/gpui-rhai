# 0.1.8 对抗式审查：发布结论、核心模型与 Issue 分级

日期：2026-09-30。主审与六个并行方向共同核验，最终以本报告和相应可复现证据为准。

## 1. 发布结论

**不建议合并或发布当前 0.1.8 候选。** 已确认两类可达 Host panic，以及跨 View 捕获失效、卸载源继续 drop、独立列表串线、不可见目标接收 drop、旧手势覆盖新受控状态、变换／命中不一致和 Tree 合法用法失败。它们不是预 1.0 breaking change，也不属于已排除的 Dock、OS 跨窗口拖放或任意 GPUI 子树变换。

架构方向可以保留，但当前“统一”主要完成了公共函数和部分事件循环的集中。**身份、生命周期取消、受控提议的修订关系、已呈现几何和集合归属还没有形成一致的契约。** 如果仅按控件逐个加条件，类似问题会继续在组合场景出现。

审查对象与外部状态：

- HEAD：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`；对照 `v0.1.7` / `1181d701055110cfaa7135348af3d711eece9aee`。
- 分支：`codex/interaction-runtime-0.1.8`；[PR #81](https://github.com/eddix/gpui-rhai/pull/81) 的远端 head 与本地一致，抓取时为 Ready / mergeable。
- Rust 1.95，gpui-pre core/platform `=0.3.7`，Rhai `=1.26.0`；当前 Runtime API 2。
- 改动规模：85 文件，约 11,805 行新增、1,242 行删除。按风险边界分工，不以行数评价设计。
- 只读拉取了当时全部 **10 个 open issues** 的正文和评论，以及 PR #81/#88 信息。快照在 [evidence](evidence/)，时间为 2026-09-30 06:48 UTC。
- PR #81 的远端 portable job 成功，见 [CI 明细](evidence/pr-81-ci-job.json)。这与下述反例并不矛盾：现有门禁没有覆盖这些条件组合。

本轮没有修改产品代码、正式测试、Issue/PR 状态，也没有提交、合并或发布。新增内容仅为此审计目录中的报告、独立探针和证据。

## 2. 发布前必须解决的确认问题

编号按本轮主报告统一；分报告里的局部编号保留，以便定位原始复现。相同根因的多种表现合并，不把每一个失败测试都算成一个架构问题。

### F01 · P1：活动手势中 suspend 触发 Entity 重入 panic

Draggable 按下并移动后，正常调用 `view.suspend(window, cx)` 即报：

```text
cannot update gpui_rhai::app::ScriptHostView while it is already being updated
```

`app.rs:1021` 的 Entity update 内同步进入 `quiesce_view → cancel_view → cancel callback`，后者清理 preview 时又经 `write_signals` 在 `app.rs:3284` update 同一 Entity。`interaction.rs:783` 没有约束回调所处阶段。

这是正常生命周期组合即可触发的 Host 崩溃。整改应定义安全、幂等的取消与预览清理阶段，并保持 generation 校验；不能只 catch panic，或在某个控件临时 defer 后留下旧 cleanup 清除新手势的竞态。

证据：[尺寸／范围控件 RC-1](resize-controls/report.zh-CN.md)、[panic 栈](resize-controls/suspend-backtrace.log)。

### F02 · P1：同 Host 多 View 的 capture 路由被最后绘制者覆盖

第一个 View 的节点捕获 pointer 后，移到边界外再松手：单 View 对照收到 move/up；加上同 Host 的第二个纯文字 View 后，第一个 View 一直停在 `down`。

`interaction.rs:415` 的 `set_pointer_routes` 只保存两个 Option；`renderer.rs:1049/1171` 每个 View 都覆盖它们。应维护按实际捕获 owner 选路的 View 注册表，而不是要求调用方改成每个 View 一个 Host 来绕开。

证据：[核心 RC-1](runtime-core/report.zh-CN.md)、[原生输出](runtime-core/probes.log)。

### F03 · P1：DragSource 已卸载，旧 payload 仍可提交

拖动开始后通过正常 Rhai 状态事务移除源节点，下一帧已经显示 `source removed`；随后在目标松手，仍收到 `dropped:deleted-resource`。

`interaction.rs:429` 对 application drag source 豁免 presented 消失检查；`drag_drop.rs:498` 的卸载没有终结协调器中的 session。必须区分“虚拟项暂时未呈现”和“逻辑 mount／源数据已不存在”。后者无条件取消，不能让相同 key 重建后的实例继承旧会话。

证据：[核心 RC-2](runtime-core/report.zh-CN.md)。

### F04 · P1：Sortable 的实例身份丢失，合法局部 key 复用导致跨列表操作

两个不同 formal `AuditPanel` 实例内部各使用 `key:"queue"` 的 Sortable；它们不是重复兄弟 key。把左列表的 `left-a` 拖到右列表 `right-b` 后，右侧收到 `left-a:after:right-b`，尽管该列表没有 left-a。

`sortable.rhai:140` 将局部 key 当 list_id；`sortable.rs:248/330` 把它直接当 Host 范围的排序通道。业务 key、payload type 与挂载实例身份应分开，目标必须验证同一列表实例及有效源成员。不能要求封装组件作者为所有内部 key 生成应用全局名称。

同一缺口影响虚拟 pin：`virtual_list_element.rs:105–107` 只按全域 `source_id` 查找每个列表，并在每次 prepaint 扫描全部数据。即使拖动来自其他列表也如此。优化微基准中，单个 100k/1m 列表仅查键的中位成本约 319µs/3.20ms；这不是完整帧耗时，但证明它随总数据量增长。应按 collection instance 路由，以投影 revision 的索引／缓存定位，不能把“没有逐帧 Rhai”当成“有界热路径”的充分条件。

证据：[拖放 DS-2](drag-sort/report.zh-CN.md)、[集合查键证据](collections-tree/pin-scan.log)。

### F05 · P1：Drop 命中不遵守裁剪和前后遮挡

两个原生反例：完全位于父 clip `Y=0..80` 之外的 DropZone（实际在 `Y=120..180`）仍收到 drop；两个相同 bounds/priority 的重叠目标，后景 `zz-back` 战胜后绘制的 `aa-front`，目标名字决定了结果。

`drag_drop.rs:290/323` 丢弃已建立的 hitbox，仅登记布局矩形；`interaction.rs:787/807` 用矩形、priority 和面积挑目标，没有 clip／绘制次序。必须先得到本帧实际可命中的目标集合，再应用业务 priority 与嵌套政策。不能用 key 排序代替画面层级。

证据：[拖放 DS-1](drag-sort/report.zh-CN.md)。键盘可达性与指针可见性应分别定义，不能为过滤指针目标把所有不可见语义对象一并删除。

### F06 · P1：缺少统一 source/constraint/ack 修订，旧预览覆盖新状态

已确认三个表现：

- RangeSlider 拖动 `[20,80]` 的 high 到 70；松手前 Host 将 source 改成 `[10,40]`，松手后旧手势又提交 `[20,70]`，连 low 都回退了。
- Resizable 拖动中把 max_width 从 360 收紧到 220，source rect 不变；旧 closure 仍提议 width=300。
- Table 的受控 width 保持 160、callback 明确不接受提议；拖动 40 后分隔条永久留在新位置，重渲染同值未使预览回退。此项是既有缺陷，也是本轮 ADR 承诺应统一的验收缺口，未误称为新增回归。

定位：`range_slider.rs:113–124/211–218`、`resizable.rs:234/344–401`、`column_resize.rs:154–167/275–282`。

应统一 source revision、constraint revision、gesture generation 与 pending proposal 的关系；明确变化时 cancel 或整体 rebase，区分接受、钳制、拒绝和迟到回复。数值相等无法证明拒绝已被处理，也无法证明旧回复仍属于当前手势。

证据：[尺寸／范围控件 RC-2](resize-controls/report.zh-CN.md)。

### F07 · P1：有限但极大的合法 PanZoom 参数直接导致 Host panic

公开参数 `scale=max_scale=1e308`，内容为普通 300×220 Canvas，所有参数都是有限数且通过 schema；绘制却在 `canvas.rs:584` 的 `expect` 崩溃：`validated canvas bounds produce a finite affine origin: InvalidTransform`。

输入有限不代表围绕视口中心组合后的矩阵仍有限。需要验证组合、逆变换、最终坐标范围，并以诊断／安全拒绝替代用户数据可达路径上的 expect；不能只限制本例常量。

证据：[变换 TS-1](transforms-selection/report.zh-CN.md)。

### F08 · P1：已呈现 affine 的消费者仍各自近似或跳过初始化

**Rotatable 初始 pivot 错误。** `rotatable.rhai:49–53` 将补偿平移初始化为 0，却把 source token 初始化为已匹配；`rotatable.rs:424–431` 随即跳过第一次同步。首次挂载 90°、pivot=(0,0) 仍按 Canvas 中心旋转；实际 298×218 内容需要约 `(-258,40)` 的 pivot 补偿，得到的却是 0。

**旋转框选几何错误。** `selection_area.rs:106–110/442–450` 只逆变换鼠标起终两点，再构造局部轴对齐矩形。Canvas 旋转 45° 后，窗口框选矩形会对应一个四边形，两点不足以定义它；实际完全包含的目标 a 没有被选中。

需要统一首次呈现、source／布局变化后的有效变换；选区必须在正确空间做完整相交／包含判断。把四角简单包成 AABB 也不能普遍实现精确的 enclose/intersect。

证据：[变换 TS-2 / TS-3](transforms-selection/report.zh-CN.md)。这两项都只使用当前支持的 Canvas，不依赖任意 GPUI 子树变换。

### F09 · P1：Tree 合法默认状态无法挂载，正常折叠也会失败

`active_key` 可省略／为 null，但 `tree.rhai:120–123` 总写入 `reveal_key:()`；底层只接受缺省字段或字符串，最小合法 Tree、空 Tree 都报错。active=child 时折叠其 parent，新投影不再含 child，仍把它作为 reveal_key，同样报错回滚。

有效 active、显示、键盘操作与 reveal 必须解释为同一对象；无有效 reveal 目标时不能构造非法虚拟集合配置。受控状态的修正应有明确的提议／fallback 政策，而不是要求调用方补隐藏约束。

证据：[集合 CT-1](collections-tree/report.zh-CN.md)、[真实 ScriptLifecycle 输出](collections-tree/tree-lifecycle.log)。

### F10 · P1：Tree 全图验证为二次复杂度，折叠状态绕过深度限制

`collection_projection.rs:108–115` 从每个节点重新沿祖先链走到根；depth 上限直到 `:153` 的可见投影阶段才检查。完全折叠的 10k 深链被接受，只输出一行，却检查约 5,000 万次祖先。

逐字摘取产品函数并 `rustc -O` 的微基准：3k 约 591ms、6k 2.66s、10k 8.77s。真实 RuntimeEngine debug 也确认公共脚本入口执行该路径。时间在并行审查中取得，不作为精密发布基线；算法复杂度和前台秒级阻塞已有充分证据。Rhai 操作预算不能打断这个 native 内部循环。

应将 cycle/depth 校验与 expanded 分离，在全图结构阶段以有界访问完成；可见投影随后只决定展示哪些行。优先用确定性访问计数与最大深度断言验证，不通过放大脚本预算解决。

证据：[集合 CT-2](collections-tree/report.zh-CN.md)，含源 SHA、优化微基准与实际 Runtime 入口。

### F11 · P1：RangeSlider 的 gap 改变 high thumb 的步进网格

min=0、step=5、minimum_gap=7、source=[20,80]，点击 high=75，实际提议 `[20,77]`。

`range_slider.rs:175–181` 将 low+gap 用作 normalize 的 min，但 `range_input.rs:514–516` 也将 min 当作 step 原点，导致 high 网格变为 27+5n；受控值再次归一化又使用全局原点。应分离网格原点和可行边界，对两个 thumb 联合求解，保证提交与再次呈现一致。

证据：[尺寸／范围控件 RC-3](resize-controls/report.zh-CN.md)。分报告另有一个确定性源码算例可导致 low 小于 min，未将它伪装为额外跑过的原生测试。

### F12 · P1：复合组件夺取 caller key/ref，破坏内部组件身份（#84）

Formal Cell 根节点有自己的 `cell_root` ref：普通 box 容纳它正常，作为 SplitPane.start 或 Draggable.handle 后，调用自身 `element_bounds("cell_root")` 报 ref 不存在。

`split_pane.rhai:123–126` 和 `draggable.rhai:92–96` 直接改 caller node 的 key/ref/style。自有 part、几何引用和 native preview 应落在组件自己的 wrapper，调用方身份不能被接管。

证据：[尺寸／范围控件 RC-4](resize-controls/report.zh-CN.md)。这是旧问题在新增组合中的扩大，不是历史 API 兼容要求。

### F13 · P1：异步结果通过 schema，却不能被脚本安全消费（#86）

`async_runtime.rs:493/1166` 只验证 schema，`:1191` 无界封装 Host error。超大 success、超长 error、10,001 项数组、嵌套累计字符串和 subscription 都能令交付失败，应用继续停在 loading。

更重要的是，Rhai 的尺寸检查可以在读取参数时才触发：callback 先执行 Host capability，再读超大参数，会出现外部副作用已发生、UI 状态随后回滚。不能在失败后重试 success 或补跑 error；应在选定并执行 callback **之前**完成可交付性校验，产生自身也有界的错误对象。

证据：[Rhai / async 报告](rhai-async/report.zh-CN.md)。这修正了原 issue 对“任何 callback 代码都不会开始执行”的过度概括。

### F14 · P2：CLI check 错拒已声明 capability 的合法 init（#90）

当前 HEAD 构建的 CLI，生成的干净项目 check 成功；在 app.toml 声明 `app.demo`、init 调用后，却报 “was not declared”。`lib.rs:438` 在读取 app manifest 前就执行 validate_entry；`:1011–1087` 的自建运行环境没有 Host handlers／manifest activation。

需要先定义静态检查与需要 Host context 的动态验收边界。不能为了让 check 通过而虚构成功数据，也不能把未执行的动态检查说成已通过。render 阶段同步 capability 的既有禁止规则继续保持。

证据：[CLI 输出](cli-check/results.json)；完整整改方向见 [Issue 分级](issue-triage.zh-CN.md)。本项可绕行，紧急程度低于 panic，但建议同轮修复公开开发流程。

## 3. 其他已确认的功能／可测试性问题

以下均有独立探针，不应丢在“已知小问题”中被无条件带过；优先级 P2，与上述基础整改一起补回归。

| 问题 | 实际结果 | 证据 |
|---|---|---|
| SelectionArea 单选不变量失效 | `multiple:false` marquee 仍提交 `[a,b]` | [TS-4](transforms-selection/report.zh-CN.md) |
| SelectionArea 空白点击不能清除 | 先选 a 再点空白，仍为 a；清空分支条件反向 | [TS-5](transforms-selection/report.zh-CN.md) |
| PanZoom modifier wheel 取消被过滤 | Ctrl 开始后松 Ctrl 再 Cancelled，预览仍为 1.10517，source=1 | [TS-6](transforms-selection/report.zh-CN.md) |
| 虚拟 Sortable 键盘不可用 | 点击后 focus=None，Alt+Down/End 无事件；bounded 对照和虚拟 pointer 均正常 | [DS-3](drag-sort/report.zh-CN.md) |
| Tree disabled active 与动作对象分裂 | Left 提议 disabled parent；接受后显示 parent，Enter 却选 child | [CT-3](collections-tree/report.zh-CN.md) |
| Resizable 固定比例角柄忽略竖向意图 | 200×100、2:1，SE Down 后仍宽 200，而不是216 | [尺寸 RC-6](resize-controls/report.zh-CN.md) |
| Resizable 自动化键盘入口缺失 | 正确定位 separator，Dispatch 返回 invoked=0，尺寸不变 | [尺寸 RC-5 / #85](resize-controls/report.zh-CN.md) |

虚拟 Sortable 的首尾目标未 realized 时如何完成键盘 Home/End，仍有独立静态疑点；本轮动态 Alt+End 被前置焦点故障挡住，没有重复计算为第二个已证实缺陷。Tree 原生层级 AX 元数据也建议专项验收，本轮没有 VoiceOver 实测，未把它升级为确认缺陷。

## 4. 对“核心模型是否真正统一”的回答

**尚未完成；现在应按共同约束整改，而不是继续增加同类控件分支。** ADR 0022 本身已经抓住了主要方向，缺口在实现落实。

| 模型 | 已有基础 | 尚未闭合的事实 | 对应问题 |
|---|---|---|---|
| 所有者身份 | Host coordinator、部分组件自行拼 component path | `InteractionOwner` 没有统一的 retained/mount incarnation；Sortable 内部通道使用局部 key；View route 最后写入者获胜 | F02–F04、F12 |
| 生命周期 | 公共 cancel 回调、frame-local presented 注册 | 同步取消可重入；presentation 缺席与逻辑卸载混淆；不同组件清理阶段不一致 | F01、F03、wheel Cancelled |
| 受控提交 | 原子 signal batch、deferred semantic proposal | 没有公共 source/constraints/proposal generation 协议；不同组件用不同的值比较猜接受与拒绝 | F06、F11 |
| 呈现几何 | 公共 Affine2D、Canvas paint/hit 路径 | Drop 忽略 clip/z-order；pivot 初始值绕开解析；框选拿两个点近似一个区域 | F05、F07、F08 |
| 集合状态 | outline/flat projection、virtual realization | active/reveal/disabled 的有效对象不一致；结构验证随展开状态变化；pin 不绑定具体集合 | F04、F09、F10 |
| 脚本边界 | retained context、schema、预算、事务 | 可交付性不在调用前验证；CLI 仿造了一个缺 Host 的运行路径 | F13、F14 |

精确区分：并非所有组件都存在相同局部 key 碰撞。Draggable 的字符串中已经包含 signal component path；DragSource/Sortable 的做法不同。问题是身份保证由各个调用者自行补，框架没有提供统一且带挂载代际的结构化身份。

`GestureSession` 当前只有坐标、phase、threshold 等字段，没有 ADR 描述的 source revision、pending proposal 或 awaiting-control；批量信号写入正确也不等于 controlled acknowledgement 已成立。

建议按五个整改单元交付：

1. **所有权与取消阶段**：框架提供结构化 owner；Host routes 按 owner 分发；逻辑 mount 与本帧可见性分开；取消、预览清理与 Entity transaction 不重入、可幂等。
2. **受控修订协议**：定义完整 source/constraints/gesture/proposal 身份及 accept/clamp/reject/cancel。晚到消息不得作用于后续手势；行为只提供各自的纯计算政策。
3. **有效呈现几何**：同一帧提供经过范围验证的变换、逆变换、clip 和目标可见性；Drop、marquee、pivot 都消费它，避免重新构造第二份近似几何。
4. **集合身份与投影**：stable key 绑定具体集合和投影 revision；active/reveal 共用有效对象；全图验证有界；pin 不逐帧扫描无关数据。
5. **交付与验收边界**：异步结果调用前可接收性验证，CLI 检查等级明确，Automation／真实键盘到达相同语义动作。

保留不同领域的产品语义：Chart 相机、Scroll offset、RangeSlider 值、矩形约束不应塞进一个万能 solver。当前中央协调器和 typed boundary 已经足以承载上述约束，不需要再新建一套平行 Runtime。

## 5. Issue 与 PR 的本轮处理建议

完整逐项理由、修法和外部链接见 [issue-triage.zh-CN.md](issue-triage.zh-CN.md)。简表：

- **本轮必须闭合：#82、#84、#86、#90。** #82 当前不能宣称完成；新增对抗缺陷均是其验收的一部分。
- **#80 修复通过独立验收**：保留，最终合并后关闭，不提高同步递归／操作上限。
- **本轮建议增量：#85、#87 的最小 Host live-register。** 优先级低于全部发布阻塞；#87 暂不加入脚本构造任意集合的接口。
- **后续版本：#83、#89、#91。** 分别是自定义 grip、Table context request、来源／用户激活模型，不能为关闭 issue 仓促扩大本轮修复面。
- **建议纳入 PR #88**：对每前置节点深拷贝 realized items 的性能根因有针对性修复。精确 head 的远端 native/package/Linux smoke 已成功，本轮未合并或本机执行该补丁；最终集成后重跑门禁。它不等于修复所有 fling 空白行。

这些新缺陷目前只在审计材料中记录，没有擅自替用户批量创建 GitHub issues 或评论。

## 6. 已证实的正向结果与测试盲区

正向结果：

- #80 真实 task completion 连续 200 跳成功，各次操作配额独立；同回调内真实递归和无限循环仍被限制。
- 单 View capture、保留有效源的正常 drop、bounded Sortable keyboard、普通 wrapper 中的 formal ref 等正向对照通过，反例不是普遍无效的测试输入。
- signal batch 的源码路径先验证全部成员，再同一 foreground update 内写入并 notify；本轮未发现这一段部分写入的反例。不能将其外推为所有 gesture 事务都已正确。

盲区说明：

- 现有 tests 大量使用默认角度 0、无旋转矩形、同步接受提议、单 View 和当前屏幕可见目标，恰好避开本轮反例。
- `workbench/interaction-lab` 中 selection、pan 和 rotate 位于不同 Canvas；一页展示多个组件不能证明它们在同一对象空间真实组合正确。
- “pointer 热路径 Rhai operations=0”没有覆盖 native 全量扫描、深链校验、几何有效性和迟到状态覆盖。
- 新增原生 probe 使用真实 GPUI test window、输入与产品 registry，优于只看 source，但不是实际设备手势、Metal 截图、VoiceOver 或 Linux 桌面认证。

## 7. 证据目录与复核门槛

| 审查方向 | 本轮独立结果 | 报告 |
|---|---|---|
| Core routes / 生命周期 | 4 个原生 probe：2 正向通过、2 正确性断言失败 | [runtime-core](runtime-core/report.zh-CN.md) |
| Drag/Drop/Sortable | 5 个原生 probe：1 正向通过、4 正确性断言失败 | [drag-sort](drag-sort/report.zh-CN.md) |
| 尺寸／位置／范围控件 | 10 个原生 probe：1 正向通过、9 失败，含产品 panic | [resize-controls](resize-controls/report.zh-CN.md) |
| Affine/PanZoom/Rotation/Selection | 6 个原生 probe 全部违反期望，含产品 panic | [transforms-selection](transforms-selection/report.zh-CN.md) |
| Tree/集合 | 实际 RuntimeEngine／ScriptLifecycle 复现，优化原函数微基准与查键基准 | [collections-tree](collections-tree/report.zh-CN.md) |
| Rhai async | 5 个特征探针通过；既含正确修复验证，也含缺陷稳定复现 | [rhai-async](rhai-async/report.zh-CN.md) |
| CLI #90 | 当前 HEAD 构建；正常项目通过、声明 capability 错拒、无效 view 正常拒绝 | [cli-check](cli-check/results.json) |

合计四组 GPUI 原生 probe 为 **25 项，4 个正常对照通过、21 项期望正确行为的断言失败**。该数字不是 21 个相互独立根因，也不包含全部既有测试。Rhai 的特征测试通过表示对应描述被证实，不能混算为产品整改通过。

下一次验收至少要求：

1. 本报告所有确认失败改为期望正确行为通过，保留正常对照；同源探针迁入正式 suite，不把断言改成接受旧错误。
2. 加入状态交叉矩阵：同 Host 多 View、同名不同实例、拖动中源／约束变化、卸载／暂停／关闭、裁剪／遮挡／滚动、非零初始 affine、旋转框选、空 Tree／折叠 active／disabled、回调大数据边界。
3. 更新 ADR 与实际实现一致；若要收缩某项公开能力，明确修改契约和展示，不用 release notes 宣称尚不存在的通用模型。
4. 在最终修复 commit 上运行既有 workspace、default/charts、独立 native/performance、严格 Clippy、rustdoc、package、release smoke／artifact audit 与适用真实平台矩阵。
5. PR #88 如纳入，证明性能修复未改变结果；对新增 native O(N) 热路径使用结构性断言和受控基准，而非只看平均帧率。

没有重跑本地全量发布套件：本轮针对当前精确基线执行对抗探针，并核验远端既有 CI。已有清晰失败时，重复同一套正常路径全绿不能代替整改。
