# H-THEME-01 · P1：空闲 native appearance 没有进入运行时失效源

日期：2026-10-03。缺陷产品源码仍为 `33d601198690bbef912e8f7737121e981f2830b1`；失败实机 evidence 标记 `4785703fb43c7c75516ff2ab960de7341a58d983`，两者 core/registry 源码相同。`b1aa8c7d` 只修 CLI/checklist，不能关闭这个主题缺陷。

**结论：REQUEST_CHANGES。D3/H 当前不能通过，整个候选仍未就绪。** 本记录追加而不改写前面的初审、行为绿色日志或原始实际App对照。

## 实测与先前证据的边界

本 reviewer 独立完整检查新增 probe 的源码、锁定官方 GPUI 控制流和提供的失败 JSON/log；本次实际Mac进程由同任务 probe agent 执行，不冒称为本reviewer独立启动。

[失败 JSON](../evidence/d3-actual-macos-idle-light.json) / [原始日志](../evidence/d3-actual-macos-idle-light.log)：

- NSApplication 本进程初始 Light；primary/child init均为1，effects均为1，marker已由正常callback变为8。
- appearance切换前等待真实timer750ms，源码要求primary timing为空、shared dirty=0、无pending virtual request，init/effect records不变。
- 然后仅改变本App native appearance，不派发假输入、不fixture refresh，也不改OS/Rift偏好。
- 实际 `Window::appearance()` 已为 **Dark**，HostTheme仍为 **Light**，脚本状态仍为 `primary:1:light:8:Light:light` / `effect:1`。
- sample timeout返回错误；cleanup对2个本进程窗口提出关闭、errors为空、remaining=0并清除app override。

失败 JSON 的 `samples` 为null，因为exercise返回Err后没有保留之前的Vec；“750ms空闲guard已通过”是结合实际Dark失败及源码执行顺序作出的推断，不是声称JSON直接含有该idle sample。修后材料应显式保存成功idle阶段，以避免证据含糊。

此前 Light→Dark→Light 和 Dark→Light→Dark 两个5-stage对照，在未清空初次/child/effect遗留帧时为exit0。它们可以保留为startup/有帧状态下的观察，**不能证明空闲后的原生外观切换会自行失效**；正式TestPlatform Light/固定Dark与手动refresh的fixture也不覆盖本缺陷。之前独立144 native/16 root/6 release的结果仍属实，但不能因此豁免这次范围内P1。

## 锁定官方源码根因

`gpui-pre 0.3.7/src/window.rs:2742–2748` 的 `appearance_changed` 只更新Window field并遍历appearance observers，不调用refresh。`:1872–1886` 的AppKit appearance callback通过foreground spawn延迟运行，避免App已借用时重入；不是在platform callback里直接执行任意Entity/Rhai工作。

当前产品只在mount/startup建立appearance，以及 `ScriptHostView::render_snapshot` 内调用 `UiRuntimeState::update_window_appearance`。当Window本身没有新frame时，runtime map仍为Light，也没有发布theme reader dirty；轮询无法凭空推断native field已变。最后的表象是Host/native theme query与实际Windowappearance分离。

现成 `Context::observe_window_appearance`（`app/context.rs:457–472`）使用weak Entity adapter，回调参数为 `(&mut T, &mut Window, &mut Context<T>)`。`Subscription`的drop取消观察；不应detach或在registry callback捕获strong entity制造长生命周期。

## 窄修复边界与生命周期审查点

共同 `install_window_lifecycle_hooks` 是primary/secondary及ordinary embedding都经过的合适入口。持有appearance Subscription，使用权威native Window值更新该View所属runtime map，并发布正常失效即可；不需GPUI fork、强制重放init、每帧Rhai或新的权限系统。

必须同时满足：

1. **Active**：原生事件真正成为ingress。更新map/去重，唤醒正常poll处理Rhai dirty；不要在observer里直接执行Rhai或重新进入同Entity。没有Rhai theme reader、仅native theme_color token的View也需要native paint/Host snapshot刷新，不能只依赖has_window_dirty。
2. **Suspended**：仍更新权威environment并保留dirty，但不执行Rhai、不启动新effect、不消费掉dirty。resume要在effect/render前看到新mode，不能先用旧theme启动effect、随后再二次重启修补。
3. **Disposed/drop/native close/remount**：回调先拒绝Disposed；dispose显式take/drop此subscription。旧Handle仍保留Entity或同ID replacement时，旧observer不应重新建立旧window map、发布dirty或获得命令资格。
4. **多View**：同physical Window中的独立Runtime/View都应拥有各自观察路径；不能仅由command-owner或首个View代为发布。window/local fixed overrides、兄弟scope及peer state隔离保持。
5. **空闲和重复事件**：相同SystemAppearance不重复notify/执行；native-only重绘与Rhai工作分离，既修token-only paint，也不制造持续帧需求。

## 最小正式/实机验证建议

- 用公开 TestWindow.simulate_appearance_change 交付真实延迟native事件。先park/clear timings，再正常advance/poll，无manual refresh；验证实际Window、Host snapshot、script mode及explicit effect1→2。same-mode再次事件无新timing/activation。
- token-only/no theme getter正控：native Host snapshot与paint能够更新，Rhai operations保持0。
- 两ordinary Views/独立Runtime同Window，包含System与fixed/local override正控；都由真实appearance事件正确更新且状态/权限隔离。
- suspend→appearance→normal poll：不执行新Rhai/effect；resume使用最新mode、init不重放。随后dispose保留旧Handle再同IDremount，后续appearance只作用于newView。
- 重跑既有窗口authority/ownership、Ref/Table/theme/effect矩阵，保留旧source/Pending reservation/贡献模型。
- 实际Mac再跑两个方向，切换前明确记录750ms idle阶段；无fixture refresh/假输入，核对native mode、Host/query/effect、state保留和cleanup。仍标为App-only native override，不等同OS global setting或manual IME/VoiceOver/120Hz。

原报告/日志不可覆写为绿色；修后采用新精确source SHA和新的evidence文件。G五张截图、OS手工和物理120Hz缺口仍另列pending，CLI/checklist收口也不授权publish。
