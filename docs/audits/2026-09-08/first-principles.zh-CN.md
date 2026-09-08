# 第二轮审计：从运行时不变量重新推导

基线仍为 `d4a7901b26fa91311f3228d768037e1d52531e16`。本轮按维护者要求，不使用 `code-review-expert` 的分类或检查清单。保留对 pinned 依赖真实行为的源码核验和可执行探针。

先纠正范围：**0.1.1 的 break change 是维护者接受的产品策略，原 A07 撤回。** 不要求旧 API、历史版本支持矩阵或兼容层，也不建议因此改发布编号。下面的 generation/incarnation 指同一版本、同一进程内的运行时身份，不是历史版本兼容。

## 1. 从什么问题重新开始

一个可持续迭代的声明式 UI runtime 至少需要以下性质：

| 不变量 | 含义 | 本轮如何检验 |
| --- | --- | --- |
| 空间隔离 | 对窗口 B 的操作不能修改窗口 A 的局部资源 | 两个窗口共享应用状态，但树、几何、capture 必须各有所属 |
| 时间隔离 | 相同逻辑 key 的新实例不能被旧实例的操作误写 | mount→unmount→同 key remount→旧 callback/signal |
| 提交身份唯一 | 两个独立候选不能只因计数相同就互相授权 | 两个 compile、只接受一个、再投递另一个的 callback |
| 状态含义不丢失 | 一种原因的恢复不能消除另一种原因的暂停 | 显式 pause→view suspend→view resume |
| 拒绝无破坏 | 返回错误的注册操作不改变现有注册 | duplicate register 后再次使用旧 provider |
| 值域闭合 | 所有可达写入入口都保持状态约束 | constructor 与 serde、Rhai 与 Rust 路径交叉验证 |
| 交付可追踪 | 暂时背压不能变成无说明的结束或丢失 | 已排入 source 的65条消息进入64容量订阅 |

这组性质揭示了上一轮的一个盲点：**逐个模块检查“有 key、有 generation、有事务、有预算”，仍不足以证明它们组合后成立。** 需要检查这些标识究竟在哪个范围唯一，状态是否保留全部语义，以及错误和资源终态是否仍一致。

本轮新增 7 组发现，全部有公开 API 探针复现。源码在 [second-pass.rs](second-pass.rs)，完整运行结果在 [second-pass.log](evidence/second-pass.log)。其中多窗口探针模拟 production factory 共享 runtime 的结构，并手工提交几何数据；尚未重新执行真实桌面的多窗口输入认证。其余探针覆盖 Engine、ScriptLifecycle、TimerRegistry、AssetRegistry、StoreRegistry 与真正的后台订阅线程。

| ID | 优先级 | 实测发现 | 实施任务 |
| --- | --- | --- | --- |
| B01 | P1 | standalone 多窗口的 NodeId、几何和 capture 相互干扰 | PR11 |
| B02 | P1 | 组件同 key 重挂载后，旧 callback/signal 重新生效 | PR11 |
| B03 | P2 | 独立编译候选共用 generation，旧 callback 被接受 | PR11 |
| B04 | P2 | timer 丢失暂停原因，签名遗漏 callback curry | PR12 |
| B05 | P2 | AssetRegistry 重复注册返回错误但替换了 provider | PR13 |
| B06 | P2 | Store schema 和 Host signal 的写入入口绕过值域验证 | PR13 |
| B07 | P1 | 内置 receiver adapter 在背压时断开并丢弃剩余流 | PR05 |

与第一轮合并后，有效发现为 **23 项：10 项 P1、13 项 P2**；A07 不计入。编号保持稳定，不把旧编号挪给新问题。

## 2. B01：多窗口共享了本应属于单个呈现域的资源

代码：`app.rs:2013–2016, 2052–2086`；`retained.rs:229–237`；`geometry.rs:62–69, 94–103, 167–180`；`lifecycle.rs:1241–1252`。

`ScriptWindowFactory` 为 standalone 窗口复用同一个 `Rc<RefCell<UiRuntimeState>>`，这是共享 app stores 的基础。但每个窗口创建独立 RetainedUiTree，NodeId 都从1开始；共享 runtime 中的 GeometryRegistry 却只以 NodeId 为 key。`retain_geometry_nodes` 又拿当前窗口的 node 集合裁剪整个 geometry/capture registry；`begin_frame` 也清空整个 presented 集合。

复现结果：

```text
shared_windows root_id_collision=true a_child_geometry_lost=true a_capture_lost=true
shared_windows a_root_expected_width=100 actual_width=300
```

窗口 A 的子节点先记录几何并捕获 pointer；窗口 B 挂载后，A 的记录和 capture 被清除。随后 A/B 的根节点都编号1，B 写入300宽度后，A 读到的也是300。

这会影响跨节点 bounds、geometry dependencies、定位 overlay、自动化可见性、无障碍快照和 pointer capture。并非所有双窗口应用都会立刻表现错误；不读取几何或只做启动 smoke 的案例可能完全通过。独立嵌入视图拥有各自 runtime 的路径，与 standalone 多窗口共享 runtime 的路径也不能互相替代测试。

建议：把 app 共享状态与 view/scene 局部呈现状态分开。Node identity 至少包含 scene/view 域，几何、presented epoch、capture 的保留/清理操作只作用于该域。**只把 NodeId 改为全局递增不够**：跨窗口的全量 retain/clear 仍会删除对方资源。PointerId 也须包含输入所属窗口，不能只靠数字1等局部编号。

验收：两个真实 standalone 窗口具有不同尺寸和相同局部 node/ref/key；交替绘制、resize、关闭其中一个、pointer capture、分别查 bounds/automation；对 A 的无关操作不能改变 B 的几何、呈现集合、readers 或 capture。单窗口和同窗口独立 Host 域测试继续通过。

## 3. B02：逻辑 key 被同时当作实例身份和持有权

代码：`engine.rs:153–161, 1419–1433`；`lifecycle.rs:643–661`；`signal.rs:150–213, 271–281, 322–354, 372–389`。

ScriptCallback 保留 component path 与 program generation；NativeSignal 保留 component/key/kind。组件卸载后这些路径对应的资源被移除，但同一个 key 再次挂载时，新实例获得相同路径。旧持有者又能查到新资源。

复现：Counter 初次挂载，保存 callback 和 signal；隐藏并提交卸载，此时旧 signal 正确返回 stale；重新显示相同 key，使用旧 signal 写入成功，旧 callback 也被接受，并把**新实例**的 count 从0改成1。原节点从1变为3，说明发生了真实的卸载/新建，不是连续实例的普通重绘。

```text
remount_signal_was_stale=true old_signal_write=Ok(true)
old_callback_accepted=true new_instance_count=Some(Integer(1))
```

建议区分三类身份：逻辑定位路径、一次挂载的 incarnation、程序 generation。callback/异步交付应绑定 incarnation，并在真正执行时验证仍存活；不能只在最早入队时检查。signal 等写句柄也应明确是“绑定某个活实例”还是“持续按名字解析的 locator”。如果需要后者，应给它独立的 API 语义，避免伪装成卸载后永久失效的句柄。

探针也观察到旧 ElementRef 会解析到新 NodeId。**这条观察没有单独判为 bug**：项目已经有“ref 跟随逻辑目标替换”的设计，需要区分同一活组件内的节点替换与整个组件死亡再重建。不能为了修 stale callback 破坏 A 系列已验证的 ref 几何自恢复。

验收：旧 callback/task/timer delivery/signal 在 owner 卸载后不能修改新 incarnation；连续 rerender、keyed move、允许的程序热迁移仍保留该保留的状态。测试应交付旧句柄，而不只是检查卸载瞬间的注册表为空。

## 4. B03：generation 只表达“下一次提交计数”，不能区分候选

代码：`engine.rs:620–635, 645–663, 1419–1433, 1708–1714`。

`candidate_generation()` 根据当前已提交 generation 返回 +1；compile 本身没有为独立候选保留唯一身份。A/B 在任何一次 render 之前编译，都获得 generation1。B 成为当前程序后，A 的 callback 仍通过检查。

```text
candidate_alias a=1 b=1 stale_a_with_a=Ok("a") stale_a_with_b=Ok("b")
```

向 invoke_callback 传旧 AST A 时执行了 A；传当前 AST B 时，同一个旧 callback 名称解析到了 B。这是公开 Engine API 的可复现行为；本轮没有声称高层 FileScriptView 的每次顺序重载都必然走到该状态。

建议用不可混淆的 program/candidate identity 建立 CompiledUi、callback 和当前激活程序的关系。需要保留 module preload 与 entry 共享候选环境的能力，因此**不能机械地让每次 compile 都递增 generation**；应先定义 preparation session，区分属于同一程序的多个 AST 和多个独立候选。

验收：预编译 A/B 只提交 B、失败候选后重试、不同 Engine 的同名函数、同一准备会话内的 module preload；外来或落选 callback 必须拒绝，合法跨模块回调仍正确解析。

## 5. B04：timer 状态把不同含义合并了

代码：`timer.rs:99–114, 186–211, 263–284`；对应说明 `docs/component-authoring-guide.md:374–380`。

第一处是暂停原因：`pause()` 设置 interaction_paused；view suspend 也设置同一个标志；resume_component_scope 无条件将它清为 false。原本因用户交互主动暂停的 timer，在整个 view 恢复后会自行运行。

第二处是执行签名：TimerSignature 仅包含 delay、callback 函数名、payload。`Fn("fired").curry(1)` 已完成后，换成 `.curry(2)`，描述的执行行为已变，但 registry 判断签名相同，timer 不重新启动。

```text
pause_before_suspend_restored=false unexpectedly_fired=1
changed_curry_completed_timer first_fired=1 rearmed_active=0
```

建议用暂停原因集合或独立 reason 状态表达 declaration、interaction、view suspension。恢复一个原因只移除该原因；签名使用完整、明确的 callback 身份，包括影响行为的 curry/provenance，generation 是否重启则按现行 timer 热迁移契约明确决定。

验收：用户暂停和 view 暂停的全部进入/退出顺序、重复 suspend/resume、暂停中改变 declaration、完成后改变 curry；同时保证未变声明不重复触发。采用 ManualRuntimeClock，不用 sleep 等到“好像恢复”。

## 6. B05：重复注册的错误结果与真实状态不一致

代码：`asset.rs:267–290`。

`AssetRegistry::register` 先 `insert` provider，再通过返回的旧值判断重复。如果重复，函数返回 DuplicateNamespace，但新 provider 已经取代旧 provider。已缓存图像不变，后续未缓存图像则从新 provider 加载，形成混合命名空间。

复现：旧 app provider 有 a/b；缓存 a；注册空 provider，收到重复错误；a 继续成功，而此前合法的 b 已加载失败。

```text
duplicate_provider_rejected=true cached_a_ok=true previously_valid_b_ok=false
```

建议在插入前验证或使用 Entry，保证拒绝时零替换；若要支持替换，使用单独、语义明确且缓存处理一致的 replace 操作。不要靠调用者预查来弥补 register 自身的错误后置条件。

验收：拒绝重复后，新旧未缓存资产、已缓存资产、provider 调用来源均保持不变。推广检查“返回 Err 的 mutator 是否仍修改状态”，但不要把使用临时局部 map 的构造函数误判为同类问题。

## 7. B06：校验放在构造函数里，不代表已接收状态永远有效

代码：`state.rs:106–127, 345–354`；`store.rs:72–82`；`signal.rs:322–354`。

ComponentStateSchema 可从 serde 构造，绕开 `new()`。StateStore::mount 会重新验证，StoreRegistry::declare 却直接复制 defaults。相同的 integer schema + string default，在 component state 路径被拒绝，在 app store 路径被接受并读取出 String("wrong")。

另一个交叉入口是 SignalValue：Rhai 的 from_dynamic 拒绝非有限浮点，但 Rust 可直接构造 `SignalValue::Float(NaN)`，SignalRegistry::write 只检查 kind，写入成功。这个新证据扩展了原 A14 的值域问题；没有据此声称已经触发 GPU 崩溃。

```text
bad_schema_component_rejected=true store_accepted=true value=Ok(String("wrong"))
host_signal_nan_accepted=true
```

建议将 validated schema/value 与外部未验证表示区分，或者在每个公共接收/写入边界复核统一验证函数。公开 enum/serde 允许绕过 constructor 时，constructor 检查不能成为唯一防线。这里保护的是宿主正常接入时的正确性，不是在把可信 Rust 当作敌对代码。

验收：同一非法数据经 builder、serde、Rhai、Rust host API 得到一致拒绝；拒绝不改变旧状态。合法数据的性能避免不必要的整树重复转换，可缓存验证过的不可变 schema。

## 8. B07：内置流桥接把背压当作终止

代码：`capability.rs:130–140`；`async_runtime.rs:409–417, 463–474`；`context.rs:2426–2432`。

`SubscriptionWork::from_receiver` 对 `emitter.emit(value)` 的任何 Err 都 break。bounded FIFO 满时产生的是 Backpressure，但 adapter 直接丢弃刚取出的值、退出工作并销毁 receiver；上游剩余值也不可达，随后被记录为 WorkReturned。

使用真实后台 subscription，预先发送65个值，将容量设为64，并在 worker 结束前不 drain；source 随后断开，只拿到64条，订阅活动数为0。轮询 disconnect 所需的额外探测值不影响前65条中的第65条已经丢失这一结论。

```text
receiver_adapter preloaded=65 source_disconnected=true delivered=64 active=0
```

这比第一轮 A05 的 host 批次失败更靠近源头：即使所有 callback 都成功、没有 reload/unmount，也能丢消息。`emit` 返回背压本身是合理契约，错误在 runtime 提供的 adapter 没有遵守它。

建议让内置 adapter 保留未成功入队的当前值，等待容量或取消，再继续；或者提供异步、有界、可取消的 send 接口。Closed、Backpressure、Poisoned 应有不同处理。不要忙等，不要把默认策略改成 Latest 来隐藏丢失。

验收：容量1/64/4096的持续 burst，消费慢于生产、暂停恢复、取消唤醒、producer正常关闭；所有已接收序号有明确去向，完整 FIFO 模式既不静默丢值也不默默终止流。把该测试并入 PR05 的端到端交付模型。

## 9. 对架构方向的再判断

新增发现集中在三个根因，值得先修这些根因，再处理表面 API：

1. **作用域没有成为类型和存储结构的一部分。** AppRuntime、ViewRuntime、Program、ComponentIncarnation、SceneNodeId、EffectActivation 的生命周期不同。当前有些地方用路径、局部整数或 global Rc 代替这些域，代码表面“有身份”但实际不够唯一。先画清拥有关系，再统一 helper；不必立即拆 crate。
2. **状态丢失了决策所需的信息。** 一个 pause bool 无法表达多个原因；只有函数名无法表达 callback；ID 集合无法恢复取消过的资源；一个 generation counter 无法区别独立候选。修复应补足模型，避免增加更多特例判断。
3. **测试 oracle 与实现共享假设。** 检查一张表为空、函数返回 Err、数字 generation 变了，都可能绕过真实问题。新增测试应验证另一个窗口、旧句柄、新实例、上游 producer 等独立参与方的观察。

推荐三类变形测试作为持续迭代的基础：

- **无关域不变性：** 加一个窗口、无关 store 或 sibling component，原场景观察结果不变。
- **连续性与新生性：** rerender/keyed move 保留身份；unmount/remount 创建新实例；合法热迁移按明确规则处理，三者不能只看相同 key。
- **守恒与拒绝后置条件：** 输入序号最终已提交、待处理或明确拒绝；返回错误后的旧资源仍可用；恢复某一个暂停原因不会消除其余原因。

本轮也没有把下列事项自动当作 bug：0.1.1 有意破坏旧 API；应用自有脚本不提供进程沙箱；HostCallback 的外部效果不可逆；logical ElementRef 可能有意跟随目标；GPUI 未开放的 AX 能力；Rhai 的可选 Grain 仍处于实验阶段。问题必须来自当前承诺和可观察行为，而不是“成熟框架通常还应该有什么”的清单。

## 10. 按当前项目阶段控制整改规模

有限使用者和快速迭代意味着现在适合直接修正错误模型。优先让 view 局部资源具有明确归属，让活实例句柄具有存活身份，让执行与交付具有可验证的提交结果；这些改动即使调整公开 API，也符合维护者当前策略。

建议把下一轮实施聚焦在四组行为：多窗口不干扰（B01）、旧操作不误写新实例（B02/B03）、失败后保持一致且消息不丢失（A03–A06/B07）、预算与计量真实（A01）。B04–B06属于可单独完成的小范围正确性修复。之后再用A08的结构基准指导性能调整。

前一轮的CI清单统一、全平台认证、供应链自动化、CLI进程崩溃恢复等建议保持为分层待办。先实现满足实际故障模型的最小恢复能力；如果暂不承诺跨进程崩溃原子性，就如实声明并保留可恢复的staging状态，不必先引入完整事务日志系统。公共面收缩也可以直接进行当前版本重构，不承担历史迁移层。

评估一个建议是否值得现在实施，使用三个问题：它修复哪个已复现行为；它新增多少必须长期维护的状态和规则；能否用更小的结构改动满足同一个不变量。审计条目数量不应变成一次性扩大框架规模的理由。
