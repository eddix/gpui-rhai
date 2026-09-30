# 0.1.8 对抗审查：Tree、集合投影与虚拟化

审查基线：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`，相对 `v0.1.7`。
日期：2026-09-30。未修改产品代码、正式测试、issue 或 PR。

本报告使用真实 `RuntimeEngine` / `ScriptLifecycle` 的独立探针，以及逐字引用产品投影函数、集合模块的微基准。未运行原生窗口、VoiceOver 或完整发布门禁。所有计时在并行审查期间取得，只用于证明复杂度与量级，不作为独占机器上的发布性能基线。

## CT-1 · P1：Tree 的合法默认状态无法挂载，折叠当前项的祖先也会使渲染失败

定位：`registry/components/tree.rhai:108`、`:120-123`；`crates/gpui-rhai/src/engine.rs:3253-3263`。

`active_key` 是可选且允许 null 的 prop，但是 Tree 无条件给 `virtual_collection` 写入 `reveal_key`，无 active 时值为 `()`。底层的 optional string 规则要求“字段不存在”或者“字符串”，不接受存在的 null。结果最小合法 Tree，以及空数据 Tree，都会以 `virtual collection reveal_key must be a string` 失败。文件顶部自己的最小示例也省略了 active_key。

另一条正常交互路径同样失败：`parent` 展开且 active 为 `child`，调用方接受 Tree 自己发出的 `expanded_change`，把 parent 折叠。Tree 的 toggle 只修改 expanded 提议，不调整 active；新的可见行不再含 child，仍传 `reveal_key: "child"`，触发 `virtual collection reveal key child is not present in its data`。因此调用方必须猜测并补齐一个组件未说明的跨 prop 约束，否则无法正常完成折叠。

证据：[tree-lifecycle.log](tree-lifecycle.log)，`no active`、`empty`、`collapsed active child` 三组均经实际 ScriptLifecycle.start 复现。

修复方向：仅在存在合法可见 reveal 目标时传入 reveal_key；定义折叠、删除、禁用后的有效 active 策略，并使视觉标记、键盘动作、可见性请求都使用同一结果。保持 controlled 契约，必要时发出 active_change 提议，不要在内部悄悄提交外部状态。

验收：省略 active、显式 null、空数据均可挂载；选中子节点后折叠父节点不报错；祖先多层折叠、数据删除、active 外部变更后，键盘焦点目标与显示一致。

## CT-2 · P1：折叠深链绕过深度限制，并在前台执行二次复杂度验证

定位：`crates/gpui-rhai/src/collection_projection.rs:91-115`、`:153`。

每个 source 都新建 ancestry set 并从自己走到根，长链累计检查 `n(n+1)/2` 次。唯一 depth 上限在 flatten 阶段，只有可见的展开分支才会经过它。因此只显示一行的完全折叠 10,000 节点链，被当成合法输入接受，却执行约 5,000 万次祖先访问。该算法在注册给 Rhai 的同步 Rust 函数中执行，Rhai operation budget 无法在这些内部循环中打断它。

逐字摘取产品函数、`rustc -O` 的独立微基准：

| 折叠链节点数 | 耗时 | 输出 |
|---:|---:|---|
| 100 | 0.525 ms | Ok(1) |
| 1,000 | 62.5 ms | Ok(1) |
| 3,000 | 591 ms | Ok(1) |
| 6,000 | 2.66 s | Ok(1) |
| 10,000 | 8.77 s | Ok(1) |

实际 RuntimeEngine API 的 debug 构建也复现：1,000 / 3,000 / 6,000 / 10,000 节点分别约 0.490 / 5.76 / 30.0 / 76.8 秒。debug 数字不用于推算生产性能；它确认公开脚本入口确实执行了该路径。深度 257 的同一输入在折叠时返回 Ok(1)，全部展开才报 depth limit。

证据：[projection-bench.py](projection-bench.py)、[projection-bench.log](projection-bench.log)、[probe.log](probe.log)。摘取源文件 SHA-256：`b5e3ac5091855e858d8fa060d82d85cf74e5c2694b9f3a9caacfdc382b718024`。脚本自动从当前产品文件提取完整原函数，只补充所需 UiValue 枚举，不重写算法。

修复方向：在全图结构验证阶段完成 cycle/depth 校验，使用三色遍历或已验证祖先/深度缓存，把每条边的访问次数约束为常数级；展开状态只影响最后的可见投影。当前 rows/expanded 的数量限制继续保留。不要通过降低性能探针规模或扩大脚本预算掩盖此问题。

验收：全图 depth 合法性与 expanded 无关；同等输入规模下正常森林、深链、孤立循环均有有界访问次数；合法 10k 森林以及超深链拒绝路径不会在前台阻塞秒级。优先添加确定性的访问计数/复杂度断言，再用受控 release 基准记录实际时间。

## CT-3 · P2：ArrowLeft 可进入 disabled parent，之后动作却使用另一个节点

定位：`registry/components/tree.rhai:103-115`、`:131-132`。

Tree 的普通导航先过滤 disabled 行，但 ArrowLeft 直接把 `active_row.parent` 作为 active_change 提议。父节点 disabled、子节点 enabled 是合法 schema 输入。当前 active=child 时，Left 的 payload 是 parent；调用方接受 parent 后，`row_index(eligible, parent)` 找不到，静默回退到 0，也就是 child。

于是父节点接收 active 样式，Enter 的 select payload 却仍为 child，Left 还会继续提议同一个 disabled parent。问题不只是 disabled 跳过策略，而是同一 controlled active 值被显示和动作逻辑解释成了两个对象。

证据：[tree-lifecycle.log](tree-lifecycle.log) 的 `disabled parent` 与 `after accepting disabled parent`。两次真实生命周期挂载打印保留的键盘 payload，后一组明确 active=parent 而 Enter payload.key=child。

修复方向：所有方向键共享有效 active 与 disabled 策略；父节点方向导航不得绕过该策略。禁止用找不到时的 0 索引掩盖无效 active。验收 disabled 祖先、动态禁用当前项、无有效项三种情形，所有方向与 Enter 必须与可见 active 一致。

## 虚拟列表 drag pin 的复杂度证据（合并到交互域问题，不重复计数）

定位：`crates/gpui-rhai/src/virtual_list_element.rs:105-107`。

只要全域存在 application drag，每个虚拟列表每次 prepaint 都线性扫描其整个 data 查找同名 source_id，来源不属于这个列表时也会完整扫描。虚拟化只限制 realized 行数，不能限制这条新热路径。

[pin_scan.rs](probe/src/bin/pin_scan.rs) 原样引入产品 `native_collection.rs` / `collection_projection.rs`，调用与 prepaint 相同的 `VirtualCollectionData::key`。本地包启用 opt-level=3，测试 10 次预热后 50 次，拖拽键不存在：

| 每个列表总行数 | 每帧查键次数 | 中位数 | P95 |
|---:|---:|---:|---:|
| 1,000 | 1,000 | 3.08 µs | 3.13 µs |
| 10,000 | 10,000 | 30.3 µs | 42.0 µs |
| 100,000 | 100,000 | 319 µs | 339 µs |
| 1,000,000 | 1,000,000 | 3.20 ms | 3.54 ms |

这是查键自身成本，不是完整 frame time。多列表成本相加；由于 source_id 缺少 collection identity，同键碰撞另由交互审查负责确认。修复应先以 collection identity 路由，仅向来源集合发 pin；通过 projection revision 的 key→index 索引，或在 drag 开始/投影变更时解析缓存位置，避免 pointer 每帧重复扫全量。索引应属于具体投影，不能错误复用排序/过滤前的 source index。

证据：[pin-scan.log](pin-scan.log)。

## Issue #87：建议本轮解决 Host 动态注册，脚本写入另作完整设计

[Issue #87](https://github.com/eddix/gpui-rhai/issues/87) 仍 OPEN。其核心事实在 HEAD 成立：`ScriptViewHandle` 只有 `replace_native_collection`（`app.rs:918`）；注册入口仅在 RuntimeState 的 registry；`NativeCollectionRegistry::replace` 对未知名字返回 UnknownCollection（`native_collection.rs:1666-1669`）。这是缺失能力，不是本轮新引入的回归。

建议 0.1.8 纳入显式 Host live-register（P2，面向现有接入需求），保持 replace 的语义清晰。需同时覆盖重名/非法名、disposed view、挂起后恢复、作用域归属和清理、发布数据与触发首次依赖渲染的一致性。新增动态名字后不能因没有历史 readers 而永远不刷新；由 Host 明确在同一受支持更新流程发布引用状态，或定义可追踪的缺失读取策略。

不建议只为关闭 issue，仓促增加不受配额控制的脚本 `set_native_collection`。脚本写入必须先定义集合所有权、事务失败回滚、热重载/卸载清理，以及总行数/字节/集合数量限制。Array→Rust 的一次性转换也不能绕过现有脚本输入限制。可先用 Host live-register 满足用户明确给出的二选一诉求。

关联文档改善可同轮完成：Table 明确 Array 面向小输入，错误提示引导 NativeCollection。Issue 中性能数字来自报告者环境，不能直接作为本项目的保证或阈值。

## PR #88：建议本轮合入，修复确实对准了性能根因

[PR #88](https://github.com/eddix/gpui-rhai/pull/88) 仍 OPEN，head `5ec2758e197b5ec6e7fdb868650c6847860a8071`。HEAD `node.rs:1256-1269` 仍在查找 collection 时把整份 realized map clone 给经过的子节点，成本随前置布局节点数 × realized 内容放大。

补丁把 map 放在 Option 中，沿遍历路径传 mutable reference，仅命中目标时 take；原有匹配首个目标和返回语义保持。新增堆分配地址回归测试能确定性检测字符串被深拷贝，适合此问题。静态审查未发现该补丁引入正确性问题。

建议作为 0.1.8 的 P1 性能修复纳入，最终集成后运行当前基线的原生、package 和 release 门禁。可补 Overlay / Layer 分支与未命中目标的测试。此次子审查没有 checkout、合并或执行 PR 分支。

主审补充核验：该精确 PR head 的远端 portable job 已成功，原生测试、package、release build、Linux X11/Wayland smoke 与 artifact audit 步骤均为 success，见 [CI 明细](../evidence/pr-88-ci-job.json)。作者的“本机未运行”不等于这些检查完全没有执行；这份远端结果仍不能替代后续合入所有整改后的最终验证。

这条修复不等于已解决快速 fling 的空白行，也不会消除整个 view transaction 的成本。按 PR 的自述，空白行改进试验已经撤销；应保留独立问题和后续 profile，不能把未验证的增大 overdraw 或提高 pump 频率一并塞入该补丁。

## 重现

在仓库根目录运行：

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-09-30-interaction-0.1.8/collections-tree/probe/Cargo.toml --offline --bin audit-018-collections-tree
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path docs/audits/2026-09-30-interaction-0.1.8/collections-tree/probe/Cargo.toml --offline --bin pin_scan
python3 docs/audits/2026-09-30-interaction-0.1.8/collections-tree/projection-bench.py
```

完整公开 RuntimeEngine 的 debug 深链计时可在第一个命令后加 `-- projection`，但已记录充分证据，不必反复占用前台运行数十秒。探针使用表征式输出：日志中的预期缺陷会显示 ERROR，而不是把这些失败当成产品测试通过。

补充验收建议：Tree 当前平铺 treeitem 没有把 depth 传入已有 `accessibility_level`。层级、同组位置和有效 active 的原生 AX 表达应增加专门检查；本次没有辅助技术实测，因此不把这一条升级为已确认的 VoiceOver 缺陷。
