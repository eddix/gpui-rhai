# `cfe09346` 复验整改记录

原始报告与证据保持不变。本轮针对 F01–F04 做局部收口，没有撤销此前已经通过的 generation、live/exit、Canvas 和解析采样架构。

| ID | 整改 |
| --- | --- |
| F01 | 兼容 timeline 保留 playback position，但每次 reconcile 都安装新编译的 resolved tracks。Mounted target 重新绑定到最新 retained NodeId；headless track 总是消费与 direct declaration 相同的编码 target。旧目标样本立即消失。 |
| F02 | 新增 presentation teardown 语义：普通 cancel 可保留暂停墓碑，window release 会同时清除 motion、timeline event、预算预留和 suspended scope。ghost 过滤使用带 `/` 边界的 domain 包含关系，`w` 不再命中 `w2`。 |
| F03 | reconcile candidate 在私有副本内完成全部 timeline/direct 替换后，再按最终 `active + geometry reservations` 一次验收 Host budget；中间安装顺序不再参与接纳结果。 |
| F04 | inertia 达到 10 秒安全上限时保留该时刻解析样本并停止速度；只有显式 `snap_points` 才吸附。无 snap 的 9999→10000 ms 位移约 0.037，不再跳向渐近终点。 |

## 正向复跑

- retained target replacement：旧 NodeId 无样本，新 NodeId 在 250 ms 得到 width=125。
- headless direct/timeline：MotionKey 完全一致。
- timeline declaration 顺序 `ba` / `ab`：均成功，最终 active=1。
- suspend→release→同 ID reopen：200 ms width=20、`needs_frame=true`。
- release `w`：`w2` exit ghost 保持 active=1。
- 原生帧：target replacement 的 0/250/500 ms 实际宽度为 `[100, 125, 150]`。

物理屏幕视觉质量和受控 120 Hz display-link 测量仍是发布前人工认证范围。

修复后复跑摘录见 [行为探针](resolution-evidence/probes.log) 与 [原生帧测试](resolution-evidence/native.log)。
