# 第七轮：交互、窗口 lease 与 reconcile 交叉验收

基线 `cda11ce7`，对照 `83b19b1d`。本分支只检查 gpui-rhai 的交互与本轮生命周期变更，不分析或执行 DiskTree。

**本分支没有确认新的缺陷。4 个小型原生探针全部通过。** 这不代替主审查的窗口 authority、共享 store/contribution、批量事务或完整 native suite 验收。

## 动态验证

复跑第五轮 3 个原始 gap/cancel 场景，源码复制到本轮独立 package，没有改历史断言：

| 场景 | 实际结果 | 结论 |
|---|---|---|
| 静止指针处于真实空隙 | `(scroll_item,offset)` 从 `(0,0)` 到 `(2,36)` | 保持 native tick 延续能力 |
| 先经过有效目的地，再进入内部 `.occlude()` 且拒绝 payload 的区域 | 保持 `(0,0)`；Escape 后 cancelled | 第五轮反例已关闭 |
| 取消旧会话，随后新拖动直接进入未经验证的 gap | 保持 `(0,0)`，最终 rejected | 旧 tick 没有把 destination 转给新会话 |

新增同 Host、双 View 的 reconcile 交叉：

1. 两个独立 PreparedScriptView 共享一个 ScriptViewHost；左 view 是 DragSource，右 view 是 DropZone 和自己的状态按钮。
2. 左源已经开始拖动并进入右目标后，通过公开 Automation Dispatch 调用右 view 的正式 click callback。它执行真实 Rhai state 更新，不是直接改测试计数器。
3. 先断言右 view 的状态确实由 `0:0` 变为 `1:0`；随后经过 native tick 和额外的 `window.refresh()`。
4. 松开鼠标，左 view 得到 `accepted`，右 view 最终为 `1:1`，仅完成一次 drop。

这覆盖了 sibling view 自己的 state commit、目标重新呈现和纯 GPUI 重绘不会误取消另一 view 的有效手势。它不要求源自己的受控业务 contract 变化后继续手势；新版 ADR 对那种变化仍明确允许取消。

## 源码检查

- 本轮 `interaction.rs` 与 `virtual_list_element.rs` 相对 83b19b1d 没有新增改动；现有容器 hitbox 已在行子树 prepaint 之前注册（`virtual_list_element.rs:107`–`:110`），与本次遮挡探针一致。
- 新的 `HostViewRegistration` 拥有 mount lease。`app.rs:381`–`:401` 在删除焦点、overlay 和 `interactions.discard_view(view_id)` **之前**检查 lease identity；过期注销不会只凭同名 view_id 清掉新实例的交互域。
- `app.rs:4915`–`:4924` 移除 native window registration 时也比较当前 registration 的 authority lease 与自身 mount lease。一般交互 view 的注销与 native 命令 owner 不是同一个布尔权限概念。
- 本轮 `engine.rs:1414` 起在一次 batch 的最终 invocation graph 上提交状态与生命周期；没有在 interaction 层引入第二份完整/增量状态权威。具体事务合并、reader contribution rollback 的正确性由对应专项继续验收，本报告不据静态形状宣称全部通过。
- 源业务渲染与虚拟 realization 的区分仍保留；本次没有发现 window registration lease 改动把它们合并成“任何重绘都取消”的路径。

## 范围与证据

本次没有新增大 target 或 profile，只使用一份本轮独立 probe package 并复用既有 native cache。没有运行大规模 benchmark、完整 workspace、跨窗口 OS drag 或其他产品。真实用户界面视觉验收不在这些 TestAppContext 结果中。

```sh
cargo test \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round7/interaction-crosscheck/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

最终日志：[probe-results.log](probe-results.log)。完整探针：[probe/src/lib.rs](probe/src/lib.rs)。结果 **4 passed / 0 failed**。产品、正式测试与历史审计保持只读。
