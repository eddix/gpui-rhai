# `23874744` 复验整改记录

本文件记录第二轮审查后的产品整改。原始报告、探针与证据保持原样；行为探针已在修复后的工作树复跑，N01–N07 输出均回到验收期望，Canvas 原生正确行为测试也已通过。

| ID | 整改结果 |
| --- | --- |
| N01 | lifecycle reconcile 显式接收 candidate generation；`reload` / `resume_reload` 在同一事务中迁移 handle 权限和 callback generation，失败候选恢复旧 handle 与进度。 |
| N02 | 普通 resume 与 resume_reload 共用 `resume_runtime_mechanisms`；timer、MotionRuntime scope、layout/trigger geometry clock 一起恢复。覆盖失败仍暂停、保留旧源、新增源和 geometry 进度。 |
| N03 | direct source 带显式 `Host` / `Live(domain)` / `Exit` 保留所有权。live reconcile 只回收本 live scene，不再删除 exit scene；view/window teardown 仍统一取消。 |
| N04 | mounted scene 使用 `presentation domain + retained NodeId` 作为节点动效身份；timeline 人类 target 在 plan 阶段解析到目标 NodeId。无 retained tree 的 Rust helper 使用可逆十六进制分段编码，并以 root key/kind remount signature 分配新实例。 |
| N05 | Canvas 一帧的 morph、trim、stroke、clip 与 affine 后路径由 paint/hit 共用。命中在呈现坐标中计算，stroke 保持与 GPUI paint 一致的非缩放像素宽度。原生鼠标测试覆盖 morph 终态新形状可点、旧形状不可点。 |
| N06 | policy 切换会重投影 settled declarations；None 的有限 timeline 终态复用完整 timeline time mapping，包含外层 repeat/autoreverse，并且策略切换不重复发送完成事件。 |
| N07 | reconcile 先提交 timeline playback 替换，再接纳 direct sources；active→idle、减少轨道和 direct 替换按最终计划预算计费，不再受旧 active timeline 的遍历顺序影响。 |
| N08 | render sampling 立即冻结该帧 domain event batch；post-frame closure 只交付捕获的 batch，之后输入产生的 event 留给下一次真正 render。terminal batch 会主动请求 post-commit frame。 |
| N09 | 无约束及无反弹 inertia 使用解析指数解；有反弹约束改为边界碰撞分段解析，不再按动画年龄重复 240 Hz 历史积分。1000 源 release p50：1 秒约 0.62 ms、8 秒约 0.62 ms。 |

## 新增交接回归

- 成功/失败 `reload` 的 generation、handle 和连续进度。
- 成功/失败 `resume_reload` 的 direct、new source、layout 和 trigger 时钟。
- exit 期间无关 rerender。
- retained root remount、slash/数字/`item:` 字面 key、retained timeline child target。
- None 的 settled direct 与 timeline 外层 autoreverse。
- active timeline → idle timeline + direct source 的预算替换。
- frame event batch 冻结。
- Canvas morph/trim/non-uniform affine stroke 命中。
- inertia 解析解及 release 年龄稳定性。

物理屏幕视觉质量与受控 120 Hz display-link 测量仍属于发布前人工认证，不由 GPUI test platform 的绿色结果替代。

修复后复跑摘录见 [lifecycle/API 探针](resolution-evidence/probes.log)、[Canvas 原生测试](resolution-evidence/native-canvas.log) 与 [release 微基准](resolution-evidence/bench-release.log)。
