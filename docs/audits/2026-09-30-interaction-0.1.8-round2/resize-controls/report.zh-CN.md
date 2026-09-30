# 0.1.8 第二轮对抗审查：位置、尺寸与范围控件

- 当前提交：`d77b4b49a667da618fe9b9583d331d8c325cf180`。
- 对比整改前：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`。
- macOS aarch64 / Rust 1.95.0 / gpui-pre 0.3.7。
- 本轮只新增此审计目录；没有修改产品、正式测试、旧审计或 GitHub 状态。
- [原生探针](probe.rs) / [原生执行结果](probe.log)：6 项，1 项通过、5 项按正确行为断言失败，对应下列 4 个根因。父审查统一复验旧探针，本分支没有重复运行旧探针。

已按修订后的 ADR 0022 审核：接受“完成一次手势后发出提议、受控 rerender 取消捕获的手势、提议派发后清理覆盖值”的当前契约，不再要求旧版 awaiting-control 状态机。本轮仍发现执行入口没有统一、合法参数走到非法中间状态的问题。

## R2-RC1 / P1：合法 RangeSlider 端点组合触发 native render panic

整改后的 `normalize_pair_values` 先把两个值分别吸附到网格，再尝试向上扩展 high。当 low 已被吸附到 max，扩展 high 的可行区间反向，后续“再调整 low”根本没有机会执行。

**最小合法参数**：

```rhai
range_slider::RangeSlider(#{
    key:"range", label:"Legal endpoint", values:#{low:95.0,high:100.0},
    min:0.0,max:100.0,step:10.0,minimum_gap:5.0,on_change:Fn("changed")
})
```

这些值通过当前公共 schema 与组件校验；范围中也确实存在合法的网格解，例如 `[90,100]`。但两个值先被吸附成 `[100,100]`，接下来把 high 求解为 `feasible_min=105, feasible_max=100`，触发：

```text
min > max, or either was NaN. min = 105.0, max = 100.0
```

**行为证据**：探针 `range_legal_endpoint_gap_must_render_and_accept_input` 能挂载 retained group，但原生 primitive render 被 guard 捕获并替换为 error 元素，点击控制区域 callback count=0。因此不把它夸大为进程必然崩溃；实际问题是合法组件配置无法渲染／交互。

**位置**：`crates/gpui-rhai/src/range_slider.rs:630-644` 的联合归一化，以及 `:663` 对可能反向的区间执行 clamp。

**整改要求**：先求“全局范围 ∩ step 网格 ∩ 两端间距”的可行 pair，再决定往哪一端调整；任何局部求解都不得把空区间传入 clamp。至少覆盖 low 接近 max、high 接近 min、非整倍数 gap/step、非零 min、非整倍数 max；加上归一化幂等性和输出总满足 bounds/gap 的性质检查。不能仅把 105 改 clamp 到 100 然后留下间距不满足的结果。

## R2-RC2 / P1：异步 rerender 绕过了手势取消，旧 axes 仍能提交

上一轮 source/constraint probe 使用节点按钮动作，整改在 `handle_node_event` 成功重渲染后取消手势，所以能通过。异步定时器／任务派发走另一条路径，没有经过该取消点。

**实测**：

1. Draggable 初始 `position={20,20}, axes="horizontal"`，开始向右拖 40。
2. 由组件声明的 `timeout(...,100,...,Fn("change_axes"),())` 将 axes 改成 `"vertical"`。探针使用正式 `AutomationCommand::AdvanceTime` 和 ManualRuntimeClock 驱动，没有调用私有方法。
3. 松手前确认状态为 `vertical:none`；松手后却收到 `vertical:60.0,20.0`，提交的是旧 horizontal 契约的结果。

**位置**：

- `app.rs:4752-4753`：取消仅附在节点回调成功 rerender 后。
- `app.rs:4994-5005`：异步 delivery 执行 `render_dirty` 后仅汇总 changed，不使手势失效。
- `draggable.rs:49-55`、`:67-74`：控件自身 source identity 不包含 axes/contain/snap 等约束。
- `draggable.rs:238-247`：finish 使用按下时捕获的旧 config。

这直接违背修订 ADR 的“source/constraint rerender 取消 captured gesture”契约。它不依赖是否有晚到的应答，也不是重提旧 ADR。

**整改要求**：在所有成功提交的新渲染契约的共同边界处理失效，而不是在每种 delivery 的调用者末尾补一个 cancel。若选择控件局部判定，要完整版本化捕获的参数、坐标与几何契约；核对 timer、task/subscription、Host collection/document/theme、motion callback 和普通节点回调的一致性。取消回调仍需遵守前轮修好的 Entity 重入与代际规则。

探针：`async_axes_change_cancels_draggable_old_gesture`。

## R2-RC3 / P1：Table 仅 pointer release 清 preview，keyboard 和 autofit 仍永久保留被拒绝的宽度

整改在 pointer finish 增加 deferred clear，但另外两种正式入口继续写同一条 native width signal 而不清理。

**同一个最小 Table**：左列受控固定宽度 160；on_column_resize 只记录 callback 次数，不采纳提议。结果：

| 入口 | 释放／派发后的分隔条 x | callback 次数 | 期望 |
| --- | --- | --- | --- |
| keyboard `key:right` | 149 → 157 | 1 | 恢复 149 |
| double-click autofit | 149 → 37 | 1 | 恢复 149 |

探针使用公开 Automation Dispatch 驱动 keyboard，以及真实 GPUI MouseDown click_count=2 驱动 autofit；不是直接调用求解函数。

**位置**：

- `registry/components/table.rhai:186-196`：keyboard 写 ctx.set_signal 后 emit，缺少完成清理。
- `column_resize.rs:232-245`：double-click 写 signal 后 propose，缺少完成清理。
- `column_resize.rs:287-292`：新清理仅放在 pointer finish 分支。

**整改要求**：统一 pointer、keyboard、autofit 三个入口的“提议／清理”收尾流程。保持 callback 缺失、接受、拒绝、clamped accept 的明确语义，不要再各加一段时序不同的 cleanup。该项是上一轮同值拒绝整改覆盖不完整，不能把通过旧 pointer probe 当作整个问题关闭。

探针：`table_keyboard_rejection_clears_native_override`、`table_autofit_rejection_clears_native_override`。

## R2-RC4 / P2：SplitPane 键盘从原始 ratio 而非已约束的可见尺寸步进

**实测合法配置**：总宽 400，ratio=0.8，min_start=min_end=80，默认 handle=8。native pointer/layout 求解将可见 start 约束到 312；按一次 Left 后，ratio 变为 0.78，但分隔条仍在 312，没有向左移动 8。

`split_key` 用 `source_ratio * group_size = 320` 减 8；native 求解起点则已是 312。键盘路径的 max 也没有扣除 handle 占用，因而与 pointer 的合法区间不同。

**位置**：`registry/components/split_pane.rhai:72`、`:77-83`。代码把 start_ref 放入 payload，却没有用来获得当前已呈现尺寸。

**整改要求**：keyboard 与 pointer 共享同一尺寸／约束解算。从当前呈现的受控解或明确归一化后的 source 开始移动，并统一 handle、min/max、RTL 与缩小窗口后 overconstrained 的策略。不能靠调用方禁止合法的 source ratio 接近边界来避免问题。

探针：`split_keyboard_can_shrink_a_clamped_start_pane`。这是新发现的既有缺陷，不是声称 wrapper 修复新引入。

## wrapper 改动的本轮正向验证

`split_wrappers_preserve_nested_percentage_geometry` 通过：400×200 的 outer SplitPane，start 中嵌套 vertical SplitPane，百分比铺满的三个内容区实际尺寸分别为 200×100、200×92、192×200，符合两个 8px handle 的空间分配。没有把增加 wrapper 本身认定为架构错误；它是修复 caller ref/key 所有权的正确方向。本对照只证明该嵌套百分比场景，不等同于所有 flex/scroll/overflow 组合都已验证。

## 重现命令

从仓库根执行：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round2/resize-controls/Cargo.toml \
  --test probe -- --nocapture --test-threads=1
```

独立 workspace 使用当前锁定依赖和旧原生套件的构建缓存。每个断言表达期望行为，当前失败是反例证据；不能通过把 expected 改为现有错误结果来作为产品回归测试。
