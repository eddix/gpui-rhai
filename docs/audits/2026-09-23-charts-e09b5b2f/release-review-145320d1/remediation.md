# 0.1.5 Chart Runtime 组合验收整改说明

本轮按 `report.zh-CN.md` 收敛出的两个工作包整改。原报告、原始证据与
复现程序保持不变；修复后的不变量已迁入正式测试，而不是仅让审计探针
通过。

## A：生命周期补偿闭环

恢复原生活动状态现在只有一条完整路径：所有 primitive 执行 resume
prepare 后，必须继续执行 `commit_resume`。Suspend 任一阶段失败时，反向
补偿也复用这条路径；补偿成功后 View 才能维持 `Active`，Chart 才会重新
启用数据监听、帧提交与活动时间。补偿本身失败时仍采用既定的故障隔离
策略：dispose 整个 View，不把混合状态伪装成可用的 Active/Suspended。

正式测试分别覆盖原生 suspend 失败和 Rhai suspend 抛错，并在补偿后发布
下一数据 revision，要求 `presented_key` 真正推进，而不是只检查公开枚举。

## B：联动视口的状态与坐标闭环

`ChartLinkRegistry` 现在持有带 source 与单调 version 的最新类型化投影，
联动状态不再只是一条即时广播。目标图表挂起时可以暂时停止工作，恢复后
会重新取得同组已确认投影；源实例离组或卸载时才清理其投影。

Cartesian 联动不再把 X/Y 两个逻辑窗口压缩成一个 zoom/pan 标量。每个
`(region, axis)` 的 visible domain 直接注入对应轴计划，因此目标图表即使
拥有不同的完整 X/Y 域，也会分别对齐两个可见窗口。Geo 继续传递带
map/projection identity 的 zoom 与归一化 camera pan，并在目标本地 plot
尺寸下重投影。

Host 控制协议只接受显式 `viewport_revision`；旧的 revision 数值猜测逻辑
已经删除。

## 验证

- release-review 独立原生探针：12/12 通过，含 R01–R03 的四个组合断言。
- Workspace 全目标、全 feature：561 项通过。
- 原生独立包：94 项通过，其中图表 22 项、生命周期故障注入 3 项。
- Workspace 与原生独立包严格 Clippy、fmt：通过。
- 100k release 单样本冒烟：prepare 130.942 ms、首帧 123.988 ms、resize
  28.698 ms、128 行滑动更新 83.133 ms；stream 后 Rhai operations 为 0。
- 完整 `scripts/release-smoke.sh`：通过，包括 Chart Gallery、全 examples、
  Theme Studio 与 Table 状态矩阵。

单样本性能结果仅用于确认没有量级回退；报告已有的 5 次预热、30 次采样
仍是本候选版本的统计基线。实机主题、RTL、缩放、键鼠与 VoiceOver 矩阵
仍按 0.1.5 发布清单执行，自动测试不替代人工体验验收。
