# PR #111 第二轮审查：修复复验与新增 issue 集成

日期：2026-10-08。最终审查 HEAD：**`48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8`**（起始候选为 `a1fd3702`）；本轮对照上一审查 HEAD `c28849a139e34a2ed25fc8e27b63633383dcd48d`。对象：[PR #111](https://github.com/eddix/gpui-rhai/pull/111)。

## 结论

**仍需修改，当前不批准作为 0.2.0 发布候选。确认 2 个 P1、2 个 P2。** 上轮原始 R1–R5 探针保持原字节，全部通过；R1 的普通样式入口已修好，但新 `.signal_style` 入口出现同类布局问题，记为 S3。

本轮还核对了 #83、#89、#91、#95、#109、#110、#112–#118 的需求、实现和现有回归。主要新增缺口集中在 **#83 的命中边界与样式解析、#91 的后续 action 来源、#117 的 Region 默认裁剪**。不需要推翻已完成的设计系统，应补齐这几个契约。

起始候选 `a1fd3702` 独立执行：workspace **679/679**、native **269/269**、`dev-reload` 的 `token_reload` **3/3**、上轮独立探针 **5/5** 全过。审查过程中追加的 `48cdd0b4` 已另行核对，结果见下文。新增 **5 项探针失败，对应 4 个 finding**（S1 分别复现 SplitPane、Resizable），在两份提交上均复现。

## 新发现

### S1 / P1 · #83：被遮挡的装饰手柄仍能发起拖动

位置：[handle_state.rs:45](https://github.com/eddix/gpui-rhai/blob/48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8/crates/gpui-rhai/src/handle_state.rs#L45)，由 SplitResize 和 Resizable 的 pointer listener 共用。

`over_handle` 把 GPUI `hitbox.is_hovered` 与装饰节点的矩形包含关系做 OR。装饰分支不检查实际前景遮挡，因此新的 overhang 命中能力绕过了原来的命中约束。

两个原生对照都在分隔线原生 hitbox 之外、装饰 grip 的伸出区域开始拖动。覆盖物是后绘制的、不带事件回调的 `.occlude()` 不透明兄弟节点：

| 场景 | SplitPane 最终比例 | Resizable 最终宽度 |
| --- | ---: | ---: |
| 无 grip，按伸出位置 | 0.5 | 200 |
| 有 grip，无覆盖 | 0.6714285714 | 240 |
| 有 grip，被前景覆盖 | **0.6714285714** | **240** |

最后一行应该保持 0.5 / 200。覆盖物下方的控件仍接走输入并修改受控值，这不是 grip 画得大一点的视觉差异。

**修复要求：**装饰区域必须通过实际可见、经过 clip 且未被前景遮挡的命中区域参加命中；不能仅用 ElementRef 的矩形兜底。保留合法 overhang 行为、原生 cursor、focus 与零 Rhai 的拖动预览。

**验收：**保留上述两组件的正反对照，补裁剪父容器及上层 Overlay 代表场景。不要通过禁止 overhang 或让覆盖物额外写 `stop()` 回调来规避框架问题。

探针：`audit_grip_must_not_start_through_an_occluding_sibling`、`audit_resizable_grip_must_not_start_through_cover`。

### S2 / P1 · #91：异步完成后的语义 action 丢失调用来源

位置：[lifecycle.rs:836](https://github.com/eddix/gpui-rhai/blob/48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8/crates/gpui-rhai/src/lifecycle.rs#L836)，以及 [app.rs:6036](https://github.com/eddix/gpui-rhai/blob/48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8/crates/gpui-rhai/src/app.rs#L6036)。

真实点击启动一个 task。task completion 先直接调用 capability，再通过 `ctx.dispatch_action` 调用相同 capability。实测：

| 入口 | Host 看到的 origin |
| --- | --- |
| click 内直接调用 | `UserInput { click }` |
| click 内派发的同步 action（正对照） | `UserInput { click }` |
| task completion 内直接调用 | `TaskCompletion { started_by: UserInput { click } }` |
| 同一个 completion 派发的 action | **`Lifecycle`** |

`invoke_async_delivery` 的来源 guard 只包住 callback；它返回时已经恢复旧 origin，Host 随后才执行 `invoke_pending_effects` 中的 action/event。因此同一交付引发的调用会因是否经过 action 而得到不同来源。

这破坏 #91 的用途：Host 无法可靠归因，按现有 `is_user_input()` 示例门控时，合法任务后续调用会被误拒。本轮证明的是归因错误与合法调用被拒风险，**没有据此声称已复现权限提升**；也没有否定本版明确选择的 async `started_by` 语义。

**修复要求：**来源必须跟随产生的后续工作。统一覆盖一条 delivery 引发的语义分发，或者让排队的 action/event 自带 enqueue origin，并在调用时恢复；不能让 drain 时恰好存在的全局 origin 决定它的来源。同时核对 Timer、Subscription、component emit 与 effect 内排队动作。

**验收：**本例中的 direct call 与 action 保持同一 TaskCompletion 来源；加 Timer/Subscription 的语义转发，以及失败后的 origin 恢复和不同来源连续交付。不要把异步调用一律改成 UserInput。

探针：`audit_task_followup_action_keeps_the_delivery_origin`。

### S3 / P2 · R1 与 #83 交叉：signal_style 的 margin 在 stretch 判断之后才生效

位置：[renderer.rs:2302](https://github.com/eddix/gpui-rhai/blob/48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8/crates/gpui-rhai/src/renderer.rs#L2302)。

R1 的修复会检查普通样式中的 margin。但新的 `apply_signal_state_style` 在 `definite_stretch` **之后**执行，所以 signal 选中的 margin 绕过了该判断。

同一个 300px column 中，两个子节点最后都拥有左右各 20px margin：

| margin 来源 | x | width |
| --- | ---: | ---: |
| `with_style(style().margin_x(px(20)))` | 20 | 260 |
| `.signal_style(state, #{ padded: style().margin_x(px(20)) })` | 20 | **300** |

后者仍越出父级。signal 的初始值就是 padded，反例不依赖 hover 时机或异步写入失败。该 API 接受通用 Style，并未限定只能改变颜色。

**修复要求：**先得到全部有效样式层的结果，再决定能否采用 stretch 快速路径；确保父布局提示也使用同一最终样式。原生 signal style 的布局变化不能走一条绕过现有正确性保护的路径。

**验收：**本例、signal 在无 margin/有 margin 间切换，以及会影响布局的 position/align 代表组合；普通 R1 回归继续通过。

探针：`audit_signal_style_margins_keep_stretch_equivalent`。

### S4 / P2 · #117：Region 的默认非滚动 body 没有按规格裁剪

位置：[region.rhai:125](https://github.com/eddix/gpui-rhai/blob/48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8/registry/layouts/region.rhai#L125)。D51、composition guide 和 release notes 均写明 body 默认填充并裁剪，`scroll: true` 才启用滚动。

当前只在 scroll=true 时设置 overflow；false 分支没有 clip。独立原生输入对照：Region 为 300×200，`fill:false`，body 为 500px 高的可点击节点。在 Region 底部再往下 100px 点击：

| 配置 | body 收到的 click 次数 |
| --- | ---: |
| `scroll: true` 正对照 | 0 |
| `scroll: false` | **1** |

反例实际验证了越出 Region 的子内容仍可接收输入，不是把未裁剪的 AX 几何当成绘制结果。长内容可以侵入相邻区域，默认路径与公开规则不符。

**修复要求：**明确为非滚动 body 应用裁剪，滚动 body 继续保持自己的 viewport；保留 header/toolbar/footer 的独立布局。测试不能只证明默认情况下“不滚动”，还应证明“不越界绘制和命中”。

**验收：**上例与真实绘制/clip 对照；footer 固定；内置 Table/VirtualList 自己滚动仍正常；Host 管理的 Overlay 保持正确的独立呈现规则。

探针：`audit_region_default_clips_overflowing_body_input`。

## 上轮 R1–R5 的复验

原探针取自材料提交 `859a06b103ce2534639d338cac6e023a9138ad84`，本轮保存为 [previous-probes.rs](previous-probes.rs)，逐字节核对一致，SHA256：`7deaa4ad5454d27494dee379da3b5bf6fa4cedaf4f90338b4e31899e00237238`。

| 原项 | 当前结果 |
| --- | --- |
| R1 普通 margin stretch | 原探针通过，正式 `stretch_layout` 的 LTR/RTL 组合通过。新 signal-style 入口仍需 S3 修复，不能把这一结果扩大为全部样式路径通过 |
| R2 token reload | 原探针通过；明确开启 `dev-reload` 的 `token_reload` 3/3，通过删除必需 token、坏表达式、纠正与组件/token 同批修改等场景 |
| R3 Native Table 键盘 | 原探针通过；正式 Array/Native plain/filtered/paged/sorted/collapsed 组合通过，Enter 与已实现虚拟行焦点修正一起验收 |
| R4 qualified Capture | 原探针通过；正式 key phases 对照通过 |
| R5 不完整 by_env | 原探针通过；完整性校验与多轴正反对照进入本轮通过的 core suite |

不会覆盖或改写上轮的失败记录。

## 13 个 issue 的处置建议

以下“通过”指本轮所述功能与检查范围，不是已经在 GitHub 关闭，也不是对所有未来组合的保证。

| Issue | 本轮检查与判断 |
| --- | --- |
| #83 装饰手柄 | **暂不完成：S1、S3。** 合法 overhang/hover 路径已通过，但前景遮挡和 signal-style 布局仍有缺口 |
| #89 Table context request | 范围内通过。Array/Native pointer 选择顺序、current-row Shift+F10 与 row anchor 的正式测试通过；selection-none pointer 请求控制也在测试中 |
| #91 capability origin | **暂不完成：S2。** direct click、Automation、Effect、Timer、TaskCompletion 的基础测试通过，语义转发仍丢来源 |
| #95 typography | 按本轮修订验收范围通过：token base/Host 决定 code 字体，Table 列与 span typography/background 测试通过；不以旧要求强迫 palette 改字阶。Linux span fallback 仍按已披露的未验证边界记录 |
| #109 overlay/layer/shared identity | 范围内通过。declaring-instance owner、歧义查询、重复 Select dismiss、实例内 submenu parent、shared-layout scope 的正式回归通过；本轮未确认新碰撞 |
| #110 virtual item paths | 范围内通过。`Item[<key>]` 下的 keyless component 在新旧 realized 行共存时不冲突，正式虚拟/事务旧套件也通过。路径变化与一次性 state reset 是已说明的 breaking change |
| #112 audit 漏遍历 | 范围内通过。ErrorBoundary、Layer、realized virtual rows 进入 Walker；`audit_regressions` 正反控制通过 |
| #113 sticky header | 范围内通过。push-out 时的 viewport clip 及重叠处 column-header 点击测试通过，未用未裁剪 AX bounds 误判 |
| #114 audit 误报 | 范围内通过。pane 不再代表首个 control、marker 内部间距与 heading/view-switcher 规则通过既有对照 |
| #115 Toolbar/DataView | 范围内通过。fill 的首/中/末位置、长 placeholder、窄宽度 wrap、DataView fill/size/inset 回归通过 |
| #116 shortcut legends | 范围内通过。caller shortcut 走统一格式化与 Kbd，正式测试通过 |
| #117 migration/Region | 文档的 macOS titlebar、Variable<Length>、token base 与 SplitPane slot 填充说明已补；scroll=true 正常。**默认裁剪还需 S4** |
| #118 容器焦点与 group header | 范围内通过。Table group header 标签字阶、容器/Overlay panel focus-frame 与只靠自有重绘请求的测试通过；AppShell 边界保持已约定范围 |

因此当前不宜把 #83、#91、#117 宣称完整解决；其他 10 项可在修复后的最终候选合入时按实际提交处理。当前 PR 尚未合入，issues 保持 open 是正常状态，本轮不执行关闭操作。

## 审查中追加的 48cdd0b4

该提交只改 PanZoom/Rotatable 的 proposal 呈现、两个回归及实机记录：保留 proposed transform 到 Host 接受/拒绝，并为 draw-time signal 同步安排后续帧。原 S1–S4 所涉及的产品文件与 a1fd3702 字节一致。

本轮已检查这段增量，并在最新检出重跑完整 workspace、native 及统一独立运行器。新增的接受/拒绝两组原生呈现回归包含在 `control_visuals` 29/29 中；未新增此范围内的 finding。实施方追加的 CGEvent 结果仍按提供证据记录，不冒称本轮亲自重做。

## 验证记录与发布标准

| 验证 | 归属与结果 |
| --- | --- |
| 完整 workspace all-target/all-feature | **a1fd3702 与 48cdd0b4 均独立 679/679 PASS** |
| 完整 native 默认 feature | **a1fd3702 独立 269/269 PASS；48cdd0b4 独立 271/271 PASS**，默认线程栈，单测试线程 |
| Mounted token reload | **a1fd3702 独立 3/3 PASS**，明确启用 native workspace `dev-reload`；48cdd0b4 未改 reload 源码，旧独立 reload 反例在最新统一运行器中再次通过 |
| 上轮独立探针 | **a1fd3702 与 48cdd0b4 均独立 5/5 PASS**，没有修改输入或断言 |
| 新独立反例 | **5 项 FAIL，对应 S1–S4**；S1 有两组件分别验证，其余各一项 |
| CI | a1fd3702 的 [run 37686893583](https://github.com/eddix/gpui-rhai/actions/runs/37686893583) **SUCCESS**；48cdd0b4 的 [run 37717655501](https://github.com/eddix/gpui-rhai/actions/runs/37717655501) 本轮发布前查询为 **completed / success**，不复用旧 HEAD 的成功结果 |
| 基线文件及目标清单 | 本轮检查 **57 PNG / 21 examples PASS**；只代表文件/清单审计，本轮没有重拍全部 GPU 基线 |
| release smoke、RGB、frame budget | 已读取 PR/交付记录；维护者在 a1fd3702 报告 28 smoke、RGB 正对照与 `Window::draw` p95 ≤3.38ms。本轮没有重复运行这些完整实机/性能流程，不混入本轮亲自执行计数 |
| CGEvent 真机流程 | 已读本次 real-window 记录及 harness；结果属于实施方提供的实机证据，不冒称本轮再次操作 OS |

按照 2026-10-07 的收尾共识和更新后的 `release-checklist`，**物理 120Hz 已不再是门槛**。本轮采用 release `gallery_profile` 的 `Window::draw` p95 ≤8.3ms 标准，不再要求补一个已撤销的硬件检查；该 CPU 帧预算也不表述成物理 120Hz 认证。

实施方已经撤回先前不可靠的 RGBA `getbbox()` 比对声明，改用 RGB 和已知不同图片的正对照；旧“415 张完全一致”不应再作为通过证据。建议最终 verification 一并保存精确 SHA、binary、RGB comparison/positive-control 的结构化结果与 p95 数据，便于跨机器核验。

IME preedit 与 VoiceOver 仍按当前共识由维护者验收；Input 失焦后 selection 仍亮、compact traffic lights 约 1pt 偏移、Windows drag 与 Linux span fallback 继续如实列为已知边界。本轮不把这些记录自动改成通过，也不将预先披露、未新增实证的事项重复包装为 S1–S4。

## 实施顺序与下次验收

1. 修 S1：让 decorative hit region 经过同一前景/clip 命中规则，两种 resize 组件一起修，保留伸出区域的合法拖动。
2. 修 S2：明确完整语义交付范围的 origin，覆盖队列中的 action/event，保留 root ancestry 与 Automation 区分，不把消费者当前环境当 producer 来源。
3. 修 S3：从最终有效样式决定 stretch 快速路径；修 S4：默认非滚动 Region body 明确裁剪。
4. 固化本轮正反对照，继续跑原 R1–R5、完整 native/workspace、dev-reload 与精确 head CI。相关视觉和 frame budget 按修改影响复核。
5. 在 GitHub 各 review thread 回复修复 commit 和结果；完整通过后再做版本/依赖/包与发行流程。当前仍不合入、关闭 issues、发布或打 tag。

## 材料与重现

- [新探针](probes.rs)、[旧探针原字节副本](previous-probes.rs)、[运行器](run-probes.py)。
- [新探针日志](evidence/new-probes-final.log)与[Region 补充确认](evidence/region-probe.log)；[a1fd3702 统一运行器结果](evidence/runner.log)与 [48cdd0b4 统一运行器结果](evidence/runner-48cdd0b4.log)为最终可重复执行记录。
- [a1fd3702 workspace](evidence/workspace.log)、[a1fd3702 native](evidence/native.log)、[48cdd0b4 workspace](evidence/workspace-48cdd0b4.log)、[48cdd0b4 native](evidence/native-48cdd0b4.log)、[token reload](evidence/token-reload.log)、[原 R1–R5](evidence/r1-r5.log)。
- [PR 元数据](evidence/pr-111.json)、[issue 需求及回复快照](evidence/issues.json)、[旧 review threads](evidence/review-comments.json)、[CI](evidence/ci.json)、[机器记录](verification.json)。

在待复验代码检出上运行 `python3 run-probes.py --repo /path/to/gpui-rhai --target-dir /path/to/native-cache`。默认跑旧、新两组并启用所需的 `dev-reload`；也可指定 `--suite previous` 或 `--suite new`。本候选预期旧 5 项通过、新 5 项失败；修复后应全部通过。

早期 `new-probes-1.log` 含 Rhai 保留字 `go` 的夹具错误；`new-probes-2.log` 的 selection-none 菜单探索没有证明原假设，不列 finding；`new-probes-complete.log` 中 Region 高度正控尚未固定。最终 Region 用合法 `fill:false` 与显式高度，量得 200px 后再做输入对照。上述早期探索不计产品失败；以最终运行器的行为断言为准。

本轮未修改产品、正式测试、历史审计或外部 dogfooding 项目；只新增审查材料和自己的临时探针。GitHub review 与材料分支将按用户要求发布，审查不以旧探针转绿替代对新增功能的验收。
