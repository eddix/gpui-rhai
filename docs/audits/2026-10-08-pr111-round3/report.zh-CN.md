# PR #111 第三轮：S1–S4 修复验收

审查开始：2026-10-08；交付：2026-10-09。精确 HEAD **`2c5e8a079304bb0c8910ddbd73eca772f49fd453`**，对照上一轮 `48cdd0b4`。本轮新增五个提交：字段 padding 设计调整，以及 S1–S4 四项修复。

**代码与已复验契约：通过，S1–S4 可关闭；未确认新的 P0/P1/P2。** 不再要求一轮大范围重构。下文 N1 是非阻断的光标反馈建议，不是新的发布否决项。

这不是整个发行流程已经完成：IME preedit/VoiceOver、统一 0.2.0 版本与依赖、最终发行检查仍按既定流程收口；本轮没有合入、关闭 issues、发包或打 tag。

## 独立执行结果

| 检查 | 本轮实际结果 |
| --- | --- |
| Workspace，all-targets/all-features，locked/offline | **680 passed，0 failed** |
| 独立 native workspace，默认线程栈、单测试线程 | **275 passed，0 failed** |
| 第一轮 R1–R5 原探针 | **5/5 PASS**，源码与原归档逐字节一致 |
| 第二轮 S1–S4 原探针 | **5/5 PASS**，S1 分别测 SplitPane/Resizable，源码与原归档逐字节一致 |
| 精确 HEAD CI | **SUCCESS**：[run 37735020988](https://github.com/eddix/gpui-rhai/actions/runs/37735020988)，包括 Mounted hot reload、严格检查及 Linux 验证 |
| 基线/清单文件审计 | **57 PNG / 21 examples PASS** |
| 已提交基线的 RGB 差异核对 | 48cdd0b4→2c5e8a07：**43 张变化，14 张不变**，正反对照有效 |

日志：[workspace](evidence/workspace.log)、[native](evidence/native.log)、[前两轮原探针](evidence/previous-probes.log)、[CI](evidence/ci.json)。本轮没有在本机额外重复完整 Clippy、release smoke、frame-budget 或所有 GPU 重拍；这些项目的 CI/实施方结果与本轮执行分开表述。

## S1：装饰手柄的实际命中边界——通过

已去掉裸矩形包含关系兜底。带 ElementRef 的节点在内容 prepaint 后记录本帧 Normal hitbox，继承祖先 content mask 和当前绘制顺序；原生手柄通过 `element_hitbox` 查询，并交给 GPUI 判断 hovered。几何注册表每帧清空 hitbox，保留 active nodes 时也清除旧条目，没有把失效 ref 的旧矩形留作命中依据。

核对锁定 gpui-pre 0.3.7 源码：`Window::insert_hitbox` 保存当前 content mask；命中时先与 mask 取交集，再按前景遮挡截断。`Normal` 本身不遮挡其他命中项。因此这是复用了 GPUI 的命中规则，不是另写一份矩形遮挡推测。

原独立对照现在得到：

| 场景 | SplitPane 最终比例 | Resizable 最终宽度 |
| --- | ---: | ---: |
| 无 grip，按伸出位置 | 0.5 | 200 |
| 有 grip，无遮挡 | 0.6714285714 | 240 |
| 有 grip，被前景 occlude 覆盖 | **0.5** | **200** |

新增正式测试还覆盖裁剪外侧不能拖、裁剪内侧仍可拖。它们在本轮完整 native suite 中通过，合法 overhang 没有被禁掉。

## S2：排队动作与组件事件的 origin——通过

`UiContext::dispatch_action` 和 `emit` 在 enqueue 时保存当前 origin，`invoke_pending_effects` 逐项使用该记录执行，并由既有 `with_origin` 恢复之前的来源。处理的是 producer 来源，不再依赖 drain 时的环境。`ActionInvocation` / `PendingEvent` 的新增字段已在变更记录中说明，不要求历史兼容别名。

原独立探针中，task completion 直接调用和它派发的 action 现在都为 `TaskCompletion { started_by: UserInput { click } }`；click 直接调用与 click action 的正对照仍一致。

正式 `invocation_origin` 扩展覆盖 click、task、timer、effect 的排队 action，以及 timer emit 的组件事件，均在本轮 native suite 中通过。Subscription 经过同一 delivery/queue 路径，本轮做源码核对，没有冒称另跑独立 Subscription case。失败恢复沿用 `with_origin` 的 Result 返回路径；本轮没有额外声称穷举所有失败交错。

## S3：从最终有效样式判断 stretch——通过

渲染顺序已调整为：普通/交互样式 → motion 尺寸 → signal 值 → signal-selected style → RTL/stretched-layout 规则；子布局提示使用同一结果。margin、width、align、position 不再在快速路径选择后才出现。

原探针的静态 margin 与 signal margin 均为 **x=20、width=260**。正式新增测试比较六种 signal 层（margin、逻辑 start margin、auto margin、self-center、absolute、width），覆盖 LTR/RTL 和 on→off→on；原 R1 普通布局对照保留，均通过。

因此第二轮指出的“R1 原入口修好、signal_style 新入口仍漏判”已关闭。

## S4：Region 默认裁剪——通过

非滚动 body 显式 `clip()`，scroll=true 仍使用自己的滚动视口。原探针固定 Region 高 200px，点击底部以下 100px：两种配置现在都为 **0 次**，不会触发内部长内容。

正式新增回归同时检查填充 quad 的 content mask 与命中：外侧不能触发、内侧仍可触发，且沿用 header/footer 不随 body 滚动的控制。本轮完整 native suite 通过。

## 新增 D61 字段 padding

实现将输入字段从 `metrics.control_pad` 分离到 `metrics.field_pad`，Textarea 使用 `metrics.multiline_pad`；Button/Tabs/ToggleGroup 保持原控制 padding。这些值仍归 token base，并按 density/size 解析；组件 metadata 的 token 声明和规格同步更新，没有把数值散落进 runtime。

新增正式 token 测试在本轮通过：两种 density × 四种 size，字段 padding 小于按钮 padding，md 的边框加 padding 与 `metrics.inset` 对齐，Textarea 的 vertical padding 与声明规则一致。

本轮独立用 Pillow 转 **RGB** 比较仓库中 48cdd0b4 与 2c5e8a07 的 57 张图片：19 张 Gallery、24 张 example 改变；14 张 Dashboard 不变，与本次声明相符。比较深/浅两张 Button 图作为已知不同正对照，比较同图副本作为负对照，均正常。结果见 [baseline-rgb-delta.json](evidence/baseline-rgb-delta.json)；[可复现脚本](compare-baselines.py)已再次运行，得到[相同结果](evidence/baseline-rgb-replay.json)。这证明的是已提交图片的差异，不冒称本轮重新从 GPU 捕获了全部页面。

另逐张查看了 compact/light 的 Gallery Form 与 light Form Showcase，字段文字和多行文本的起始边、边框及空间比例没有发现新的视觉阻断。

## N1 / P3：Resizable 装饰 grip 的方向光标可补齐（非阻断建议）

源码对照中，SplitPane 的装饰 shape 显式设置 resize cursor；Resizable 的 `resizable_grip` 未设置 cursor，而 Rust 侧 cursor 仍绑定原生手柄 hitbox。装饰节点有自己的 occluding hitbox，建议按 handle 方向给 grip 明确设置光标，并覆盖 lane 外的 hover。定位：`registry/components/resizable.rhai` 的 `resizable_grip`，对照 `split_pane.rhai::split_grip`。

这是源码层的可用性完善建议；本轮没有读取 OS 实际 cursor 状态，不将其升级为已实测的功能回归，也不据此阻断 S1 的命中修复验收。

## 发布与协作收尾

- 本轮原有代码阻断已消除；#83、#91、#117 对应的 S1–S4 缺口可以按上述范围验收。此前通过的其他 issue 不重开；issue 本身待实际集成时按流程处理。
- 沿用已更新的 release `Window::draw` p95 ≤8.3ms 标准，物理 120Hz 不再作为门槛。实施方报告本 head p95 ≤2.94ms，本轮未重跑该 benchmark。
- IME preedit、VoiceOver 仍由维护者完成。已披露的平台/光标/失焦选区等边界继续如实记录，不把自动化测试或静态 PNG 当作 OS 人工验收。
- 三 crate 与生成依赖仍需按计划统一到 0.2.0，并完成最终 package/干净消费者及获准后的发行步骤。当前通过是代码范围结论，不是合入、发包、tag 授权。
- 建议接下来集中完成这些既定收尾。没有新的失败证据前，不扩大成第四轮大范围架构重构。

本轮材料将随 GitHub Review 交付，并在 S1–S4 原线程回复复验结果。产品、正式测试及外部 dogfooding 仓库未修改。

## 重现

[运行器](run-probes.py)默认执行两轮全部十项原始探针；[第一轮源文件](previous-probes.rs)、[第二轮源文件](probes.rs)保持原字节。对本 head 预期 10/10，通过 `--repo` 指定待验收检出，`--target-dir` 指定现有 native cache。运行器只临时添加自己的测试并清理，不改产品文件。

机器记录：[verification.json](verification.json)。原始 PR 与修复回复见 [PR 快照](evidence/pr-111.json)、[评论快照](evidence/comments.json)。
