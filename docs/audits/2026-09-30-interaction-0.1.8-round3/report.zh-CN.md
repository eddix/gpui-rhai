# 0.1.8 第三轮对抗审查

日期：2026-09-30。审查基线：`815bb82b8103c88ef6ba50620128780c9d64b216`，已合并 PR #92；对照上一轮 `d77b4b49`。

## 结论

**前两轮的48个GPUI回归探针、第二轮6个异步探针及CLI init依赖用例全部独立通过，整改有效。但当前仍有确认的发布前缺口，不建议立即发包或打正式tag。** 本轮按共同整改单位归为 **7组P1、6组P2**，主要涉及原生实例身份、失效/取消的所有者、批量异步交付、虚拟行回调作用域，以及几何和资源不变量。

其中一部分是本次修复带出的新组合，例如Escape局部处理阻断pointer取消；另一部分是此前未识别的现存基础问题，例如primitive创建和存活检查使用不同key。不能将它们全部归因于最新补丁，也不能将旧用例变绿外推为这些新场景已经满足。

本轮继续按最新ADR 0022审查，不要求已撤回的旧awaiting-control设计、不把预1.0 API break当bug、不扩大到Dock/跨窗口OS拖放/任意GPUI子树变换。产品、正式测试、历史审计与GitHub状态均未修改，只新增本目录。

## 1. 历史整改验收

| 范围 | 本轮重跑 | 日志 |
|---|---:|---|
| 第一轮GPUI四组 | 25/25 | [r1-core](baseline/r1-core.log)、[r1-drag](baseline/r1-drag.log)、[r1-resize](baseline/r1-resize.log)、[r1-transforms](baseline/r1-transforms.log) |
| 第二轮GPUI五组 | 23/23 | [r2-core](baseline/r2-core.log)、[r2-drag](baseline/r2-drag.log)、[r2-resize](baseline/r2-resize.log)、[r2-transforms](baseline/r2-transforms.log)、[r2-tooling](baseline/r2-tooling.log) |
| 第二轮异步边界 | 6/6 | [r2-async](baseline/r2-async.log) |
| CLI的init依赖 | 无capability和有capability两条合法路径均通过；后者准确提示未运行依赖init的view | [cli-results.json](baseline/cli-results.json) |

[run-regressions.py](run-regressions.py) 只读取旧探针、写本轮日志；[results.json](baseline/results.json) 记录源文件SHA256和当前HEAD。没有修改旧断言来取得通过结果。

具体正向进展也已确认：生命周期read/write lease能支持正常PanZoom暂停恢复和失败补偿；虚拟列表的静止指针自动滚动已工作；源删除/重排/禁用/内容替换会撤销旧租约；新的RangeSlider pair求解在40,114组合法参数中满足bounds/gap/幂等性；超Rhai配额的数据现在先于schema检查被拒绝。

## 2. P1确认问题

### F01：primitive创建与存活检查使用两套key，RangeSlider每次重绘丢失Entity和焦点

`RangeSliderPrimitive` 构造key为`props.key`，随后`.with_key(key-range)`修改外层UiNode。`primitive.rs:1675–1683`创建实例使用PrimitiveNode.key，`:1837–1847`收集存活实例使用RetainedNode.key；同一个逻辑节点被反复卸载/重建。

实测无业务状态变化的原生刷新，focus从Some变None；点击high=80后Right不产生90的提议。仅在probe字符串中将两个key对齐，焦点跨帧稳定，同输入正确提交90。该诊断对照没有修改产品。

**应修统一实例身份来源，而不是只改RangeSlider的一行字符串。** PanZoom也有相同key失配且有Entity生命周期；Rotatable/Draggable/SplitPane存在不同字符串，但部分当前是无状态primitive，不能据此夸大为全部都已证实逐帧重建。

证据：[resize-controls R3-RC1](resize-controls/report.zh-CN.md)。验收覆盖mount/retain/dispatch/unmount一致、caller `.with_key`、无状态repaint、真正卸载、focus与callback，不只看pointer计数。

### F02：异步batch中后项失败，抹掉前项已经成功提交的交互失效事实

同截止时间两个timer：第一条成功把Draggable axes从horizontal改为vertical，第二条throw。第一条状态已提交，AdvanceTime准确报告第二条错误，但松手后旧horizontal手势仍提交`{60,20}`。

`app.rs:4999–5032`先得到包含contract_changed的结果，随后因任一delivery_error将Ok整体改成Err；只有最终Ok分支才标记交互失效。每条独立事务的已提交事实不能受整批返回状态控制。

整改应让失效记录与各次成功提交共同生效；保留后项失败回滚和首个错误汇报，不能为修复而把整批改成一个事务。证据：[runtime-core R3-CORE-1](runtime-core/report.zh-CN.md)，含单成功timer正向对照。

### F03：旧effect activation已清理，已取出的batch消息仍可执行

真实subscription一次取出两条消息。第一条改变effect依赖，旧activation执行cleanup、emitter已是`ScopeDisposed`，新工作已开始，phase=`new-query`；第二条此前取出的旧消息仍成功调用callback，把phase覆盖成`old-query-result`。

`context.rs:537–545`只能清registry和pending_async队列，不能清已取出的局部Vec；`lifecycle.rs:780–808`检查callback generation/component owner，却未确认当前effect activation有效。

应在每条消息实际执行前验证作用域租约。正常producer close后缓冲消息仍应交付，不能简化为“registry里没有entry就丢弃”。证据：[rhai-async R3-ASYNC-1](rhai-async/report.zh-CN.md)。这是本轮新识别的现存边界，不等于前两轮的payload大小修复失败。

### F04：Escape的接收者与实际交互owner不一致，取消后仍可能提交

合并两组相同取消模型的实证，不重复计数：

| 场景 | 结果 |
|---|---|
| PanZoom pointer pan后按Escape再move/up | 预览暂归零，下一次move又变tx=70，最后提交一次 |
| Second持焦，在First上wheel后Escape | 清理了错误控件；First scale仍1.10517，Ended继续提交 |
| Dialog内idle PanZoom持焦后Escape | 控件无条件stop，Dialog仍open |
| Sortable源已离屏，active lease仍存活时Escape | 自动滚动继续6→15，松手提交item0到item20前 |

PanZoom新增Escape分支只清本地wheel信号，不取消pointer owner，并无条件停止传播；Host取消又依赖当前focus祖先链，离屏源的句柄即便仍Some，也可能不在本帧dispatch tree中。源仍可见、或测试先恢复Host焦点的对照能正确取消。

需要稳定的Host/window交互owner取消入口，覆盖pointer、wheel和delayed commit；仅真正消费取消才stop。不能强制绘制所有离屏源来保住路由，也不能取消另一个Host域的手势。

证据：[transforms-selection R3-TS-1](transforms-selection/report.zh-CN.md)、[drag-sort R3-DS-1](drag-sort/report.zh-CN.md)。

### F05：Rotatable仍使用容器/旧几何，实际Canvas尺寸与active resize会使pivot漂移

正式content part将Canvas改成200×100，补偿仍按298×218 wrapper计算，实际`(-258,40)`而应为`(-150,50)`。另一场景在旋转手势中真实resize，Canvas宽1918→598，update闭包仍使用按下时旧viewport，pivot偏移约`(-529,+647)`。

位置：`rotatable.rhai:71–75`、`rotatable.rs:135–152/424`。静态token拼入宽高不会更新已捕获的gesture几何。应以绘制使用的实际Canvas矩形和同版本transform求解；几何改变时一致地cancel或rebase。

证据：[transforms-selection R3-TS-2](transforms-selection/report.zh-CN.md)。这两个新变体在前两轮静态pivot/无part override用例通过时仍存在。

### F06：虚拟行里组合正式组件，callback绑定到未存活的synthetic owner

公共virtual_collection行renderer返回正式DropZone并传入根脚本的`Fn("dropped")`。真实drop匹配目标后，调用报错：

```text
callback `dropped` belongs to an unmounted incarnation of component
`/View[audit-drag-sort]/VirtualCollection[targets]`
```

`engine.rs:3333–3340/3595–3597`的synthetic collection context只是结构命名空间，却被`caller_component_binding`用作callback owner；成功render的incarnation清理删除该非正式scope，之后正确的stale检查拒绝它。

必须区分虚拟行构建身份、正式item组件状态身份和回调原始所有者。不能关闭incarnation校验，也不能简单给synthetic scope造一个空state mount，否则根状态会在错误scope中查找。

证据：[collections-tree CT3-3](collections-tree/report.zh-CN.md)，以及其引用的实际原生drop日志。该问题不一定由本次补丁引入，是本轮扩大组合检查发现的公共能力缺陷。

### F07：missing-read tracking形成没有总量边界的持久原生依赖缓存

当前缺失读取在返回错误前把name存入missing_readers，未验证注册名格式，也没有总数量/字节预算。真实callback每次捕获64个不同缺失名的错误、不修改状态，连续调用后缓存64→128→192；render_dirty=false，因而不发生清理。使用超过Host允许长度、永远不能合法注册的名字也会留下依赖；合法短名字同样累计。

位置：`native_collection.rs:1686–1696`，注册名约束`:1617–1623`。已有单次Rhai预算不会约束跨callback累计的这份native缓存；事务snapshot还会复制其内容。

记录前验证name；明确event-phase读取是否应形成持续render依赖；为负依赖设置与组件/Runtime生命周期一致的资源边界。保留正常“child先等缺失集合、注册后精确更新”能力。probe仅使用小内存序列，没有进行耗尽试验。

证据：[collections-tree CT3-1](collections-tree/report.zh-CN.md)。这是补负依赖时新增的资源管理缺口。

## 3. P2确认问题

| 编号 | 反例 | 改进边界／证据 |
|---|---|---|
| F08 | 已接受`[20,97], max97, step10`，点击当前最右high却提议90 | pair和pointer合法值集合不一致，不是强加端点政策。[RC2](resize-controls/report.zh-CN.md) |
| F09 | Table无callback先接受200宽，第二次drag取消后退回初始160 | committed本地宽度与当前preview共用一个override，cancel无条件清None。[RC3](resize-controls/report.zh-CN.md) |
| F10 | 普通typed DragSource进入虚拟DropZone，接受lane4但边缘移动后目标仍不滚动 | virtual auto-scroll从源collection查ListState，普通源没有collection；应按目标及滚动祖先定位。[DS2](drag-sort/report.zh-CN.md) |
| F11 | 支持的scale=1e6下，20×20px框无法enclose可见5×5px对象 | 固定Canvas-local面积epsilon把正常屏幕选区当退化；容差需空间明确、尺度一致。[TS3](transforms-selection/report.zh-CN.md) |
| F12 | 合法10k节点共享depth256 disabled祖先，完全折叠只输出一行，优化原函数仍约290ms，enabled对照16.6ms | nearest enabled parent重复走公共祖先；需缓存最近祖先或parent-first传递，不能只把Rhai扫描搬到Rust。[CT3-2](collections-tree/report.zh-CN.md) |
| F13 | 通过Rhai限额的100k项Map在schema拒绝时创建100k错误和约4.2MB文本，最后才截到约1KB | 资源预检前置已修；配额内在线schema诊断仍需早期限量，离线完整报告可另保留。[ASYNC-2](rhai-async/report.zh-CN.md) |

性能观测都是定位证据，不作为跨机器帧率承诺。F12同时有逐字生产函数与源SHA；F13有确定的错误数量与中间文本字节数，判断不依赖某个debug毫秒阈值。

## 4. 核心模型判断及整改顺序

这一轮不宜继续按失败测试所在控件逐条加条件。共享边界仍有几处相同概念使用了不同的事实来源：

1. **实例身份**：native构造key、retained key、虚拟结构scope、callback owner没有统一映射。先修F01/F06，再讨论局部focus修补。
2. **交互存活与取消**：逻辑源租约已能保活，但取消输入仍依赖可能消失的focus路径；wheel和pointer的owner又各自管理。合并F04及F10的来源/目标职责。
3. **提交与过期**：事务已提交不等于整批成功；callback对象仍存活不等于它的effect activation仍有效。分别解决F02/F03，不要用整体回滚或重试掩盖。
4. **几何与合法域**：内容矩形、wrapper矩形、旧gesture矩形及不同单位下epsilon继续混用；pair normalization与pointer solver对端点规则不一致。用明确的共同坐标与可行域解决F05/F08/F11。
5. **持久资源及前台工作**：负依赖有生命周期不等于有容量边界；Rust索引有深度上限不等于每节点无需重复遍历；短最终错误不等于诊断构造有界。统一F07/F12/F13的预算与早停原则。

Table无callback的本地宽度已经成为有意支持的模式，应把它与一次手势preview分开；不得通过删除该模式规避F09，除非明确修改公开契约和调用方。

## 5. 正向证据、排除项与检查局限

- 前两轮48个GPUI探针不改断言通过；旧panic、源卸载drop、普通多View capture、原pivot/选区及原Automation callback错误都已复核。
- 原CLI init依赖现在准确走static-only结果；没有将此旧问题继续列为未修。
- 正常PanZoom暂停/恢复后可继续pan；注入后置native suspend失败，补偿和重试也正确。
- 最初的恢复对照漏了条件式Host重新组合通知。主审补上Host自身notify后保留原断言重跑，结果通过；不把这个脚手架错误算产品缺陷。
- RangeSlider生产pair函数40,114组合法输入的bounds/gap/幂等性通过；这不外推为pointer/keyboard另一套路径正确。
- source删除、投影重排、disabled和同key内容替换均会撤销旧virtual lease；普通Normal hitbox和显式keyboard_target的既定政策不重新误报。
- 负依赖的事务回滚、窗口清理、跨root精确失效通过；缺口是容量与记录条件，不是声称新字段完全不参与rollback。
- 两个并行分支在后续整理阶段被平台自动安全检查中断，提示可能涉及网络安全风险。主审核对了已生成源码、完整日志及相关产品路径并整理其报告；未完成猜测不计入结论。核心UI对照另在主审独立runner复核。

本轮没有重跑全部发布矩阵，也没有把headless原生测试当作真实设备GUI/VoiceOver/Linux桌面认证。历史CI成功和用户提供的全量结果保持原有事实，本报告新增反例仍需在最终修复commit上与全量门禁共同验证。

## 6. 证据入口与下次验收

| 方向 | 结果／边界 | 入口 |
|---|---|---|
| 核心提交 | 最终reviewed原生4项：3通过、1失败；原始脚手架误报已排除 | [runtime-core](runtime-core/report.zh-CN.md) |
| Drag/Sortable | 原生5项：3通过、2失败；另保存raw虚拟回调原始证据 | [drag-sort](drag-sort/report.zh-CN.md) |
| Resize/Range | 原生6项：3通过、3失败；通过中有1个仅probe改key的诊断对照 | [resize-controls](resize-controls/report.zh-CN.md) |
| Transform/Selection | 原生6项：6个期望行为断言失败，合并为3组原因 | [transforms-selection](transforms-selection/report.zh-CN.md) |
| 集合/Tree | 小内存ScriptLifecycle表征、原函数优化对照、raw虚拟回调源码分析 | [collections-tree](collections-tree/report.zh-CN.md) |
| 异步 | 4项表征完成；其中旧activation和完整诊断构造记录的是缺陷行为 | [rhai-async](rhai-async/report.zh-CN.md) |

本轮四组最终新增原生测试合计21项，9通过、12失败；一个通过项是明确标识的诊断修正对照，不是未修改产品的通过证明。独立根因数与测试数不同；表征测试exit0也不等于缺陷已修。

下一次至少保留历史48项并加入：无状态重绘的Entity/focus存活；mixed success/failure batch；旧effect cleanup后的已drain消息；离屏源和异控件focus的取消；virtual row正式组件callback；active resize和content part尺寸；端点点击幂等；连续操作后cancel；合法负依赖累计及有界诊断。

本轮只新增审计材料，没有创建或修改issue/PR，没有变更tag、release或crates.io。基线和PR元数据见 [evidence](evidence/)。
