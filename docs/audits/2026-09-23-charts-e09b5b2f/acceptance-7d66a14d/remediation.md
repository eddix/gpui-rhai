# 第四轮图表验收整改说明

本轮按 `acceptance-7d66a14d/report.zh-CN.md` 的 U01–U02 整改。原报告、探针和证据保持不变，两个新增复现均已迁入产品回归测试。

## U01：独立轴计划

Cartesian 的单轴编译已抽为共享入口：每个 axis ID 独立计算类型、完整域、visible 域、方向、range 和 `ChartScale`。series group、轴绘制和 annotation 都消费该入口。

未限定 axis ID 的 annotation 仍按文档选择每个 channel 的首个已声明有效轴，但 X/Y 不再要求被同一个 series 同时引用。交叉主轴的 `mark_point(5,5)` 现在严格落在 plot center `(325,185)`；mark point、line、area、baseline 和 threshold band 共用同一对独立 scale。

## U02：retained primitive 生命周期

`PrimitiveHandler` 新增默认 no-op 的 `suspend` / `resume` hook，`PrimitiveRegistry` 在 `ScriptViewHandle` 生命周期边界通知所有已挂载 retained primitive。脚本 suspend 成功后才 quiesce 原生实例；脚本 resume 事务成功后才恢复原生实例。

Chart 在 suspend 时：

- 取消数据 listener、prepare/layout/timer task，并推进 job generation 拒绝晚到候选；
- 清理 wheel、pan、brush、hover 与未确认 preview/proposal；
- 回到 Host 已提交的 zoom/pan，但保留 scene、配置、选择和实体身份。

Chart 在 resume 时先重新建立 NativeChartData listener，再读取最新 snapshot 并只准备一次。因此挂起期间的更新 burst 不逐个布局，也不会在监听与 snapshot 之间丢 revision。被 suspend 截断的 Started 手势不会污染恢复后的 phase-less 鼠标滚轮。

## 验证

- 第四轮审计 public 探针：交叉主轴标注精确落在 `(325,185)`。
- 第四轮审计 native 探针：2/2 通过。
- `cargo test --workspace --all-targets --all-features --locked --offline`：560 项通过。
- 原生独立包：84 项通过，其中图表 15 项。
- fmt 与严格 Clippy：通过。
- 100k debug 基线冒烟：prepare 902311 µs、首帧 433515 µs、resize 187064 µs、streaming 340801 µs，stream 后 Rhai operations 为 0。单样本只用于冒烟。
- 完整 release smoke：通过。
