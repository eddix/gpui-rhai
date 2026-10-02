# 0.1.8 最终 H 独立审查：be7a8eb8

日期：2026-10-03。最终测试/文档源码：`be7a8eb83f5c97b9932661d74e5c3b7c684be4ce`。冻结产品源码为 `53a62e3f2be05e5ae7a961936648195bc4fe9477`；已独立通过的核心检查点为 `99f77b1a`。RC draft PR #108 的main基础为 `c088e96f`。

## 两层结论

**代码与已验证契约：APPROVE（限定批准计划的源码/行为范围）。** 本次独立复验关闭 H-THEME-01，先前 C-TABLE-01 和 E-DOC-01 也均已关闭；当前未保留范围内 P0/P1/P2 产品/文档finding。

**整个0.1.8 RC：NOT READY，最终 H/整体目标尚未全部通过。** G要求的五张最终binary Table视觉基线、OS手工矩阵和物理120Hz仍真正pending。源码53的远端CI已SUCCESS（包含完整Linux X11/Wayland）；最终RC归档/test head的CI仍待完成，不能借用源码53的结果将其计SUCCESS。候选代码正确性结果不替代这些必需门槛，也不授权merge、GitHub closure、publish或tag。

没有从“suite绿色”单独推导批准：本审查完整读取计划/契约与相关源码，发现并保留了真实idle主题P1；最后独立实际Mac运行验证其修复和生命周期边界。所有旧失败、旧阶段报告和有遗留帧的观察材料保持不变。

## 范围与源码对应

按 A–H 固定范围检查D1/#105 Host quota/parser、D2/#106 keys/phases、D3/#107 resolved theme/native startup、#98 bounded foreground publication、Gallery/copied source/notes与共同生命周期边界。未扩入#83/#89/#91/#95/#14，disktree-rhai排除。

主 reviewer完整读取 code-review-expert、Rhai和相关GPUI context/entity/layout/action/async/event技能及所需参考，再按本机pinned官方source核查版本敏感控制流。保留 scoped state delta/final invocation manifest、ReadDependency owner/contribution、qualified mount/reservation和Style COW；没有要求重写已通过模型。

- `4785703f` 相对 `33d60119` 仅原正式Table断言迁移/实际mixed四列控制和公开文档，产品/CLI/registry字节相同；当时独立执行结果按该SHA保存。
- `b1aa8c7d` 只修CLI生成immutable manifest及checklist，本reviewer独立CLI33测试通过；没有把旧release CLI binary换SHA冒称新生成码验证。
- `53a62e3f` 实施appearance observer窄修，并加入actual native probe/test依赖。
- 本reviewer确认 `53a62e3f` → `be7a8eb8` 的core、CLI、registry、performance harness diff为空，后者仅docs与新增生命周期bin。因此产品等价沿用明确标注，不把旧源码失败日志重命名为新源码通过。

工具链/依赖：rustc/cargo1.95.0；提供的stable check为1.99；macOS27.0.1/26A434/arm64。官方 crates.io `gpui-pre/pre-platform =0.3.7`、`rhai =1.26.0`，无Rhai fork或gpui-component；Rhai default+internals/metadata/serde，volatile stored context仍隔离于crate-local适配器。native workspace单独使用test-support，没有进入SDK release依赖图。

## 发现与关闭

| ID | 历史结论 | 当前关闭依据 |
| --- | --- | --- |
| C-TABLE-01 / P1 | prepaint的refresh无效，viewport变化保持旧列宽 | 核心99的native request_animation_frame；未改原probe与正式TABLE-03验证实际列宽和有界zero-Rhai跟进 |
| E-DOC-01 / P2 | release checklist只到R4，缺最终维护索引 | b1新增final contract/ADR、R5–R7及POLICY/KEY/THEME/ASSET、G/manual/pending规则；独立源码核对闭环 |
| H-THEME-01 / P1 | 750ms真正空闲后Window Dark而Host/query Light | 53 common weak appearance subscription、authority map ingress和normal deferred work；be7独立实际Mac双向idle及生命周期矩阵通过 |

历史入口：[旧C失败报告](../checkpoint-c-review.zh-CN.md)、[C99独立通过](../checkpoint-c-99f77b1a-review.zh-CN.md)、[H初审](initial-33d60119.zh-CN.md)、[478代码/行为阶段](behavior-4785703f.zh-CN.md)、[主题P1](theme-appearance-ingress-p1.zh-CN.md)、[CLI/checklist b1闭环](cli-and-checklist-b1aa8c7d.zh-CN.md)。旧actual五stage对照仅证明startup/存在帧时的行为，不能继续代表idle switching通过。

## 主题修复的独立源码审查

锁定 GPUI0.3.7 `Window::appearance_changed` 只更新field/投递observer，不refresh；AppKit回调经foreground spawn延迟，避开App借用重入。`Context::observe_window_appearance` 本身使用weak Entity adapter；Subscription drop取消，不能detach或捕获strong entity造成长存活。

当前共同 `install_window_lifecycle_hooks` 同时覆盖primary、secondary与ordinary embedding。view持有Subscription，dispose显式take/drop；回调Disposed早退、更新权威map/去重，Active以weak defer进入正常poll并native notify，Suspended只保留新environment/dirty。没有在observer直接执行Rhai或重放init；token-only/native paint不再依赖有Rhai theme reader。既有源授权、target reservation及callback-generation入口没有复制第二套逻辑。

本机GPUI内部TestWindow simulator不是独立workspace可直接使用的公开入口；先前建议没有被伪造成验收API。维护的bin使用公开App appearance API和真实MacPlatform，不需要GPUI fork或fixture refresh。

## be7独立实际Mac运行

静默performance sampler完成并释放后，本reviewer顺序启动两个bin的三个进程；未并行GUI/构建干扰，未发送键盘/鼠标、未改OS偏好/Rift，最多每进程1或2个自有native窗口。检查JSON断言而不是只看exit0：

| 实际进程 | 结构化结果 | 独立证据 |
| --- | --- | --- |
| appearance initial Light | 7stage；两个750ms idle的timing/dirty为0，无pending/records变化；Light→Dark→Light的native/Host/query一致，init1/effect1→2→3，marker8保持 | [JSON](evidence/appearance-light-be7a8eb8.json) / [log](evidence/appearance-light-be7a8eb8.log) |
| appearance initial Dark | 同样7stage与idle判据，Dark→Light→Dark，primary/child均按实际mode更新 | [JSON](evidence/appearance-dark-be7a8eb8.json) / [log](evidence/appearance-dark-be7a8eb8.log) |
| appearance lifecycle | passed=true/error=null；同Window A/B独立System Runtime+fixed F、same-mode去重、pause/resume、retained disposed A→同ID A2、token-only T均通过 | [JSON](evidence/appearance-lifecycle-be7a8eb8.json) / [log](evidence/appearance-lifecycle-be7a8eb8.log) |

生命周期具体判据：

- 四段600ms idle所有view timing为0、records不变；同mode请求不重启effect。
- A暂停时appearance更新不执行Rhai（timing0/records不变）；resume立即得到新Light、init仍1，effect只按既定resume语义启动一次。
- 同ID重建后旧Handle仍Disposed，旧A records不增；A2与B接收新appearance，fixed F始终Dark/effect1。
- T只用native theme tokens，没有theme_variant render读者；appearance后Host snapshot/实际呈现几何更新，Rhai timing0。
- 三进程cleanup分别关闭2/2/1个自有窗；remaining=0/errors=[]，清除per-App appearance override。

这是**真实Mac应用的native appearance override**，不是TestPlatform注入、不是用户OS全局设置切换，也不是OS IME/VoiceOver或物理120Hz手工验收。token-only证明native重绘/Host snapshot与geometry，不冒称全窗口GPU色彩截图；G的真实图像审查仍须完成。

运行时复用的bin/source与构建材料strong hash一致，产品源等价检查为空diff：

- appearance source：`ea3a220a39829385bbc466e5784a14cec4552284c06c4bf04a44eaf51aa4268f`；binary：`54eabd3c890f71286d603d392a4506a3890eac29a98929eb837427d54d741ef4`。
- lifecycle source：`88241b41cee2b065ae3e4cf1ea1ec03df77a1d9f2f1a1d677c0b556409f3d169`；binary：`532c9ce1f941e73579e20dc965449ffe735c2c1838dbf3bc643aeb8dbe9f3b4e`。

## D与既有核心的独立覆盖

- D1：同一450k循环debug/release，default1000001拒绝、提高1350012成功/结果450000、降低500001拒绝；nested/sibling累计、retained/delayed新round、imports/failed reload、parser32/48拒绝64通过、builder/extension ordering和candidate独立存储/累计diagnostic均覆盖。没有把quota当frame时间或native-code抢占。
- D2：统一grammar/lowercase、Escape、六类标点、Capture/Target/Bubble trace、stop、disabled、fallback、key与key_char/modifier、Host Escape owner与committed input控制通过；不承诺NativeHandlerDescriptor协议或OS preedit已经升级/验收。
- D3：lightweight projection、render-only deps、body-only不重启/explicit deps重启、scope/window/siblings、Host/Rhai setter、missing theme/borrow、token override、首次真实native startup与secondary失败/同reservation重试覆盖。实际idle/生命周期新增证明关闭了之前未覆盖的ingress边界。
- #98：typed worker/foreground同步publisher、stable ID/opaque handle/new content ID、decoded BGRA red→blue、实际两个Host root刷新且peer state隔离通过。pending decode强一致、per-asset arbitration或GPU截图没有被冒称交付。
- 原核心：窗口origin/target/peer pump与drop/dispose/remount、Ref持续绑定/retarget/contribution/rollback、Table extent/RTL/四态/native override/零Rhai、正常keyboard/focus/termination与scoped virtual state/read贡献回归保留。
- 原Table helper迁移保留exact fixed120/flex2/percent30/fixed90 specimen及source descriptor/autofit同group，native Array/Native×LTR/RTL在300/500视口验证宽度与末列，不是删掉失败断言或改窄fixture。
- Gallery15实渲染含所有story/case、locale/viewport、所有bundled theme热切换和Host/Operations边界；public Table0.1.8/copied-source/SDK配套breaking约束明确。

## 执行归属与复用

本reviewer亲自执行的日志保持原精确时点，不重新贴新SHA：

| 亲自执行 | 结果/时点 |
| --- | --- |
| native D+核心129 + Gallery15 | 144 PASS，478（产品当时等价33，尚无appearance窄修） |
| root policy6/theme5/virtual5 debug | 16 PASS，478 |
| root policy release同fixture | 6 PASS，478，与debug重复语义 |
| CLI完整lib | 33 PASS，b1；CLI产品对be7未改 |
| 实际Mac3进程及structured checks | 3 PASS，be7；本次最终P1与生命周期证明 |

同任务其他独立/实施者提供的208 native完整suite/strict fmt/clippy（源53及等价be7）、root656/default473/MSRV/stable/rustdoc/package95/146/12、26个release smoke launch与artifact审计另作提供证据，不冒称为本reviewer全部重跑。26次为20个examples（含1次short performance）、Studio1、Gallery1、Table四态4，即25次long+1次short。源53/等价be7的quiet performance3/3记录旧PID90509约2.3–3%噪声，不声称noise-free、A/B或physical frame/120Hz认证。

本reviewer保留原default stack，未调整Rhai/布局预算、依赖、正式测试或产品；只新增audit/evidence文件，无commit/GitHub操作。旧failed/falsegreen证据不覆盖或删除。

## 仍未通过的整体交付门槛

1. **G五张最终binary视觉基线**：temporary floating尚未获确认，缺980×752原始/logical/DPI/binary SHA与逐张差异验证；旧69 PNG审计和native geometry不能替代。
2. **OS手工矩阵/physical120Hz**：真实input/preedit、VoiceOver、focus/clipboard/window/manual按变更适用项仍pending；本机物理60Hz。不能无依据N/A或把App override/模拟输入计通过。
3. **最终CI/完整verification收口**：源码53 CI已completed/SUCCESS，实际Linux X11/Wayland smoke均通过，见 [CI JSON](../evidence/ci-53a62e3f.json) / [完整日志](../evidence/ci-53a62e3f.log)。这是源码53的证据；最终RC归档/test head CI仍pending，须等待其实际结束，fail/cancel/pending分别记录，不制造“日志再提交→换SHA→重跑”的无限循环。package CLI的local-path/no-verify边界和未发布index消费者也必须如实保持，不计已完成crates.io clean-install。

结论因此是：**已核验代码契约通过，整个RC/H目标尚未就绪**。后续只补批准计划内的真实门槛；全部必需项完成后再做最终整体结论，不以当前代码审查自行批准发布。
