# 0.1.8 第五轮：目标 ancestry 修复后的内部遮挡与 native tick

基线 `23fce238`，对照 `f6e936a5`。依据更新后的 ADR 0022 检查。
新增 3 个原生探针，最终 **2 passed / 1 failed**，确认 1 项 P2。未修改产品、正式测试、历史审计或其他会话目录。

## R5-DS-1 · P2：行间延续滚动看不到虚拟列表内部的显式遮挡

当前真实 retained ancestry 的方向正确，第四轮“同 View 矩形相交等于祖先”的问题已经从本次代码中移除。本次反例涉及新增的 **row-gap continuation**，不是重复第四轮外部兄弟覆盖层场景。

定位：

- `crates/gpui-rhai/src/virtual_list_element.rs:107` 先执行整个列表子树的 prepaint。
- `virtual_list_element.rs:109` 随后插入 viewport 的 Normal hitbox，注册为延续滚动的可见性依据。
- `crates/gpui-rhai/src/interaction.rs:633`–`646` 在没有接受目标时，使用上次验证的 container identity，并通过该 viewport hitbox 的 `is_hovered_at` 判断是否继续滚动。

由于 viewport hitbox 在自己的行子树之后插入，它排在内部 occluding 子节点前面。GPUI 查询 hitbox 时已经先收集这个容器命中，后来遇到内部 `BlockMouse` 也无法撤回它。因此该谓词无法证明“容器当前位置没有被自己的子节点遮挡”。

### 严格对照场景

100 行虚拟列表，高 180px；每行高 60px，前 40px 是接受 `card` 的 DropZone，后 20px 默认是空隙。外部 DragSource 提供 `card`。

比较两个只有空隙内容不同的版本：

1. 真正的 20px 空隙。
2. 该区域放入另一个只接受 `other` 的 DropZone，并明确 `.occlude()`。

拖动先经过有效 Lane 1，建立接受目的地；再移动至列表底边内 5px，之后不再移动指针，只推进 12 次 16ms 的 native tick。

| 场景 | 起始滚动位置 `(scroll_item, scroll_offset)` | tick 后 | 期望 |
|---|---|---|---|
| 真正空隙 | `(0, 0)` | `(2, 36)` | 可以继续，正向对照通过 |
| 显式 occlude、拒绝 payload 的内部子节点 | `(0, 0)` | `(2, 36)` | 应停止，断言失败 |

失败探针还先验证了两项前提：

- 指针坐标确实落在 `Blocked Lane 2` 的实际 GPUI 几何内。
- 不先经过接受行、直接拖入同一阻挡区域，原生 drop 结果为 `rejected`，滚动保持 `(0,0)`。因此不是这个节点实际上接受了 payload，也不是测试误把空白位置当成遮挡层。

两种场景最终 Escape 都正确取消，`last_error=None`。问题集中在先前接受过同一容器后，拒绝/遮挡区域仍被延续规则当成空隙。

### 影响和整改

列表内部的不可放置区域、阻挡层或覆盖内容不能停止已开始的边缘滚动；指针静止时列表仍会自行移动。该行为违反新版 ADR 中“只能在可见且未遮挡的 row gap 延续”的条件。

延续滚动的 viewport 命中应处于真实容器层级，能被其后绘制的 occluding 子树阻挡。需要使用当前帧实际 viewport；不要简单提前读取上帧 `ListState.viewport_bounds()` 来换取命中顺序正确。也可以让延续条件显式验证目标区域的遮挡/拒绝状态，但不能把 `target=None` 一律视为空隙。

保留接受 target 的 ancestry 与上次验证的 container identity 是必要的；本次不建议退回扫描全部 targets、比较矩形面积或按 source collection 决定滚动。

验收应同时覆盖真实空隙、内部 `.occlude()`、外部 `.occlude()`、拒绝目标、嵌套接受目标以及取消。应保留本次“先建立目的地再进入阻挡区域”和“直接进入同一阻挡区域”的区别，防止只有冷启动命中测试通过。

## 本次确认通过的边界

- **静止指针可以继续通过真正空隙滚动。** 探针推进时间并让正常任务队列处理，没有用额外 pointer move 或显式 window.refresh 代替 native tick。
- **取消后的新会话没有继承旧 destination。** 第一场拖动验证 Lane 1 后，在 tick 待执行阶段 Escape/up；随后新拖动直接进入未验证的空隙。等待 12 tick 后仍为 `(0,0)`，最终 drop 为 `rejected`。
- **本次外部 DragSource 的 Escape 取消有效。** 两个 row-gap 探针最终均收到 `cancelled`，没有继续提交 drop；这里的源仍在窗口中可见，不冒充虚拟离屏源租约的专项验证。

读取了本轮真实 parent-chain 查找、container identity、tick 排队与取消实现。跨 View、deactivation、先前 offscreen lease 和第四轮原始命中回归由主审查统一复跑，本分支不重复声称独立验证。没有扩大到明确排除的跨窗口/OS 文件拖放。

## 证据与重跑

```sh
cargo test \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round5/drag-sort/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

完整日志：[probe-results.log](probe-results.log)。源码：[probe/src/lib.rs](probe/src/lib.rs)。
最终为 2 passed / 1 failed。使用独立本轮 package/源码，复用现有 native target 缓存。失败保留为产品预期行为断言，修复后应转绿。
