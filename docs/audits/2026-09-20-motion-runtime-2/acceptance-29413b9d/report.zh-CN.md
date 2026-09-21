# 29413b9d：本轮四组整改验收通过

日期：2026-09-21。提交：`29413b9d683d0fba10c92816fe629f495c05ba30`（`fix: finalize motion target and teardown semantics`）。对照上一轮 `cfe09346c19074c5180cc7a723e5bc87a375f881`。

**结论：F01–F04 可以关闭。本次改动及扩展验收范围内，没有发现新的阻断问题。** 这一结论基于独立复跑、正向断言和源码检查，没有直接采用整改记录中的通过声明。

本轮只新增审计材料，未修改产品代码，也不将历史 API 兼容列为问题。

## 四组修复的独立结果

| 项目 | 实际复验结果 | 结论 |
| --- | --- | --- |
| F01 目标绑定 | 旧 NodeId 无样本，新 NodeId 在 250 ms 得到 width=125；headless direct/timeline MotionKey 相同；原生呈现宽度为 **[100, 125, 150]** | 通过 |
| F02 窗口 teardown | 关闭 `w` 后 `w2` ghost 仍 active=1；挂起后释放、同 ID 重建，200 ms 时 width=20、needs_frame=true | 通过 |
| F03 候选预算 | 原来的 ba/ab 顺序都成功；新增 3 个 timeline 的 **6 种排列**全部成功；超预算候选原子拒绝 | 通过 |
| F04 inertia 停止 | 9999 ms 位置 **632.083769**，10000 ms **632.120559**，变化约 **0.0367898**；不再跳到 1000 | 通过 |

原始输出：[上轮行为探针复跑](evidence/previous-probes.log)、[报告原生测试复跑](evidence/previous-native.log)。

## 扩展验收

新增的 [acceptance.rs](acceptance.rs) 使用正确行为断言，失败时退出非零。交付文件已通过 [run-acceptance.sh](run-acceptance.sh) 再次运行，输出见 [delivered-acceptance.log](evidence/delivered-acceptance.log)。

- **目标重绑定：**分别在 idle、paused、completed 状态下替换子节点，保持原 playback state 与当前样本，清除旧 target；随后再进行 generation migration 并再次换节点类型，新 handle 正确取得权限，旧 handle 失效。
- **预算的排列不变性：**穷举 3 个 timeline 的 6 种声明顺序，检查合法切换的最终 active=1。再提交 active=3、budget=1 的非法候选，断言原 handle、snapshot、event 队列不受污染；随后有效计划仍可接纳。
- **临时限额隔离：**非法候选失败后，直接启动额外 Host motion 仍被预算拒绝，确认候选中的临时 `usize::MAX` 没有泄漏为真实 runtime 配置。
- **inertia 参数边界：**56 组参数组合，覆盖 7 种 friction、正负 velocity、有无边界、bounce=0/0.75；检查 cap 前后连续性、cap 后冻结、有限值、边界约束，以及 direct/timeline 的采样一致性。
- **重复 teardown：**连续三次 suspend→dispose→release→同 ID reopen，确认旧源和事件清空，新实例正常推进。

这些检查验证的是行为性质，并未仅复述新分支的实现步骤。

## 实现判断

**F01：绑定与播放状态分离正确。** [motion.rs:1009](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1009) 为 mounted/headless 都生成 resolved target；兼容分支在保留 playback state 的同时安装新 tracks。子节点身份变化不再被“父 timeline 兼容”掩盖。更新 duration 的行为与已编译 tracks 保持一致。

**F02：取消与销毁职责明确。**（[context.rs:377](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/context.rs:377)、[motion.rs:1613](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:1613)） window release 使用带 `/` 边界的 domain 包含关系，并通过 `release_node_scope` 清理暂停状态和终止事件；普通 cancel 的语义无需因永久销毁而改变。本轮邻居窗口和重复 ID 的运行结果支持该实现。

**F03：完整候选接纳有效。**（[motion.rs:2441](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2441)、[motion.rs:2529](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2529)） candidate 是事务私有副本；安装期间临时解除其中的 active 限额，最终按 active+geometry reservations 校验，恢复真实限额后才整体提交。中途错误与最终超限均不会发布 candidate。这个方案能消除声明遍历顺序造成的误拒绝；扩展回滚断言也通过。

**F04：安全上限与几何终点分离正确。**（[motion.rs:2232](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/motion.rs:2232)） 无显式 snap_points 时保存停止时刻的实际解析位置。保护时长结束不再被等同于达到渐近终点，且保持了此前消除历史积分的性能收益。

## 基线检查

| 检查 | 独立运行结果 |
| --- | --- |
| Workspace，all-targets/all-features，locked/offline | **503 通过、0 失败**，其中核心库 **394 项** |
| 产品 native-keyboard | **59/59 通过** |
| 报告原生回归 | **2/2 通过** |
| performance 默认测试 | **2 通过、2 ignored** |
| fmt | 通过 |
| 严格 all-features Clippy | 通过 |
| 扩展正向断言程序 | 全部通过 |
| exact-commit GitHub Actions | 查询返回 `[]`；未找到该 SHA 的远端运行记录 |

证据位于 [evidence](evidence/)，构建、测试、commit、环境与文件摘要保留在 metadata 和 SHA256SUMS 中。独立报告测试与产品套件有重叠覆盖，不把它们简单相加来扩大测试数量。

## 性能复核

沿用上一轮原样采样基准，release + thin LTO，10 次预热、100 次测量；先构建，待其他检查完成后单独运行。Apple M1、macOS 26.6.2、Rust 1.94.0，1000 个无边界 inertia：

| 动画年龄 | inertia p50 | inertia p95 |
| --- | --- | --- |
| 1 s | 0.649 ms | 0.689 ms |
| 8 s | 0.644 ms | 0.759 ms |

历史积分被消除后的年龄稳定性保持。测量只包含 `tick + snapshot`，原始结果见 [bench-release.log](evidence/bench-release.log)，不包含布局、GPU 和完整应用帧率。

## 验收边界

本结论关闭的是 F01–F04 及其上述相邻场景，没有声称整个项目不存在任何缺陷。物理屏幕视觉质量、真实 display-link/120 Hz 行为、其他操作系统仍不由本地 GPUI test platform 的绿色结果替代；本轮也没有执行 performance 中的两项 ignored benchmark。这些边界不影响本轮四项修复通过。

当前可以结束这一轮修复循环。后续变更宜保留此次的排列不变性、失败回滚、旧 target 清除及重复 teardown 断言，作为持续迭代的回归约束。
