# 0.1.8 对抗审查：尺寸、位置与范围控件

- 审查提交：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`，对比 `v0.1.7`。
- 平台：macOS aarch64，Rust 1.95.0，实际 GPUI 依赖 `gpui-pre = 0.3.7`。
- 产品代码、正式 tests、issue 状态均未修改。
- 证据：[独立原生探针](probe.rs)、[完整结果](probe.log)、[生命周期 panic 栈](suspend-backtrace.log)。探针共 10 项：正常容器对照 1 项通过，9 项期望正确行为的断言失败；其中 SplitPane/Draggable 是同一 ownership 缺陷的两个表现。失败不是最终产品测试计数。

结论：这一组不能出货。共享协调器接管指针路由之后，生命周期取消、受控值版本与约束快照仍未统一；现有正常路径测试没有覆盖这些冲突。优先修共同机制，再补控件级别的数值和组合所有权。

## RC-1 / P1：拖动中 suspend 发生宿主 Entity 重入 panic

**实测**：Draggable 开始拖动，移动 30×20 后调用 `view.suspend(window, cx)`，直接 panic：

```text
cannot update gpui_rhai::app::ScriptHostView while it is already being updated
```

触发路径：

1. `app.rs:1021` 已进入 ScriptHostView 的 entity.update。
2. `app.rs:4344` → `quiesce_view`；`app.rs:313` 同步取消此 View 的交互。
3. `interaction.rs:783` 同步执行 cancel 闭包。
4. `draggable.rs:253-254` 清理 preview；`primitive.rs:966` 写 signals。
5. `app.rs:3284` 的 signal dispatcher 再次 update 同一个 ScriptHostView。

这不是脚本恶意输入，也不需要竞态：正常的 View suspend API 与正在执行的公开控件手势组合即可复现。Resizable 的清理曾使用 defer，Draggable 采用同步清理，说明共享 coordinator 尚未约束 cancel 回调可重入边界。

**修复方向**：将生命周期取消和 preview 清理做成宿主允许的事务阶段；或退出当前 Entity update 后统一执行回调，但要同时携带 incarnation/gesture generation，防止迟到的 cleanup 清除后续手势。不能只在一个 Draggable 回调外围 catch panic。

**验收**：活动手势期间 suspend、dispose、Host remove、window close、源节点卸载；验证无 panic、无遗留 preview、无回调提交；恢复后可再次交互。探针名：`active_draggable_suspend_is_safe`。

## RC-2 / P1：手势没有绑定受控源和约束版本，旧 preview 能覆盖新数据

### A. RangeSlider 整段覆盖新 source（本轮新增）

实测步骤：`[20,80]` 的 high 开始拖到 70，释放前通过一个已挂载的按钮动作将受控值改为 `[10,40]`；当时状态已确认是 `10.0,40.0,0`。释放后变成 `20.0,70.0,1`。旧 low=20 也被写回了。

根因：`range_slider.rs:113-124` 虽更新 controlled，但拖动中保留整对 preview，不取消源替换；`range_slider.rs:211-218` 在 mouse-up 用旧 preview 提交完整区间。不能通过只使用新的 high/min/max 消除此类旧低端覆盖。

### B. Resizable 用旧约束提交非法尺寸（迁移引入的风险）

实测步骤：source width=200、max_width=360，开始拖向 300；释放前 max_width 改为 220，source rect 不变；释放后仍提议 width=300。`probe.rs` 的 callback 仅记录 proposal，没有接受它。

根因：`resizable.rs:234` 的失效键仅包含 rect/signals，没有 constraints；`resizable.rs:344-401` 在 gesture closure 捕获按下时的 config。新 props 已经生效，手势仍然使用旧 max。旧的每帧监听器改为长寿命闭包时，没有同步定义参数修订策略。

同类静态风险：SplitPane 仅比较 ratio/signal（`split_resize.rs:128-136`），Draggable 仅比较 position/signals/disabled（`draggable.rs:64-73`）；约束、axes、snap、direction 等变化应按相同版本规则处理。本报告没有把这些未单独运行的变体算为额外已证实缺陷。

### C. Table 同值拒绝不能恢复宽度（既有缺陷，本轮明确承诺统一的验收缺口）

实测：Table 受控 width 固定为 160，on_column_resize 只增加计数、不采纳提议；拖动 40 后 separator x 从 149 变为 189，callback=1，永久保留被拒绝的宽度。`column_resize.rs:275-282` 释放后保持 native override；`column_resize.rs:154-167` 只有 source/signals **值变化**才清理。重渲染同一个 width 不会构成拒绝。

Table 原来的实现已有这种 retained override 行为；这不是本轮新引入的回归。但 ADR 0022 的 Atomic native preview 明确承诺 controlled rejection/source revision 能一致收敛，本轮不能把迁移“完成”作为通过证据。若 Table 要支持 uncontrolled 模式，必须单独给出明确 API，不应让有 callback 的受控列悄悄变成另一种模式。

**共同修复方向**：源修订、约束修订、gesture generation、pending proposal/accept/reject 必须有共同身份。规定 source/constraint 变更是取消还是显式 rebase；如果 rebase，必须对整个受控对象原子生效。不能依靠比较数值猜“上一次提议被接受了没有”。晚到的 accept/reject 不得覆盖新一轮 gesture。

**验收**：保持原值拒绝、不同值接受、clamped accept、拖动中 source 替换、约束收紧、callback 不存在、异步迟到应答、下一轮 gesture 已开始。三个对应探针：`range_source_replacement_cancels_existing_drag`、`resizable_constraint_replacement_cancels_existing_drag`、`table_resize_rejected_value_restores_controlled_width`。

## RC-3 / P1：RangeSlider 的 gap 改写 high 的步进网格

实测：min=0、max=100、step=5、minimum_gap=7、values=[20,80]，点击高端的 75 位置，提交的是 `[20,77]`。普通 gap 参数改变了远离边界的位置的 step 语义。

根因：`range_slider.rs:175-181` 将 `low + minimum_gap` 作为 `normalize_value` 的 min。该函数在 `range_input.rs:514-516` 同时把 min 用作网格原点；因此 high 的网格变成 27+5n，而 low 仍使用 0+5n。之后 `normalize_pair_values` 又按全局 min 对受控值归一化，造成提交值与再次呈现值不同。

此外，源码可直接推导 `normalize_pair_values({low:0,high:4},0,100,10,4)` 生成 `{low:-4,high:4}`（`range_slider.rs:601-605`）：输入满足当前 schema，却能在归一化后越过 min。此例是确定性源码算例，未另计为原生探针。

**修复方向**：分离“全局 step 网格原点”和“当前另一 thumb 带来的合法上下界”；按联合约束求有效 pair，确保任何提交/绘制值满足 min≤low≤high≤max、gap、step 规则。非整倍数 gap、非整倍数终点、不可满足的参数组合要有明确策略；不能只给现有例子加特殊判断。

**验收**：step=5/gap=7、step=10/gap=4、小数 step、非零 min、两端接近 min/max、两个 thumb、Home/End、pointer/keyboard 与 controlled echo 一致。探针：`range_gap_preserves_the_global_step_grid`。

## RC-4 / P1：SplitPane 与 Draggable 接管调用方节点的 key/ref，破坏组件组合（#84）

已用真正的 formal component 验证，而非只看 fluent builder。Cell 的 render 根节点有自己的 key/ref：

```rhai
text("CheckCell").with_key(props.key).with_ref(element_ref("cell_root"))
    .on_click(Fn("check"))
fn check(ctx, payload) {
    let bounds = ctx.element_bounds("cell_root");
    ctx.emit("checked", bounds != ());
}
```

- 直接放进普通 `box([Cell(...)])`：通过，checked=true。
- 直接放进 `SplitPane.start`：`component /View[...]/Cell[cell] has no mounted element ref cell_root`。
- 直接放进 `Draggable.handle`：同样报错。该变体在原 issue 中尚未运行，本审查已确认。

定位：`registry/components/split_pane.rhai:123-126` 覆盖 caller key/style/ref/signal；collapsed 分支 99/104、end 152 也改 caller key/style。`registry/components/draggable.rhai:92-96` 对 caller handle/content 直接装饰。

**修复方向**：组件自有 wrapper 承载组件自己的 key/ref、预览信号及 part style；调用方节点原样放入。需要同时验证 wrapper 对 flex/min-size/百分比高度的影响，不能用“在文档中声明会破坏 ref”代替修复。两个嵌套 formal pane、折叠展开、重新排列和自有 signal/ref 均需保持身份。

**Issue 判断**：[#84](https://github.com/eddix/gpui-rhai/issues/84) 虽建议 0.1.9，建议升为本轮必修；SplitPane 是交互工作台组合基础，新增 Draggable 也继承了同样缺陷。

## RC-5 / P2：Resizable 的自动化键盘路径为空（#85）

对可定位的 `separator / Demo: e resize handle` 执行 `AutomationCommand::Dispatch(event="key:right")`：返回 `Ok(Dispatch { invoked:0, visited:[] })`，width 没有变。不是“找不到对象”，也不是 dispatch 返回 error。

定位：`resizable.rs:674-712` 只有原生 div.on_key_down；`app.rs:3801` 只调用 retained dispatch_plan。相邻 SplitPane 有 retained on_key_value，形成两个对 Host 不一致的路径。

**Issue 判断**：[#85](https://github.com/eddix/gpui-rhai/issues/85) 已确认。推荐这一轮与键盘／native action 入口整理一并修；它不如 RC-1/2 紧急，但放到下轮必须明确留作已知缺口，不能声称所有新增行为可由公开自动化验收。避免给每个组件再造脚本版尺寸计算；原生按键与自动化应汇入同一个语义动作。探针：`resizable_keyboard_is_automatable`。

## RC-6 / P2：固定比例的角柄不能沿其中一个轴调整

实测：200×100、aspect_ratio=2，聚焦 SE 角柄按 Down，callback 提议 width=200，预期 height+8 后 width=216。角柄识别 Down，但比例求解忽略了这次竖直位移。

定位：`resizable.rs:541-547` 比较 `|w/r-h|` 与 `|h*r-w|`。第二项始终等于第一项乘以 r，所以 r≥1 总是选宽、r<1 总是选高，与用户这次真正改变哪个轴无关。

**修复方向**：明确角柄的主轴选择或投影规则；键盘在已知单轴意图下保留该轴并计算另一轴。不要用两个单位尺度不同且线性相关的误差决定主轴。探针：`aspect_corner_keyboard_vertical_resize_is_effective`。

## #83 是否进入本轮

[#83](https://github.com/eddix/gpui-rhai/issues/83) 自定义 SplitPane / Resizable grip 是真实扩展缺口，当前 schema 没有 handle node/per-handle slot；默认线条由 native paint 生成。它涉及 paint order、overhang hit target、原生 hover/drag/focus 状态绑定；在本轮 P1 仍未关闭时，不宜只加一个 Rhai prop 仓促交付。

建议保留 0.1.9 enhancement。当前先修 #84，让所有权边界正确；后续再增加自定义视觉节点且保留分隔条单一 focus/action owner。4px line inset 是否 token 化可并入同一设计，不能把添加装饰节点误当作解决了命中与层叠。

## 复现命令和测试边界

从仓库根目录运行：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8/resize-controls/Cargo.toml \
  --test probe -- --nocapture --test-threads=1
```

这是独立 workspace，锁文件来自当前原生测试依赖图，避免为审查引入新版本依赖；复用已有 target 目录以免重复生成大型产物。`probe.rs` 的只读 include 路径指向本地仓库，未修改正式 suite。使用 GPUI VisualTestContext 的实际 layout/input/retained Runtime，不是纯 mock；没有声称执行真实 OS VoiceOver、Metal 截图或全量平台验收。

额外 panic 栈：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target RUST_BACKTRACE=1 cargo test --offline --locked \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8/resize-controls/Cargo.toml \
  --test probe active_draggable_suspend_is_safe -- --exact --nocapture
```
