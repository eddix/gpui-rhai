# Motion Runtime 2 审查整改记录

对应审查基线：`9f4ba04fc9c5198d7164359f47139a5d306de1bc`。

本文件记录审查后的产品整改；原始报告、探针和证据保持不变。缺陷复现探针仍然按旧错误行为断言，不能作为整改后的通过测试。正确行为已迁移到产品单元测试和 `tests/native-keyboard` 的真实 GPUI 测试。

## M01–M12 状态

| ID | 整改 |
| --- | --- |
| M01 | 修正 Gallery 的无效 Style 调用；原生测试真实 mount、提交首帧，并操作 timeline 与 keyed reorder。Gallery 增加 play/pause/seek/restart、受控 Tabs、reorder 和 shared-layout 切换。 |
| M02 | `MotionHandle` 绑定 runtime、presentation domain、component、incarnation、generation 和 timeline instance。兼容 rerender 保留句柄；迁移生成新句柄并保留进度；旧实例与跨 view 控制明确失败。 |
| M03 | property、timeline、layout 和 trigger 统一执行有效 `MotionPreference`；`None` 立即投影到静态/终态，play/restart 不能绕过。 |
| M04 | timeline 由明确 position 生成完整快照；pause 先采样命令时刻，seek 立即更新，重复 play 幂等，future tracks 在回退时恢复初态。 |
| M05 | direct/timeline 共用 transition、keyframe、spring、inertia 采样器；spring retarget 继承速度；完成保存实际样本；极端物理参数在接纳前拒绝。有限 source duration 在接纳时缓存，避免逐帧重算 spring settle。 |
| M06 | 候选提交前解析 timeline target 并验证目标能力；direct、progress、NativeSignal 和 timeline 使用同一 property owner plan，冲突与缺失目标原子拒绝。 |
| M07 | 公开 reconcile 在 clone 上规划后提交；play/restart 重查预算。layout/trigger 按 presentation domain 预留同一 runtime active budget，多窗口合并计算；替换计划先释放不再存在的旧 source。shared-layout 历史按已接纳 identity 与已呈现帧回收。 |
| M08 | exit 从 retained `unmounted` NodeId 和对应 view geometry 建立；ghost 位于原 presentation scope，按 domain 渲染和清理，关闭窗口取消 source。支持的 paint subtree 被清除交互/资源/外部布局；不支持的嵌套类型在 reconcile 阶段拒绝。 |
| M09 | suspend 是 MotionRuntime 内的 scope 状态，共享 runtime tick 不会推进或维持该域帧循环。slot、普通 renderer、geometry 和 ghost 使用同一 Host `RuntimeClock`。 |
| M10 | virtual motion 注册和 renderer 均复用 `collection_item_key`，稳定路径为 `item:{data_key}`，不再使用 index。 |
| M11 | property/timeline 在真实 GPUI render frame 按 presentation domain 采样，并由 `Window::request_animation_frame` 继续；terminal event 在 `on_next_frame` 的已提交边界后分域交付。每个 callback 独立事务，失败不丢邻居；stale generation/incarnation 事件被丢弃。 |
| M12 | Canvas rect/circle/path 使用同一 affine point transform，pointer hit test 使用其逆矩阵。GPUI 无法精确表示的 affine motion + axis-aligned path clip 在计划阶段明确拒绝。 |

## 回归入口

- `cargo test --workspace --all-targets --all-features --locked --offline`
- `cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline`
- `cargo test --manifest-path tests/performance/Cargo.toml --locked --offline`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

本轮没有把普通 GPUI 子树的任意 2D transform、通用 blur、shader 或 3D 伪装成已支持能力；这些边界继续遵循 ADR 0020。
