# 最终 H 独立代码与行为复验：4785703f

日期：2026-10-03。实际执行 HEAD：`4785703fb43c7c75516ff2ab960de7341a58d983`。产品源码/CLI/registry 与 `33d601198690bbef912e8f7737121e981f2830b1` 字节一致；增量仅测试与公开文档。最后 native appearance probe 仍在收口，最终候选 SHA 待主实施者确认。

## 阶段结论

在已审源码与独立行为范围内，**未确认新的 P0/P1 产品缺陷**。独立 native D+核心固定矩阵 **129/129**、实际 Gallery **15/15**、root policy/theme/virtual **16/16** 以及 release policy重复 **6/6** 均通过。

这不是最终 H 整体批准：发现一个明确的 E 文档交付缺口（下文 P2），并且真实Mac外观证据、G五张截图、OS手工/物理120Hz及最终head的完整验证清单尚未全部完成。**当前整个候选仍为“未就绪”。**

## 已覆盖的代码与行为

### D1 · Host execution policy

- 配额 Cell 和累计 OperationTracker 从新 candidate engine 建立，继承有效值但不共享可变quota。File/Embedded prepare、secondary 重新构造遵守 builder→trusted extension 顺序；reload candidate不改变active engine。
- 默认1M、parser64/32、call/data limits分别保持；零规范为1。源码直接核对 Rhai1.26.0 `api/limits.rs` 的零unlimited惯例，Host层显式没有采纳它。
- debug和release对完全同一450k循环实测：默认在 **1,000,001** 拒绝；提高到2M后 **1,350,012** 操作、输出450000；降低500k后在 **500,001** 拒绝。debug记录约319ms、release约25ms，这只是该有限脚本的执行时间，不是帧或OS延迟门槛。
- 非默认quota nested/sibling累计、dirty siblings第二span小于配额但round超配额、retained/delayed新回合、imports及失败reload保留last-good通过；失败diagnostic仍报告原quota与累计round consumption。
- parser32/48拒绝、64通过，同一fixture覆盖 RuntimeEngine/File/Embedded，debug/release均通过；没有调整全局Array/Map/String或引入脚本with_budget。

### D2 · 单键 grammar 与真实 phase

- Rhai四入口共用lowercase规则；Escape归一化，`? / [ ] - =`的实际focus-path派发通过。非ASCII、空白、chord、多标点与长度限制在执行时拒绝。
- Capture外到内、Target/Bubble内到外有真实trace `PCTBQ` 对照；capture stop阻止后续phase。disabled、Enter/Space显式handler替代click fallback、无handler fallback、raw key/modifier/key_char边界均通过。
- Host active gesture Escape先取消owner，再允许普通script capture，未使原input/IME/focus/正常终止回归。
- Rust NativeHandlerDescriptor仍使用独立snake-case协议，文档不承诺其支持key:?。OS preedit不由模拟committed text代替。

### D3 · resolved theme 与 native startup

- projection只复制identity/motion所需字段，复用scope-aware resolver；metadata不先clone整个ThemeVariant。render读取tracking，body-only effect不自动重启，explicit deps正常重启且同值不重复。
- root/native5+6覆盖missing manager/borrow、Dark headless fallback、nearest scope/window/siblings、Host/Rhai setter、token override、首次TestPlatform Light、init一次与secondary失败/同reservation重试。
- 新native create次序仍保留同一qualified mount/native binding：appearance先于init/effect，失败child与其排队leaked Open不留下native窗或reservation；窗口B1全部正式组合同时通过。
- 提供的实际Mac probe源码已独立完整读取：`gpui_platform::application()`的官方source调用`current_platform(false)`；App appearance setter实际设置NSApplication appearance而非TestPlatform enum。该probe是**本进程native应用外观**对照，不修改OS-wide偏好，不等于完成OS global appearance/manual matrix。尚未把构建/Clippy当作实际运行通过。

### #98 · typed background → foreground publication

- worker仅携带allowlisted AssetId、revision、prepared数据；TaskWork的Send边界不含AssetRegistry/Rc/Rhai/GPUI。publisher在foreground处理schema、allowlist/stale guard，并refresh已注册namespace。
- 稳定logical ID/opaque handle保持，GPUI content ID改变；CPU decoder BGRA red→blue全部像素验证，同时有真实mounted image geometry。shared registry显式Host刷新两个root，peer app state不共享。
- 没有把root redraw+decoded content提升为全窗口GPU截图或强per-asset/pending decode一致性；公开文档明确这些局限，保留scope freeze。

### 原核心及Gallery交叉

- 当前native窗口authority/origin/deliveries/ownership **19**、Ref **8**、Table boundary/extent/frame **20**、keyboard **63**均独立通过。预算、主题startup和key routing没有破坏这些共同入口。
- 原Table结构断言迁移不是跳过旧fixture：保留原fixed120/flex2/percent30/fixed90源descriptor，加入root plan唯一宽度消费者、header/body autofit同signal group，并在Array/Native×LTR/RTL实际300/500viewport测四列宽度及隐藏末列可达。新native原样specimen通过。
- Gallery15包含每个story/case实渲染、viewport/locale、每个bundled theme热切换、Host embedding及异步Operations交叉；不是prepare-only。wide/四态公共Table API与ctx.theme_variant metadata未发现集成违例。

## P2 · E-DOC-01：release checklist尚未链接最终契约索引/最新回归

源码位置：`docs/release-checklist.md` 的0.1.8段落（当前第81行起）至第116行结束。清单仍只逐轮追加到R4，没有 `docs/runtime-contract-tests.md`、R5–R7核心收口或新增POLICY/KEY/THEME/ASSET正式矩阵入口。

批准计划E明确要求“补上统一契约索引及R5–R7必要回归”。已有维护索引与产品测试本身是正确方向，但阅读release checklist的人无法从该清单辨别最后一轮窗口来源、Table真实extent/frame-demand、长期Ref、scope/contribution与新集成包的验收要求。

建议只做文档收口：链接最终维护contract index，列出它对R5–R7及D1/D2/D3/#98必要回归的映射，明确历史probe不按原退出码机械判正确、不替代实际矩阵；不要改写原审计报告或复制新大型workspace。该缺口不是新增产品能力或泛化技术债。

## 独立执行记录

所有执行均清除RUST_MIN_STACK；root/native缓存复用，没有新大型target/profile，无产品/正式测试/原审计改动。debug/release policy是重叠语义，不相加当不同契约。

| 独立执行集合 | 结果 | 日志 |
| --- | --- | --- |
| native D1/D2/D3/ASSET +窗口/Ref/Table/keyboard | 129 PASS | [native-d-and-core-4785703f.log](evidence/native-d-and-core-4785703f.log) |
| 实际native Gallery | 15 PASS | [gallery-4785703f.log](evidence/gallery-4785703f.log) |
| root policy6/theme5/virtual5，all-features debug | 16 PASS | [root-d-and-virtual-debug-4785703f.log](evidence/root-d-and-virtual-debug-4785703f.log) |
| root policy同fixture release | 6 PASS | [policy-release-4785703f.log](evidence/policy-release-4785703f.log) |

实施者655/root、207/native、strict Clippy、stable1.99、package/CLI smoke、release性能等材料另外作为提供证据审阅，不列为本reviewer独立执行结果。旧core99检查点仍按其精确SHA独立报告保留，没有重命名为当前RC全量通过。

## 仍然阻止整体就绪的门槛

- 最后probe/dependency/doc收口后的精确head确认、对应完整verification清单与CI/Linux结果；验证product diff为空后可明确沿用本轮同源码行为，不为纯日志/docs反复全量build。
- 实际Mac应用native appearance运行及恢复材料；本轮只是审其source/提供build证据，不称实际运行通过。
- G要求的最终binary五张新980×752 Table视觉基线、原始/logical/DPI与binary SHA、逐张审查和浮动恢复；没有获准浮动时不能伪造或复用旧PNG。
- OS input/preedit、VoiceOver、真实focus/clipboard/window手工矩阵及物理120Hz。实际机器为60Hz；release结构或性能sample不能替代尚未完成的人工门槛。
- 完整候选F清单的source/command/pass/fail/pending映射与既定CLI no-verify边界。未批准merge/publish/tag/closures。

修好E-DOC-01及其他证据收口后，最终H再统一给出结论。没有要求#83/#89/#91/#95/#14新能力，也没有触碰disktree-rhai。
