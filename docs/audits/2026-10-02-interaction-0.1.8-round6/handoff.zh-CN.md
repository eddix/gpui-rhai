# 本机实施 Agent 接手清单

2026-10-02。框架验收头：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`（PR #104）；已合main基础：`c088e96f`（PR #102）。公开dogfood已clone到 `/Users/eddix/Codes/github.com/eddix/disktree-rhai`，head `e33a61c4ebc8a6cd6de7ca55b36896452ee92685`。

本清单可以直接交给本机负责实现的engineer；不需要连接原作者机器。PR源、diff和issue正文已只读保存到本目录evidence，也可从GitHub的pull refs重新取得。此轮是评审与接手安排，没有代为合并、关闭或发表评论，也没有将“列入清单”视为已交付。

## 先完成框架阻断

以PR #104为集成基础，先处理[本轮报告](report.zh-CN.md)的三个P1、两个P2。窗口授权和多scope提交各用独立整改提交，保留可审核的边界；Table壳层修复也独立。不要先将#104合入再把已确认P1留作普通后续增强。

| 工作包 | 必须保持的约束 | 关键验收 |
| --- | --- | --- |
| queued native command授权 | 执行时必须仍持有发起mount的有效lease/epoch；Entity仍存活不等于仍被授权 | close入队→最后handle drop→新Host接管，旧close不得影响新窗口owner；显式dispose、正常owner仍有效的控制保留 |
| 多virtual target原子提交 | 各scope的candidate不能用旧全局snapshot覆盖彼此；父级存活集合必须包含最终子级candidate | 父子、并列集合的顺序/同批结果一致；新增组件自己的callback有效；已卸载状态不能复活 |
| raw row依赖贡献 | executable owner负责唤醒，item contribution负责存活；两者不能只保留一个身份 | 66条数据、始终1行的raw/formal对照；当前late collection可唤醒；共享名字保留正确引用 |
| Table状态外壳 | data/loading/empty共享横向viewport/clip；不用改窄demo掩盖溢出 | 三个early-return路径与data态同宽列声明；Action不画出边框；随后重拍基线 |

## 所有开放PR的处理

| PR | 本机负责人要做什么 | 合入/关闭条件 |
| --- | --- | --- |
| [#104](https://github.com/eddix/gpui-rhai/pull/104) | 保留完整mount adapter、RTL和COW方案，修上表及主报告阻断 | 远端CI已绿但独立反例仍失败；最终修复commit重新验收后合入 |
| [#100](https://github.com/eddix/gpui-rhai/pull/100) | 由#104的`new + mount_window`取代旧`new_with_policy`接法 | #104合入且对应目标验收后，关闭为已由#104覆盖；不要再并入旧构造器形成第二套权限入口 |
| [#103](https://github.com/eddix/gpui-rhai/pull/103) | 同样由显式mount adapter覆盖，保留需求/作者归属 | #104合入且稳定后关闭为superseded；不要公开另一条可覆盖native registry alias的路径 |
| [#101](https://github.com/eddix/gpui-rhai/pull/101) | 作者原始提交已经保留在#104；本轮RTL/边界阻断已验证关闭 | #104合入并完成Table状态壳层/视觉验收后关闭为已纳入，不再重复cherry-pick |
| [#96](https://github.com/eddix/gpui-rhai/pull/96) | 本机接管作者小改：保留Host operation limit，补真实超1M与所有构造入口传播测试、零值/extension优先级文档、fmt | 纳入0.1.8。将表达式深度作为独立Host配置处理，默认保持原策略；新dogfood实证其main在64可解析，不必全局128 |
| [#97](https://github.com/eddix/gpui-rhai/pull/97) | 统一大小写与named key/单标点grammar；补真实focus分发；修fmt | 纳入0.1.8。`key:Escape`不能接受后静默失效；`cmd-s/shift-/??`不能被当作普通单键成功注册 |
| [#99](https://github.com/eddix/gpui-rhai/pull/99) | 主开发补首帧System appearance初始化/失效；render读取+显式effect deps；轻量读取不clone整个ThemeVariant | 纳入0.1.8。初始System Light的Host与脚本一致，window/local作用域正确；修unused_mut后跑完整CI |
| [#14](https://github.com/eddix/gpui-rhai/pull/14) | 保留实验分支/数据，结束当前产品集成目标 | 推荐关闭，不纳入0.1.8；不是因CI账单红灯，而是当前预算/回调契约脱节、默认workspace fork污染、无当前release整应用收益 |

对#96的补充是本轮新增证据：实际 `ui/main.rhai` 和 `skins/minimal/main.rhai` 在function depth32/48失败、64/128通过，core算法文件在32已通过。应支持可信Host的合理配置，不能继续仅说“深度22不足以证明需求”。仍不应让Rhai脚本自行提高预算，也不据此把全局默认改成128。见[dogfood脚本报告](dogfood-script/report.zh-CN.md)。

合入时保留原作者提交和贡献说明，再追加本机整改提交。可以从当前远端pull refs取得代码，不需要原机器上未公开的工作区；不要把README提到的旧`disktree-integration`分支直接当作验收过的发行基线。

## 所有开放issue的归属

| Issue | 当前交付与owner | 完成判据 |
| --- | --- | --- |
| [#98](https://github.com/eddix/gpui-rhai/issues/98) 可变图片 | 0.1.8先由框架维护者补稳定ID前台refresh桥和版本化ID示例；per-asset revision/last-good/反序结果/共享窗口重绘是后续独立核心设计 | 文档路径可用不等于新API已完成；若保留增强诉求则保持issue open，并拆清文档已交付与API待办 |
| [#95](https://github.com/eddix/gpui-rhai/issues/95) mono/列字体 | 主开发统一theme role、Table两种数据源、auto-fit测量、Theme Studio round-trip；默认0.1.9，span另拆 | 换主题后code字体、指定列、离屏重建行与测量一致；不接受硬编码平台字体或仅改Rhai表格 |
| [#91](https://github.com/eddix/gpui-rhai/issues/91) invocation origin | 主开发独立设计可信source/view归属、同步继承/异步重置、native defer、Automation与真实输入区别；默认0.1.9 | origin不能由脚本payload伪造；不把“最近窗口有点击”当授权；不用Event phase代替UserInput |
| [#89](https://github.com/eddix/gpui-rhai/issues/89) Table context request | 主开发先定row/column/anchor/选择语义，dogfood用例驱动实施；0.1.9独立增强 | Array/Native一致，右键不误触row_click，键盘来源有正确anchor；复用现有Menu而非新建Overlay系统 |
| [#83](https://github.com/eddix/gpui-rhai/issues/83) 自定义grip | 主开发native绘制、命中与状态契约；0.1.9 | 溢出部分可命中、paint顺序正确、hover/drag/focus/disabled独立正确、theme token可换；不能仅增加node插槽 |

#93已经关闭且前轮独立验收支持其修复，本轮没有重新打开或重复列问题。

“本机接管”不等于把所有增强强行塞进0.1.8。每个仍open项都应有明确责任、版本和完成判据；延期项保持真实状态，避免为清空列表关闭尚未完成的需求。

## disktree-rhai本机实施顺序

1. **先固定可复现依赖**：当前Cargo.toml是相邻path，Cargo.lock不固定它的Git版本。记录实际框架集成SHA与features；README改成已验证版本，不让新贡献者猜工作分支。
2. **接线框架API**：采用修订后的operation/expr-depth配置、标点按键、resolved theme；窗口改为`ScriptViewHost::new + mount_window`，保留默认不授权的普通mount。
3. **先修应用P1旧扫描发布竞态**：每次scan拥有revision/activation，发布SharedTree前确认仍是当前请求；取消检查与写入处于同一发布规则。不能只依赖每100ms progress.send失败来发现取消。
4. **保留原生路径身份**：Node展示名可lossy，但后续open/reveal使用原始OsString/PathBuf；invalid crumbs应返回错误，不静默指向祖先。
5. **修主题、缓存与键盘**：保留system标志；palette随主题变更，geometry无需重算；先建by_key再缓存；统一cache key精度；去重k、补/，过滤文本不接收left/tab等named key。
6. **按整份值测资源预算**：1200项小fixture已证明2000 tiles cap不足以约束当前重复缓存结构；先去重复持有、拆读取域，再用真实RuntimeEngine测边界。
7. **默认用小内存树做验收**：cache hit/miss、同整数区间resize、Light/Dark往返、两次scan完成顺序相反、cancel、/ k Left Tab、嵌套/长名/非UTF8路径。通过后才由使用者选择受控目录作真实扫描。
8. **更新开发与完成声明**：`--dev`需对应Cargo feature；`rhai_check`的raw Engine/depth1000不是实际runtime验收。README/CHECKLIST/UPSTREAM_GAPS关于q与System主题的声明应一致。

保留Rhai可替换布局算法的产品目标；先修正确性、缓存和依赖边界，测过高频路径之后再决定是否需要更强的native数据适配。不要为了通过验收直接把所有布局策略搬回Rust。

详细证据：[脚本侧](dogfood-script/report.zh-CN.md)、[Host侧](dogfood-host/report.zh-CN.md)。两仓产品源码本轮都未修改，也没有运行用户目录扫描。
