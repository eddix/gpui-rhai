# PR #14 处置评审：adaptive Grain JIT 实验

**建议：不合入当前主线；关闭这次集成 PR，保留实验分支、固定提交和研究材料。** 等真实 release 工作负载证明需要脚本计算加速，再以独立实验和新的后端契约评审重新进入。当前没有理由把它作为 0.1.8 的补充功能。

这是处置建议，本轮没有关闭、评论或合并 PR，没有编译或运行 JIT。

## 范围和证据

- PR：[feat: add opt-in adaptive Grain JIT experiment #14](https://github.com/eddix/gpui-rhai/pull/14)，head `6427de4733e61fcc92b9e1d05ff98c177baf899d`。
- 固定主线基线：`f6e936a509d9aaf75f2e28836bedd439fe83887e`。评审过程中其他会话修改了工作树，本报告依赖固定 Git 提交，不把那些未提交修改归入本次分析。
- merge base 为 `a38e301ccd95779d34dd528dac19fd85567a8dee`，主线此后有 51 个提交。
- 已读取 PR diff、最终 head 代码、两份历史性能/接入报告、官方本地 Rhai 1.26.0 源码，以及当前 engine/invocation/backend 的固定版本。
- 未独立复跑 PR 宣称的 384 项测试、131 项 fork 测试或性能数据；也没有对汇编生成器做完整内存安全证明。以下明确区分代码事实、已有实验数据和迁移时必须处理的风险。
- CI 红灯由仓库账单/额度限制导致 job 未启动，不能用作代码编译失败证据。CI 状态由总评审单独核实。

## 1. 实验原理仍有价值，但集成方案与当前运行时已经脱节

当前官方 Rhai `=1.26.0` 已含可选 Grain 字节码 VM。主线的 `grain-backend` 只用于特征测试，生产执行仍为 AST；官方 Grain 本身不是这里的 AArch64 JIT。

PR 同样标称 Rhai 1.26.0，但实际 workspace 解析到 `eddix/rhai@989efd986d05ce9799ab2d0472ef7f697d022316` 的 `jit` 分支。`CallbackBindings`、`FunctionAccelerator`、`NativeFrame`、`on_progress_batch` 等是该 fork 的额外接口，不能因为 crate 版本字符串相同，就把它当成当前已锁官方源码的另一个 feature。

PR 的有效研究成果包括：

- 标量 lane 将整数、浮点、bool 等控制流变为本机代码。
- managed lane 通过 Rust helper 保持 Dynamic 生命周期，没有直接读取其私有布局。
- 只在调用入口选择后端；执行发生副作用后不从头重试。
- executable owner 保持代码和嵌入站点存活，覆盖重入/eviction 的测试。
- 自适应机制把“可以编译”与“值得执行”分开，并记录收益不足及小函数开销。

这些适合保留为实验资产。但当前主线对比基线已重写大量执行与生命周期代码：engine `+2027/-228`、context `+1684/-87`、lifecycle `+1662/-163`、invocation `+98/-2`。行数不是质量结论；真正的问题是下面这些契约已改变，而旧测试不能为新契约背书。

### 1.1 旧 operation observer 无法直接承接当前执行预算

当前 `engine.rs:553-585` 的 OperationTracker 聚合嵌套 evaluator、多个 dirty component 和 virtual item 的执行量；`:642-648` 的 progress callback 执行总预算终止；`:2266` 把 Rhai 自身 max_operations 设为 0，由上述累计机制负责。

PR [adapter 的 configure_engine](https://github.com/eddix/gpui-rhai/blob/6427de4733e61fcc92b9e1d05ff98c177baf899d/crates/rhai-jit-experiment/src/gpui.rs#L31) 安装 batch progress callback，只 `observer.record(operations)` 然后返回 None；[observer 接口](https://github.com/eddix/gpui-rhai/blob/6427de4733e61fcc92b9e1d05ff98c177baf899d/crates/gpui-rhai/src/backend.rs#L48) 只存储一个绝对计数，没有预算消费/拒绝语义。它所属旧基线还使用 `set_max_operations(1_000_000)`。

所以不能仅解决编译冲突后保留该 adapter：它会替换当前终止 callback，而旧 observer 又没有当前累计模型。**这是把旧实现接到当前主线时的明确安全契约阻断，并非声称 PR 当年的分支已被实测绕过预算。** 如果未来接入，必须让所有 AST/Grain/native lane 消费同一执行会话预算，正确处理批量 tick、嵌套返回、多个虚拟行、重入与延迟回调，计时观察不能代替终止机制。

### 1.2 回调环境、深度及生命周期不能按旧 adapter 覆盖回来

当前 `invocation.rs:21-59` 分为 `capture_retained` 与 `capture_entry_retained`，显式重置延迟调用的 stack level，并处理入口模块的 library/source 环境。PR 的旧 `capture` 主要依赖 fork 在 accelerator 存在时给 stored context 新 invocation 边界。

当前的 callback provenance、incarnation、虚拟行作用域、原子事务和 hot reload 已继续演进。PR 的 imported render smoke 仍使用 runtime API `[1,2)` 的老组件，只证明当年接口场景；这不是要求保留历史兼容，而是说明验收集不覆盖现有 runtime API 2。将来需要针对当前真实组件的 imported helper、callback chain、effects、虚拟化、失效回调、候选 reload 失败做 AST/Grain/JIT 对照。

### 1.3 错误和回退的比较基准必须是生产 AST

当前主线的官方 Grain 特征测试还明确记录 divide-by-zero 的错误调用栈格式差异。PR 有大量 native/Grain 的值、操作计数及错误一致性测试，也有一部分 AST 对照，这些是有用证据；但 native 与 Grain 一致，不等于与当前生产 AST 的结构化诊断、错误位置和事务行为一致。

没有在本轮检查完整 fork 实现或运行新对照，不能断言所有这些行为错误；同样不能把旧 parity 数量当作它们已经通过。

## 2. published package 隔离已修，默认 workspace 隔离仍然不足

PR 最终版本已经处理了旧 review 的重要问题：公开 core feature 不再直接命名 fork-only 类型，具体 JIT 移到 `publish=false` 的实验 crate，新增官方依赖图下的 all-features package 验证。不应继续用早期“发布包存在 11 个缺失 API”的问题否定修订后的 head。

但是 [根 Cargo.toml:48-49](https://github.com/eddix/gpui-rhai/blob/6427de4733e61fcc92b9e1d05ff98c177baf899d/Cargo.toml#L48) 的 `[patch.crates-io]` 无条件替换 Rhai。即使没有启用 experimental-backend，仓库普通开发/测试/CLI release 的依赖来源仍是 fork；`default-members` 排除实验 crate 不会撤销根 patch。

因此应该准确区分：

| 范围 | PR 最终状态 |
|---|---|
| 发布到 crates.io 的 core package | 保持官方 Rhai 类型/依赖边界；旧问题已修 |
| workspace 默认构建及 CLI | 根 patch 仍解析到 fork，依赖来源并非 opt-in |
| 真正的 Grain/JIT 执行 | 需 feature、adapter 和显式 attach，仍为 opt-in |

若保留后续实验，放在独立 workspace/独立仓库，或仅由实验 Host 的根 manifest 持有 patch，主线普通构建维持官方锁定。当前没有必要先把尚未验证的通用后端注入 API 变成框架长期维护面。

## 3. 原始数据不支持“自适应后至少不影响 UI 性能”

保留了 [原始历史 JSON](historical-benchmark.json) 与 [按 AST 基准重新计算的结果](benchmark-vs-ast.json)。数据来自 2026-09-03、`50fbda2` 上未提交的实验实现，不是 PR 最终 head 或当前主线的新实测。每个样本是 100 次调用的平均值，40 个交错样本；其 P50/P95 不是 UI 单帧分布。

同一历史 adaptive run：

| 工作负载 | AST P50 | Grain P50 | Adaptive P50 | Adaptive 相对 AST |
|---|---:|---:|---:|---:|
| numeric | 79.397µs | 60.345µs | 3.609µs | 约 22× 加速 |
| objects | 7.341µs | 7.495µs | 6.879µs | 耗时降低 6.3% |
| native_mix / host-heavy | 12.034µs | 15.250µs | 15.455µs | **耗时增加 28.4%** |

PR 摘要的 17× 数值收益成立于其数字计算样本；managed 数据处理也确有小幅收益。但是 host-heavy 被判为 unprofitable 后，策略返回的是 **Grain**，不是生产 AST。其自身 1.34% 的 adaptive-over-Grain 开销，叠加 Grain 相对 AST 的差距，仍为约 28.4% 的历史微基准退化。

这不是“当前应用将必然慢 28.4%”的预测，而是说明该收益保护机制选择的参照对象不足以支持整应用迁移。真实应用常见 UI 节点构建、集合投影、属性克隆、调度、布局和绘制成本不等于纯算术成本；尤其本轮正在暴露的是虚拟化生命周期与失效模型问题，JIT 不解决这些正确性问题。

历史真实 Host 只有 debug 模式 28/28 功能断言的维护者报告，没有独立提供完整日志和 release frame 分布；其真正编译运行的 `entry_row` 随后被降级为 unprofitable。该文档本身已经明确承认没有建立整应用收益，应保留这种严谨表述。

## 4. 安全与交付边界的判断

实验使用 executable memory 与 unsafe AAPCS64 调用边界，并针对 helper panic、重入期间代码存活、禁止副作用后重试做了具体设计。这些措施值得保留。**不能因为出现 unsafe 就认定有内存漏洞；本轮也没有确认新的生成器内存安全反例。**

它仍只提供 AArch64 native emission；其他平台不会自动获得相同收益。没有一般性 OSR/deopt、完整捕获闭包覆盖或完整 Rhai 配置组合认证。缓存的 16MiB executable-buffer 上限也不等于进程 RSS 上限，源码库 lowering、元数据和调用中保留的代码需要另行测量。

这些限制符合独立实验的定位，但没有证据证明应扩大当前产品支持面。不存在“用户必须在自己 Host 里做不了，因此框架必须现在提供 JIT”的需求。

## 5. 建议的具体处置

1. **关闭本次 PR 的主线合入目标，保留分支 `chenxin.pro/rhai-jit-experiment` 和固定 SHA。** 理由是集成基线和当前验证目标已改变、整应用收益尚未成立，不是 CI 账单问题或旧版本不兼容。
2. 保留 scalar/managed ABI、入口选择且不重放副作用、pinned executable ownership、收益采样和微基准数据。保留在研究目录或独立实验即可，不必把实验 crate 和根 Rhai patch 合入当前 workspace。
3. 当前继续修复真实 profiler / dogfooding 揭示的瓶颈和组件模型，维持 AST 的统一预算、错误与生命周期契约。不要为了消化历史 PR 而先增加后端功能。
4. 重新启动的触发条件：当前 release 应用能证明脚本计算占显著成本，而且 Rust 原生热路径/增量化不足以解决；随后在相同当前源码、数据、硬件下，对官方 AST、Grain、adaptive 做包含冷启动、交互 P95、整帧、内存和 long-session 的对照。
5. 满足该需求后另开小型设计评审：无根 workspace patch 污染；统一 budget controller 而不是只读 observer；完整 runtime API 2、回调与失败事务等价性；最后才决定是否保留公开 backend injection API。

可用于处置说明的一段话：

> 该 PR 已完成一轮有价值的 JIT 可行性实验，但尚未建立当前 UI 应用的 release 收益，且集成基线早于现行预算、回调和虚拟化模型。本次不合入产品主线，保留实验分支和研究数据；后续由明确的脚本计算瓶颈驱动独立方案重新评审。

本轮仅提出上述建议，没有执行任何外部处置。
