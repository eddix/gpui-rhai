# 0788e3c 整改验收与 issue #42 定位

验收对象：`0788e3cbfa243b6e985b2a554c310aa29c33ca5f`（Harden runtime identity and transactions）。对照审计归档提交 `29482d5`，53个文件，新增3,381行、删除746行。验收开始和测试期间产品工作区干净。

**结论：暂不通过整体验收。修复方向有实质进展，多项旧缺陷已消失；但有5个已确认问题，以及1个需修正并补并发测试的竞争条件。** A07继续撤回；不检查历史API兼容，不要求兼容层或改变0.1.1发布策略。

本次仅新增验收材料，没有修改被验收的产品代码，也没有向issue发送评论或关闭issue。

## 1. 已执行的验证

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| workspace/all-targets/all-features，locked/offline | 434 passed | [日志](evidence/workspace-tests.log) |
| native-keyboard独立工作区 | 49 passed | [日志](evidence/native-tests.log) |
| performance工作区默认测试 | 2 passed、2 ignored | [日志](evidence/structure-tests.log) |
| 全目标全特性严格Clippy、fmt | 通过 | [Clippy](evidence/clippy.log)、[fmt](evidence/fmt.log) |
| core全特性及CLI lib严格rustdoc | 通过 | [日志](evidence/rustdoc.log) |
| verification manifest、PNG检查 | 21个example、38个PNG通过 | [manifest](evidence/target-manifest.log)、[PNG](evidence/visual-assets.log) |
| release Table/Document基准 | 显式执行的2项基准通过 | [Table](evidence/performance.json)、[Document](evidence/document-performance.json) |
| 同一提交真实GitHub CI | **失败** | [CI run](https://github.com/eddix/gpui-rhai/actions/runs/34201369168)、[失败摘要](evidence/github-ci-summary.txt) |
| 新增验收探针 | 复现C01/C02/C03/C05，同时确认多个旧缺陷修复 | [问题探针](evidence/probes.log)、[旧缺陷复验](evidence/second-pass-adapted.log)、[事务复验](evidence/positive-probes.log) |

环境：Rust1.94.0，Rhai1.26.0，GPUI0.2.2，aarch64 macOS26.6.2。使用全特性编译的公开API消费者作定向探针；针对新的合法API形态做了适配，没有把旧审计探针因API变化而编译失败当作产品bug。

未重新认证真实多窗口桌面输入、IME/AX、全部GUI启动smoke、干净package/install和受控120Hz完整帧。这里的“通过”只针对实际执行的检查。

## 2. 已确认问题

### C01 / P1：#42是A13修复引入的合法语法回归

位置：`crates/gpui-rhai/src/dependency.rs:81–130`，核心为`87, 105–112`；`ModuleDependencyGraph::from_source:65–71`。上游对照：`rhai-1.26.0/src/parser.rs:1433–1493`。

[issue #42](https://github.com/eddix/gpui-rhai/issues/42)中的根因判断基本正确，已独立复现：

```rhai
let x = `a${if true { "" } else { `b${1}` }}`;
```

plain Rhai1.26 compile成功；当前`extract_imports`返回`Open string is not terminated`，位置1:45。另测“内层模板后接真实import”、多段嵌套、插值内的真实import、模板内含假import文字、Map中的嵌套模板，均发现类似失败。不是所有嵌套形式都会失败，例如一个简短三层模板当前偶然能通过，因此只新增一个“三层嵌套可运行”的测试也不够。

`Engine::lex()`输出的是需要parser配合控制的token流，不能当成自行管理全部插值状态的完整解析器。Rhai原生parser递归解析插值block，在每一层block结束后显式将`is_within_text`设为true。当前扫描器用一个`Option<usize>`保存层级：

1. 外层`${...}`的brace depth正在累计。
2. 遇到内层`InterpolatedString`，无条件覆盖为`Some(0)`，外层上下文丢失。
3. 内层结束后切回文本；其最终`StringConstant`出现时没有弹出内层并恢复外层。
4. 后面的外层brace/backtick在错误模式下被解释，导致“未结束字符串”或后续中文字符异常。

因此，错误位置和中文报错是模式错位的下游症状，不能通过改中文文案或要求业务组件避免合法嵌套模板来解决。

我在[参考原型](issue42-reference.rs)中验证了两条路线，均通过9类正向场景、动态/越界import负向场景以及当前50个registry组件：

- **状态栈：** 每层保存brace depth和“等待下一段文本”的状态；`InterpolatedString`要区分同一个字符串的续段与表达式中的新内层字符串；最终文本token弹栈恢复外层。只把Option机械换成Vec仍可能处理错续段。
- **未优化AST：** 用真正parser解析并遍历`Stmt::Import`。必须关闭优化，以免折叠动态路径或丢掉`if false`内的依赖；解析限制应与正式compiler同源，尽量复用缓存AST避免额外全量编译。

[原型结果](evidence/issue42-reference.log)用于证明修复方向可行，不是已经落入生产代码的补丁。Changelog中的“import extraction uses actual AST”也应与最终实现一致：当前free function仍是lexer加手写状态机，只有部分入口使用AST校验。

验收要求：将上述场景写成自动回归测试，特别是“嵌套模板结束后还有代码/import”和“每层有多个`${}`”；验证导入集合，而不只验证不报错。通过真实FileScriptView/CLI check的prepare路径，不只调用裸Rhai compile。

### C02 / P1：A01只修好了普通延迟callback，增量render仍继承旧消费量

位置：`lifecycle.rs:293, 399`；`engine.rs:529–548, 1150–1234, 1361–1416, 1586–1590`。

新OperationTracker将相邻绝对计数之差累计。普通callback会先用`operation_base`初始化tracker，但`render_dirty`从0开始会话，随后直接用保存的context执行增量组件；第一次progress仍含父render的历史计数，因此被当成当前新工作。

同一个正式组件，初次render很轻；把自身`heavy`状态改为true后，只在该组件中循环100,000次：

| 初次root在创建组件前的循环次数 | 后续同一个增量render |
| --- | --- |
| 0 | 成功，记录300,029 operations |
| 250,000 | 失败，记录1,000,001，报operation budget exceeded |

两次增量工作完全相同。区别只在保存context前父级已消耗的额度。这说明“delayed callbacks fresh quota”测试通过，不能推导所有retained invocation都已修复。延迟虚拟项执行也使用保存的context，需要同类覆盖；其完整超限场景本轮未另行复现。

修复应让每个真实progress消费新操作，或在每次进入保存context时仅调整绝对计数基线；不要把每个子组件的累计预算也重置，否则会重新放过合计超限。验收应同时覆盖多dirty组件、virtual realization、嵌套component、effect/timer/callback，以及相同工作在轻/重父级下额度一致。

### C03 / P2：旧generation的带缓存订阅永远留在注册表

位置：`async_runtime.rs:1020–1100`。

`drain_up_to`发现generation不同后标记GenerationStale并continue，不清空pending；退出循环时只有pending为空才移除closed entry。旧generation的pending永远没有机会被消费，因此该entry永久保留。容量满时的阻塞producer也可能得不到所需的关闭唤醒。

实测：创建旧generation订阅、emit一条值，连续两次以新generation drain：

```text
stale_sub first=0 second=0 active=1 reason=Some(GenerationStale)
```

不投递旧值是正确的，但active必须最终归零，队列和producer必须完成明确关闭。此探针针对公共SubscriptionRegistry；普通effect替换会另行取消scope，可能掩盖该分支，并非每次高层reload都会泄漏。

建议把“生产者正常结束，缓冲值仍应交付”和“generation失效，缓冲值必须丢弃”分成不同终态；对后者无条件回收、丢弃旧数据并通知容量等待者。测试空/非空/满队列，以及`drain_up_to`容量不足的情况。

### C04 / P1：新的CI步骤先使用Linux原生依赖，后安装

位置：`.github/workflows/ci.yml:47–55`，`tests/performance/Cargo.toml:9–11`。

新增Performance structure tests使用GPUI test-support，在Linux会启用X11/Wayland相关依赖。安装原生库的步骤仍排在后面。此次提交的真实[CI run](https://github.com/eddix/gpui-rhai/actions/runs/34201369168)已经在这个步骤失败：

```text
unable to find library -lxcb
unable to find library -lxkbcommon
unable to find library -lxkbcommon-x11
```

这不是macOS测试能够证明或排除的问题，也不是泛化的网络故障。将库安装放到第一个依赖这些库的测试之前，重新跑同一完整workflow；本次run中后续原生测试、文档、package和release检查不能被算作CI已通过。

### C05 / P2：A15只阻止了native事件入口记录，语义事件仍泄露敏感值

位置：`context.rs:1711–1740, 1748–1766`。

`handle_node_event`已不再记录原始payload，sensitive state/store快照也正确脱敏；但`UiContext::emit`和`dispatch_action`仍将payload以`sensitive=false`写入TraceBuffer。正式组件可以把敏感state通过语义事件传递给父组件，此时诊断轨迹再次保留原值。

使用纯合成token复现：从标为sensitive的state读取值，再`ctx.emit("change", value)`，state快照是`<sensitive>`，最后一条trace却包含`AUDIT_ONLY_SYNTHETIC_TOKEN`且sensitive=false。没有使用任何真实用户凭据。

这不要求实现通用污点追踪。应统一事件/动作trace的默认策略：不保存原始payload，或由可信宿主显式选择允许记录的字段。验收需经过state→语义event/action→trace/Inspector整个链路，而不只分别测试state快照和TraceBuffer手动传入true的情形。

## 3. 额外的并发验收缺口

### C06 / P2 / 静态确认：emit_blocking取消存在丢失唤醒窗口

位置：`async_runtime.rs:671–695, 710–716, 1120–1126`。

关闭条件是独立atomic，发送者在拿queue mutex之前检查；close路径不拿同一个mutex就修改条件并notify。存在以下合法交错：

```text
producer：读到未关闭
closer：关闭 + notify_all
producer：拿到mutex，看到队列满，开始Condvar::wait
```

关闭通知早于wait，之后可能没有任何新通知；producer会无限等待。`close()`重复调用也不保证再次notify，因为首次关闭已成功。

建议所有改变等待条件的路径遵循同一个mutex/condvar协议，或使用不丢失通知的事件机制。检查producer close、scope cancel、generation stale、registry drop全部路径。现有容量等待测试验证了普通排空，不覆盖这个交错。本条有明确控制流证据，但本轮没有对实际私有queue注入调度屏障，不能宣称已完成确定性并发复现。

## 4. 原审计项的验收状态

“定向通过”是本轮运行的特定契约通过；“实现已检查”表示代码方向正确，但不足以宣告完整平台/故障矩阵完成。

| 原ID | 本轮判断 |
| --- | --- |
| A01 | **未通过**：C02；普通延迟callback和嵌套总量测试已通过 |
| A02 | 定向通过：const Map在compile阶段返回InvalidAssignmentTarget，未触发panic |
| A03 | 候选Engine隔离已实现；活动注册表不再先清空。尚需文件观察器失败→修复→继续交互的专门端到端矩阵 |
| A04 | 外层checkpoint已包含lifecycle；实现已检查。尚缺“虚拟投影成功后普通render失败”的独立提交指纹测试 |
| A05 | 独立delivery事务已实现；尚需A/失败B/C与暂停容量极限的专门端到端测试 |
| A06 | 定向通过：effect替换失败时旧树恢复，旧订阅active=1且closed=None；unit覆盖取消rollback |
| A07 | 已撤回，不参与验收 |
| A08 | COW与单实例mount直接更新已实现，5,000实例与大store恢复测试通过；现有结构测试没有直接断言clone/work复杂度，不能代替完整性能验收 |
| A09 | 默认受限resolver与测试通过 |
| A10 | worker pool、panic终态、合作取消和数量限制已实现；结果channel仍无界，接纳计数不含已取消仍运行的work，整体字节/运行资源预算仍不完整；另见C03/C06 |
| A11 | staging/项目锁/普通失败rollback已实现，CLI新增测试通过；未进行进程崩溃认证，不要求当前增加重型日志系统 |
| A12 | 目标依赖闭包、资产处理、manifest更新已实现；新增依赖测试通过 |
| A13 | **未通过**：C01 / issue #42 |
| A14 | 非有限数与递归值验证已加入，Rhai/serde定向拒绝通过；递归转换在完整validate前发生等大输入路径尚未完整压力认证 |
| A15 | **未通过完整链路**：native入口和sensitive快照已修，语义event/action仍绕过默认保护，见C05 |
| A16 | automation错误现在返回Err，原生action失败测试通过；当前仍依赖last_error桥接结果，未来可进一步类型化 |
| A17 | 本机manifest/PNG检查通过；**CI失败**，不是完整认证完成，见C04 |
| B01 | 定向通过：不同presentation域可使用相同NodeId，A几何/capture不被B裁剪，A宽度仍100；尚未重新认证真实双窗口输入 |
| B02 | 定向通过：同key重挂载后旧signal和callback拒绝，新实例count保持0；logical ref跟随保留 |
| B03 | 定向通过：候选generation不同，旧callback拒绝 |
| B04 | 定向通过：恢复view不清除显式暂停；完成timer改变curry后重新排程 |
| B05 | 定向通过：duplicate被拒绝后旧provider未缓存资产仍可加载 |
| B06 | 定向通过：非法serde默认值在store声明时拒绝，Host/Rhai非有限值验证已加 |
| B07 | 正常背压等待修复且测试通过；取消/失效终态还需C03/C06修复，不能把整个流生命周期标为完成 |

上述表不把没有补齐的测试都当成已知产品bug，也不把“代码已经写了”都当成完整验收。

## 5. 性能观察

本次release，3 warmup、10 samples：Table unchanged/reverse/selection/native resize p95分别为5.528/10.603/21.911/14.326 ms。Document resize dispatch p95为4.368 ms、settle p95为28.185 ms。

完整原始数据已附。未控制机器负载、硬件字段unknown，也没有相邻二进制AB；因此不据此宣称性能回退或显著提升。A01计量仍存在C02，operation字段还不能作为全部retained invocation的可信认证数据。显式执行release基准通过，和普通performance工作区里2个ignored测试不是同一件事。

## 6. 建议的下一次提交范围

1. 修CI安装顺序，让后续证据可信地跑完。
2. 修#42，并把9类语法/导入集合变形场景纳入测试；同步纠正AST/lexer文档描述。
3. 修所有保存context入口的预算会话，不只callback；轻/重父级×component/virtual/callback作为统一矩阵。
4. 修订阅正常关闭/失效丢弃/取消唤醒状态机；补确定性交错测试和旧generation非空队列测试。
5. 统一语义event/action的trace策略，补敏感数据完整路径测试。
6. 重跑本次检查及失败探针，再补A03–A05的高价值端到端故障用例。

无需撤销已经验证正确的身份、COW和事务方向，也无需引入历史兼容。验收材料的“已完成”应绑定这些新增测试和真实CI结果，避免再次让单个原始复现通过就代表一整类不变量成立。

## 7. 复跑入口

从仓库根执行，下面的命令只建立临时消费者，不改产品代码：

```sh
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/acceptance-0788e3c/probes.rs"
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/acceptance-0788e3c/second-pass-adapted.rs"
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/acceptance-0788e3c/positive-probes.rs"
GPUI_RHAI_ACCEPT_REPO="$PWD" bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/acceptance-0788e3c/issue42-reference.rs"
```

问题探针打印当前行为，不是通过即代表正确的回归测试。参考原型中的assert只证明列出的场景；正式修复仍应运行完整prepare/lifecycle路径。
