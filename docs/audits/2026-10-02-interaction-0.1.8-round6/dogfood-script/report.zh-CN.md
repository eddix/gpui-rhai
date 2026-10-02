# disktree-rhai：皮肤架构、真实依赖与本机接手清单

基线：disktree-rhai `e33a61c4ebc8a6cd6de7ca55b36896452ee92685`，gpui-rhai `83b19b1d`，2026-10-02。已读 README、Cargo.toml、DESIGN、UPSTREAM_GAPS、CHECKLIST、两套 skin 的主要脚本及 Host 接线。dogfood 仓库未找到 AGENTS.md，工作区干净；没有修改两仓产品。

本报告以源码分析为主，只运行了 **Rhai 1.26.0 的小型 parser/value/7 项 squarify fixture**。未启动 GPUI、未构建 disktree app、未扫描用户目录，也未复验 README 中的截图或 45 项测试声明。

## 结论

这个 dogfood 有价值：它已经把“可替换的脚本 UI/算法”与“Host 扫描、数据查询和外部动作”分开，两个 skin 的 tiling 确实不同。没有理由在接手前把所有计算搬回 Rust。

但“克隆后使用当前 gpui-rhai main 即可完成验收”还不成立。它依赖未合入的 #96/#97/#99 与旧窗口挂载方式；其脚本还存在独立的主题、缓存、键盘和累计数据预算问题。应把 **框架接入** 和 **dogfood 自身修复** 分开验收，避免再次把应用错误归咎于框架。

## 1. 真实依赖：不要把未合 PR 当成主线功能

| 依赖 | dogfood 的实际使用 | 当前处理 |
|---|---|---|
| #96 Host operation budget | `src/main.rs:287,303,312` 的三个入口都调用 `.operation_limit(10_000_000)` | 当前主线没有该 API。沿上轮 triage：作者小改后合入预算配置，默认保持 1M，补真实提额/传播测试 |
| 表达式深度 | 两份 main.rhai 确实超过 function depth 32 | 本轮有新增实证，见下一节；与 operation budget 分开决策，优先 Host 显式配置，不直接全局改为 128 |
| #97 标点 key handler | `with_keys` 注册 `? [ ] - = + .` 等 | 当前主线仍拒绝这些标点。采用完成原生按键规范化与验收的修订版，不能只以 parser 接受证明实体键盘可用 |
| #99 resolved theme query | `ui/main.rhai:266-268` 调用 `ctx.theme_variant()` | 当前主线没有该 API。沿上轮 triage 由主开发补初始系统外观、作用域、effect 依赖等闭环后合入 |
| 旧 #100 窗口策略方案 | `src/main.rs:386` 调用 `new_with_policy`，`:426` 使用普通 `mount` | 当前接法是 `ScriptViewHost::new` + **`PreparedScriptView::mount_window`**，普通 mount 不授予窗口命令权。详细 Host 权限接线由同轮 Host 审查负责 |

`Cargo.toml` 使用相邻目录 path dependency；Cargo.lock 不能固定这个目录的 Git 状态。README 要求 `disktree-integration` 工作分支，但它不是当前已验收 main 的等价物。接手时应记录实际集成 commit，避免切换用户正在使用的 gpui-rhai checkout 来满足隐藏依赖。

## 2. 表达式深度：新证据支持需求，但不支持原 PR 的全部论证

用锁定 Rhai 1.26.0 对**实际源文件**做 compile-only：

| 文件 | depth 32 | depth 48 | depth 64 | depth 128 |
|---|---|---|---|---|
| `ui/core.rhai` | 通过 | 通过 | 通过 | 通过 |
| `skins/minimal/core.rhai` | 通过 | 通过 | 通过 | 通过 |
| `ui/main.rhai` | 第 1448 行失败 | 第 1464 行失败 | 通过 | 通过 |
| `skins/minimal/main.rhai` | 第 1440 行失败 | 第 1456 行失败 | 通过 | 通过 |

第一处失败来自长 `on_key_value` fluent 链。仅在内存中将它分成每段 16 个调用后，32 又在后面的复杂 UI 表达式失败。因此这是实际 skin 表达式复杂度需求，不能继续只引用“算法函数深度约 22，尚未到 32”；但 squarify 本身在 32 下能解析，7 项有限样例也成功执行。

建议本机实现 Agent：给 Host 单独的表达式深度配置（默认仍 32，dogfood 显式选择已验证的 64），并贯穿 File/Embedded、reload candidate、secondary window。全局 128 不是这份代码运行的必要条件。适度拆分长 UI 函数仍有可读性价值，但不要把改写所有皮肤作为支持 Host 策略的先决条件。

这里的 fixture 只证明解析和小样例运行；没有重新验证原作者“2.7M operations / 78 ms”的全视口性能。10M 是整个 View 的执行上限，不是仅给 layout effect 的独立额度，也不保证一帧内完成。

## 3. 应用自身需要修复的确定问题

### D1：`--theme system` 在 Host 入口被改成了 dark

`src/main.rs:176-182` 对显式 flag 只保留 `light`，其余一律返回 `dark`。CLI parser 允许 `system`，脚本 `init` 也有 `set_theme_system("Disktree")` 分支，但正常启动链路永远给它 dark。此项由 Host 审查交叉确认，统一归此处，不应再作为 #99 的缺陷。

最小修复：显式保留 `system`，修正 `--help` 只写 dark/light 的文本；使用受控的 Light/Dark 窗口分别验证启动和运行时切换。

### D2：即使修复 Host system，已缓存的 tile 填色仍不会随主题切换

`build_paint` 在 `ui/main.rhai:250-262` 读取 light/dark 并缓存 rgba 整数；`paint_by_mode` 只按颜色模式索引。`render_ScanRoot:1364-1369` 的 layout effect deps 不含主题；`:1373` 虽重新读取 `is_light(ctx)`，传给 `stage` 的 `light` 参数实际未被使用（`:1906-1978`），mosaic 继续消费旧 paint cache。

这是源码确定的断链，不是“Rhai view 重绘就会自动更新所有缓存”。具体复验：同一份扫描结果、mode/viewport/metric 不变，切换系统外观；框架 chrome 应随 token 变化，同时检查 tile fill 的颜色值也改变。

最小修复：把 layout geometry cache 与 palette/paint cache 分开；在 render 中读取 resolved theme metadata，并把影响调色的 mode 或主题标识明确放入 paint effect/signature。主题变化只重建颜色，不重做目录查询或 squarify。不要靠 effect body 读取主题来假定 effect deps 会自动改变。

### D3：缓存写入时 `by_key` 仍为空，命中后丢失 tile 命中索引

`ui/main.rhai:402` 先 `kept_cache.push([cache_key, tiles, layout.by_key])`，`:407` 才填充 `layout.by_key = tiles_by_key(tiles)`。`empty_layout()` 初始给的是空 Map。缓存命中分支 `:344-359` 又原样恢复 `cached[2]`，导致 `tile_lookup` 查不到瓦片，hover/ring/selection 等失效。

独立 Rhai 值语义 fixture 已确认：先把空 Map push 进数组，再重新赋值原字段，并不会更新数组里保存的 Map。

现有 cache key 将 viewport width/height `to_int()`，而 effect deps 保留浮点尺寸。因此同一个整数区间内的两次尺寸（如 `800.1 → 800.2`）可以触发重算并命中该缓存。这也会保留旧尺寸几何。修复时先计算索引再写完整 cache entry，统一 viewport canonicalization；对“缓存命中与重新计算”验证 tile 数、索引、viewport 与 paint 一致性。

### D4：键盘表与处理器不一致

- `on_key` 接受 `/`，README/CHECKLIST 也承诺 `/` 打开过滤，但 `with_keys:1422-1478` **没有 `/` 绑定**。
- `k` 在 `:1431` 和 `:1458` 注册两次；GPUI-Rhai 将同一事件追加为多个 Target binding，普通 Stop 只在该阶段所有 binding 后停止，并不会代替 StopImmediate。当前 `on_key` 返回普通 unit，因此一次 `k` 会产生两次处理。
- `find_open` 分支 `:860-871` 把除 Enter/Escape/Backspace 外的所有注册名字传给 `find_push`。按 Left/Tab 等会把 `left` / `tab` 字符串追加进过滤文本，而不是移动光标或忽略。

最小修复：声明一份去重的 keymap；区分快捷动作与文本输入。过滤框优先使用正式 Input 的文本/IME事件，如果暂留键盘式过滤，至少只允许单个可输入字符，不把 named keys 当文本。实体键盘、Automation/native key fixture 应都跑，不能继续把全部差异归为 wtype 注入问题。

两套 main.rhai 复制了相同控制逻辑，修复和回归须同时覆盖。

### D5：`2000 tiles + 单槽缓存` 并不保证低于累计数组限额

layout 同时持有 `tiles` 和 `cache[0][1]` 的 tile 数据，每个 tile 还带 crumbs 数组；paint.base 和 label indices 继续增加累计元素。Rhai 对整棵值累计计数，不因数据来源相同而去重。

有界 fixture 只用了 **1200 个 depth-3 crumbs 的最简 tile**，再放入一槽 tile cache 和每 tile 一条 paint row（未加入 labels/insights/history），就得到 `Size of array/BLOB too large`。这比代码传入的 2000 tile allowance 更小。`core.rhai:333-423` 也只用 allowance 控制是否继续递归，没有在添加每个同级 tile 时严格封顶。

建议先去除当前布局与单槽 cache 的重复持有，按“整份数据”的真实数组/Map预算做承诺；必要时将独立数据放入分别读取的 store field，而不是同一大 Map 下换几个字段名。不能把“改 Array 为 Map”当成普适扩容原则，它只是转移到另一项预算。若要 Host 可配数据限额，必须连同 async preflight/UiValue domain/诊断使用同一配置；本轮不建议为 dogfood 直接放宽全局常量。

## 4. 保留架构优势，同时改进高频路径

应保留：Rust 持有扫描树，Rhai 通过 `disktree.tree.children` 读取每层受限窗口（96 + Others），而不是序列化整个文件树；布局只在 effect/event 中计算，zoom/pan 使用独立 view transform；第二套 skin 可换 tiling，验证了布局策略确实属于皮肤。

还需测量与优化的点：

- `on_pointer_move:1137` 在判断是否仍位于同一 tile **之前**读取整个 `layout` store。`tile_lookup` 查找本身是 O(1)，但获取整份树状值并不因此变成 O(1)。优先用主线已支持的 `get_app_store_path("disktree", "layout", ["by_key", key])` 与选中的 tile 路径，或拆分元数据/索引/paint field。
- `render_ScanRoot` 同时依赖 cursor、scan、layout；hover 改变会重新执行整个根视图。`mosaic_nodes` 每次仍运行 `paint_commands` 遍历全部 base rows、创建 Canvas commands，retained Rust 节点不更新不等于 Rhai 没有工作。
- 在保持布局算法可换的前提下，把稳定 base canvas、labels 和 cursor/rings 拆成独立 formal component/dependency 边界；再比较 pointer 同 tile、跨 tile、zoom/pan、theme palette rebuild 的操作数与耗时。若纯脚本结构优化后仍不足，再考虑复用 native transform/signals 或一个 Host-owned scene 数据适配器，而不是先迁走所有算法。
- `build_paint` 重复 `base = base + ...`、`sorted_label_candidates` 插入排序的成本属于一次性路径；先测 representative scene，不依据函数名或代码行数判断瓶颈。

## 5. 给本机实施 Agent 的执行顺序

1. **建立可复现接入基线**：保留当前主干；用独立集成工作区应用已经评审过的 #96/#97/#99 修订版；记录 SHA、features、工具链。不要把远端旧 integration branch 当已验收发行版。
2. **完成 Host adapter**：`new + mount_window`，正确 system flag，保留出错诊断和 close owner 权限。Window/scan cancel 等细节以本轮 Host 报告为准。
3. **修两个 skin 的 D2–D5**：主题/paint signature、完整 cache entry、keymap、真实累计数据预算。优先修正确性，再做高频拆分。
4. **添加小而确定的 fixture**：固定内存树，不先扫 HOME；同一参数 cache hit/miss 对照；同整数区间 resize；system Light/Dark 往返；`/ k left tab`；1200 多层 tile 数据；scan cancel/rescan/old result rejection。
5. **测实际 RuntimeEngine 链路**：parse、mount、effect、capability、pointer、reload 都使用真实 Host 配置；`examples/rhai_check.rs` 目前用 raw Engine + 深度 1000，只能当语法工具，不能作为运行时契约验收。
6. **整理维护结构与文档**：两套 skin 保留可导出的自包含产物，同时尽量共享控制器的单一作者源或明确复制同步检查。更新 CHECKLIST/FEATURES 对应测试 commit；当前 CHECKLIST 勾选 q 可用，而 UPSTREAM_GAPS G10 又记录 q 不可用，不能二者都作为验收证据。

上游贡献继续保留作者归属。应用自身 bug 的修复不需要伪装成 gpui-rhai patch，也无需等待所有新 API 才先写 fixture。

## 验证材料

- [小型 fixture](probes.rs)、[运行器](run-probes.py)、[最终输出](probes.log)：**1/1 测试通过**，包含 4 文件 × 4 深度解析、缓存 Map 值语义、1200 项累计预算、小型 Rhai 语法对照与 7 项 squarify。
- Rhai 1.26.0 中 computed `a[start..end]` 的实际错误是“Range 不能作为整数 index”。UPSTREAM_GAPS 把它解释为 `(a[start])..end` 的优先级问题没有被 fixture 支持，不应原样抄进框架文档；`_i` 被拒绝则已在本次小样例确认。
- 这是 parser/值语义和源码流分析，不是完整皮肤挂载或桌面交互已通过的声明。没有应用任何产品补丁，也没有把参考截图复制进 gpui-rhai。
