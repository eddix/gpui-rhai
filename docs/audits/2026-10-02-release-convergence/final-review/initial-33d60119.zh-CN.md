# 最终 H 独立审查：33d60119 初始记录

日期：2026-10-02。当前产品 SHA：`33d601198690bbef912e8f7737121e981f2830b1`，RC 分支 `codex/release-candidate-0.1.8`。比较入口为独立通过的核心 `99f77b1a`；完整 RC 的 main 基础为 `c088e96f`。

状态：**COMMENT / 独立复验进行中；整个候选尚未就绪。** 这不是最终批准，也不把实施者的207项原生或release性能绿色结果改称独立验收。

## 已做的源码审查

主 reviewer 完整读取本计划 A–H、契约矩阵，并按 code-review-expert、Rhai、GPUI context/entity/layout/action/async 技能核对所依赖的锁定官方源码。

- D1：检查 RuntimeEngine operation-limit Cell、OperationTracker、candidate-engine 构造、File/Embedded/secondary extension ordering、reload path、parser depth 与 ExecutionTiming/Diagnostic round consumption。候选 Cell/Tracker 使用新引擎独立存储；默认64/32显式设置，零规范为1；诊断保存 timing 起点的 quota，而不是后来Host的值。未发现已确认的范围内代码缺陷。还需要精确冻结候选上的独立 debug/release 工作量与回滚复跑。
- D2：检查统一 key grammar 和 lowercasing、Capture/Target/Bubble 分支、disabled/fallback、stop 路由及 active gesture Escape interceptor。GPUI0.3.7 macOS source 区分 Keystroke.key/key_char，并对shift punctuation生成实际key；TestPlatform重点覆盖实际focus path，但不替代OS预编辑/真实输入法验收。`NativeHandlerDescriptor` 的独立snake-case协议没有扩展，公开文档明确此边界。
- D3：检查 lightweight family/name/mode projection、render-only tracking、scope/window resolver、body-only与explicit effect deps、primary mount和secondary startup-error路径。appearance在init/effect前建立，不重放init；secondary仍保留旧核心的mount/reservation/native binding连续性。TestPlatform首次Light、fixed Dark和headless fallback是不同证据；真实macOS SystemLight/SystemDark仍需独立实机材料。
- #98：worker只捕获thread-safe数据/类型，不携带AssetRegistry/Rhai/GPUI；foreground synchronous handler有Asset allowlist、revision guard和受控SVG。稳定ID、opaque handle、changed GPUI content ID及decoded BGRA pixels有正式断言；shared-registry fixture显式Host刷新两个真实root并检查peer state隔离。文档承认namespace refresh较粗、pending decode不强一致、不同registry不自动共享；不得扩大为新per-asset API或全窗口GPU像素证明。
- Gallery/notes：wide/LTR/RTL、四态和controlled resize使用公共Table API；theme metadata来自ctx.theme_variant。Table source升至0.1.8，release note明确SDK/copied-source配套、Operation const/ExecutionTiming/WindowCommand及key whitespace的breaking变化。Runtime API2没有冒称保证旧SDK能执行新Table native builder。

本轮当前未修改或运行产品测试；只做源码和提供日志的read-only检查。之后的独立实测将在新的evidence目录保存。

## 已识别的证据缺口（不是泛化范围扩张）

1. **源码尚未最终冻结**：正式Table结构测试正在迁移旧per-cell flex/signal断言，并增加实际mixed四列几何控制；公共文档也有WIP。当前`final-workspace.log`仍为48通过/2失败，CI旧head亦失败。不能跳过失败、放宽期望后直接称整个RC通过；须核查语义等价的新断言和对应实际几何，在新精确head重跑。
2. **真实 System appearance**：独立实际Mac probe正在准备。TestPlatform Light、固定Dark、core注入enum均不能冒称实际SystemDark startup/switch。必须核对实际native appearance、init/effect次数、presented metadata与Host snapshot。
3. **工作包G五张新基线**：暂无本轮最终binary的5张实际980×752截图、原始/logical尺寸、DPI、binary SHA及逐张差异审查。浮动权限尚未确认；不能用旧PNG审计或native几何代替视觉通过。
4. **OS手工与120Hz**：现有机器物理display60Hz。release性能groups3绿可以按实际配置记录，不是物理120Hz手工门槛。OS preedit/VoiceOver/实际focus与clipboard/window manual matrix尚pending，不能依据模拟输入计通过。
5. **完整F/外部状态**：最终source的rootworkspace、targeted regressions、package/CLI smoke、rustdoc、licenses/artifacts、最终CI/Linux smoke及verification manifest仍需逐项对应最终head。已存在的提供日志是实施证据，不是本reviewer独立结果；CLI no-verify/package边界也不得改称干净index解包验证。

这些都来自批准计划的既定门槛。没有扩入#83/#89/#91/#95/#14，disktree-rhai排除。当前没有已确认的新P1/P2源码finding，但这不能推导为最终H或发布就绪。

## 后续执行边界

等待最终test/doc/probe收口后的精确冻结SHA，再独立执行D1/D2/D3/ASSET与核心共享边界的正式矩阵；新测试将核查实际几何/排程、实际窗口/focus，而非仅enqueue成功或AST形状。

原审计报告、旧probe和原始日志保持不变。不会修改产品、正式测试、依赖，不commit，不执行GitHub review/comment、merge、publish、tag或closure。最终报告必须把代码结果与强制平台/视觉pending分别列出；任一必需门槛未完成时仍为“候选未就绪”。
