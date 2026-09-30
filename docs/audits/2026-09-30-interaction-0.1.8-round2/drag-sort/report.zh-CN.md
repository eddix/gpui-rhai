# 0.1.8 第二轮：Drag / Sortable 复审

基线：`main d77b4b49`，对照上一轮 `0b9b887c`。依据修订后的 ADR 0022 检查，不把第一版 ADR 中已明确调整的要求继续当作缺陷。

本次新增 5 个独立原生探针：3 通过，2 失败。确认两项需要整改的虚拟排序问题；普通绘制遮挡和显式键盘目标的边界行为不列为缺陷。未修改产品、旧审计或正式测试。

## R2-DS-1 · P1：虚拟源离开可视区后仍逻辑存在，却被当作失效交互取消

定位：

- `crates/gpui-rhai/src/interaction.rs:462`–`472`：`finish_frame` 以本帧 `presented` 集合判定 active owner 失效，直接取消。
- `crates/gpui-rhai/src/sortable.rs:204`–`215`：源身份仅在行真正 paint 时 `present_interaction`。
- `crates/gpui-rhai/src/virtual_list_element.rs:103`–`125`：新的 collection/index pin 只要求源继续 realized；它不使离屏行继续经过 GPUI paint。
- 同文件 `311`–`314`：GPUI variable list 只回调实际参与绘制的行。

真实复现：

1. 挂载固定的 100 项 `virtual_data` Sortable，视口 180px，源为 Item 0。
2. mouse down 后移动至可见列表中间，已经开始合法拖动。
3. 保持鼠标按下，向同一列表发送原生 wheel；没有触发业务事件，没有替换数组，没有改变 order、key、disabled 或约束。
4. 列表实际滚动，最终 realized 由 Item 0..4 变为 Item 2..6；再 move/up，状态仍是 `ready`，没有 reorder。

这不是新 ADR 所允许的“业务 source/constraint 成功重渲染后取消旧手势”：场景中的业务数据始终不变，变化只来自虚拟列表滚动及 realized 窗口维护。`source_index` 仍为 0，`key(0)` 仍是 Item 0。若将内部 realization 也一律解释为业务变更而取消，ADR 同时承诺的虚拟拖动 pin 将无法成立。

影响：大列表里已开始的排序无法跨越初始可视窗口，即便源在逻辑集合中仍然有效。新索引 pin 正确减少了查找复杂度，但没有完成源生命周期与可见性的分离。

整改：目标必须按 presented hitbox 命中，源的有效性则应按受控集合成员身份、retained 实例/挂载世代和拖动租约验证。对合法的虚拟源，离屏不等于卸载。不要恢复上一版“只要 active drag 存在就永远豁免 stale”的无条件规则；真正删除、禁用、重建、投影改变或 dispose 仍必须取消。pin 的释放与有效性应由同一集合身份管理。

验收：保留本次真实 wheel 探针；再覆盖边缘自动滚动、连续跨多个 realized 窗口、结束后释放 pin、Escape、删除源、相同 key 重建、外部投影更新。仅断言最终 source_key 正确不能证明源确实离过屏。

## R2-DS-2 · P2：虚拟 Sortable 没有连接实际 ListState，边缘自动滚动不生效

定位：

- `crates/gpui-rhai/src/interaction.rs:880`–`915`：auto-scroll 只遍历 `DropTargetRegistration.scroll_handles: Vec<gpui::ScrollHandle>`。
- `crates/gpui-rhai/src/virtual_list_element.rs:179`、`192`–`201`、`311`：虚拟列表实际滚动由私有 GPUI `ListState` 管理。
- `crates/gpui-rhai/src/renderer.rs:3564`：行 runtime 继承普通 retained 滚动句柄，没有把本列表的 ListState 注册到拖放滚动链。

真实复现：100 项、180px 视口，从 Item 0 拖到下边缘内 5px，进行 30 次交替 1px 的鼠标移动，每次都执行 foreground/绘制流程。最终 realized 仍是 Item 0..4，释放只能得到 `item-0:before:item-4`。没有向下滚动，也就无法继续定位更多候选项。

原有 `tests/native-keyboard/tests/control_visuals.rs:1060` 的 `virtual_sortable_pins_the_dragged_key_while_realization_moves` 同样进行边缘移动，但仅检查 status 以源 key 开头；它没有检查 scroll offset、visible range 或 realized 集合真的变化。当前无滚动的行为也能让该断言通过，因此该测试不能证明命名中的“realization moves”或“pins”。

整改：把普通 ScrollHandle 和虚拟 ListState 都接入统一、带集合归属的原生 auto-scroll 适配边界。保留边缘策略和外层滚动链优先级，实际 offset/reveal 由各自原生滚动系统处理。不要把所有虚拟行实现出来，也不要在 pointer move 中调用 Rhai 来绕过 ListState。

验收：断言滚动位置/首个可见 key 确实持续变化，并能在越过初始窗口后提交正确 keyed anchor；视口边界、嵌套普通滚动区、到达末尾、指针离开边缘和取消后必须停止。此项与 R2-DS-1 的修复需要一起验收，否则自动滚动恢复后仍会立刻触发离屏取消。

## 已确认的正向行为与不应误报的边界

- **同 Host 跨 View DragSource/DropZone**：新增双 View fixture。真实鼠标拖放使目标 count 从 0 到 1；点击源后 Enter 通过显式 keyboard_target 使 count 从 1 到 2。通过。
- **显式 `occlude()` 的无关前景**：前景普通 clickable 面板真实点击成功，覆盖后景 DropZone；拖到同一区域，背景回调不发生。通过。这验证新的 Hitbox 查询确实消费 GPUI 遮挡策略，而非只比较 DropZone 的 paint_order。
- **普通 Normal hitbox 的前景**：没有 `occlude()` 的前景仍允许背景 drop。源码和 `docs/style.md` 已提供显式 occlusion 契约，GPUI 也明确 Normal 不阻断其他 hitbox；前景 click callback 的 stop 只作用于 click 路由。因此不把这项当作安全漏洞或未修复 P1。
- **模态框内源的显式 keyboard_target 指向背景**：Enter 会调背景目标，鼠标拖动没有触发背景目标。此行为已做原生 characterization。公开契约目前承诺相同 type/operation 协议，并没有规定显式语义目标必须可被鼠标命中；类似模态确认操作可以修改外部业务数据。这应作为后续明确“键盘语义目标与 modal scope”政策的文档事项，而非本轮直接阻断。探针名称和断言明确标注 characterization，不作为模态安全验收。
- **scoped collection/index 设计**：新 collection identity 来自完整组件 scope，查找验证 `key(index)==source_key`，不再全局扫描。它对局部 key 隔离和索引成本的修复方向正确。数据发生真实更改时按新 ADR 取消旧手势是允许行为，不把这种取消列为 bug。本分支没有重复跑全部上一轮探针，统一结果由主审查记录。

## 复现与证据

```sh
cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round2/drag-sort/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

最终日志：[probe-results.log](probe-results.log)。完整原生探针：[probe/src/lib.rs](probe/src/lib.rs)。

最终应为 3 passed、2 failed；失败保留为修复后的验收断言，不应修改为接受错误行为。这里的测试使用 GPUI TestAppContext 实际 layout/input/paint 路径，未宣称做过手动 GUI 或操作系统级跨窗口拖放验收。
