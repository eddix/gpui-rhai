# 第五轮图表与生命周期验收整改说明

本轮按 `acceptance-30c29929/report.zh-CN.md` 的 V01–V03 整改。原报告、探针和证据保持不变，三个新增性质均已迁入产品回归测试。

## V01：补偿式原生生命周期事务

原生 primitive hook 现在是脚本状态切换前的 prepare 阶段：

- Registry 即使遇到中间实例失败，也继续通知所有后续实例并保留首个错误。
- native suspend 失败时，对所有实例执行 resume 补偿；脚本 lifecycle 未切换，公开状态保持 Active，可真实重试。
- native resume 失败时，对所有实例执行 suspend 补偿；脚本 lifecycle 仍为 Suspended，公开状态保持 Suspended，可真实重试。
- native resume 成功但后续脚本 resume 事务失败时，同样重新 suspend 原生实例。

故障注入永久测试覆盖中间实例 suspend/resume panic：失败后的三个实例状态一致，第二次调用会完成真实转换，不会返回伪 `Ok(false)`。

## V02：每个 axis ID 单次编译

Cartesian 场景先为每个 `(region, axis ID)` 收集唯一贡献集合并编译一次 `ChartScale` / `ChartAxisDomain`。保型的空 schema 参与 Auto 类型推断；series group、ticks、annotation 和 custom context 全部引用缓存计划。

空 String X schema 与非空 numeric X 数据共享 Auto 轴时，公开轴统一为 Category；数据点与 `mark_point(x=10)` 均落在 `x=461.5`。Category annotation 的 numeric literal 使用其稳定显示文本匹配类别，不会另走 numeric mapper。

## V03：聚合动画冻结

Chart suspend 保存 runtime-clock 暂停时刻；resume 将挂起时长加到 `transition_started`，因此恢复第一帧保持冻结 sample。若数据 revision 未变化，resume 不重新 prepare/layout 或重启动画；若挂起期间数据变化，则从冻结 sample 过渡到最新 target。

ManualRuntimeClock 前进 60 秒的原生测试确认：恢复后旧位置仍可命中，动画不会追赶 suspended wall time。

## 验证

- 第五轮审计 public 探针：通过。
- 第五轮审计 native 探针：3/3 通过。
- `cargo test --workspace --all-targets --all-features --locked --offline`：561 项通过。
- 原生独立包：87 项通过，其中图表 16 项、生命周期故障注入 2 项。
- Workspace 与原生独立包严格 Clippy、fmt：通过。
- 100k debug 基线冒烟：prepare 926676 µs、首帧 428330 µs、resize 184761 µs、streaming 342632 µs，stream 后 Rhai operations 为 0。单样本只用于冒烟。
- 完整 release smoke：通过。
