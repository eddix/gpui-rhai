# 核心模型审查整改说明

本轮按 `core-review-a31da5a8/report.zh-CN.md` 的三个结构工作包整改，不以逐条条件分支作为完成标准。原报告、探针和证据保持不变，核心性质已迁入正式测试。

## A：呈现帧提交

ChartController 现在明确维护：

- `ChartDataKey { source_epoch, revision }`：区分数据源身份、spec/transform 代际和数据 revision；
- `ChartFrameKey { data, frame_epoch }`：对 viewport、bounds、theme、selection、link 等布局输入统一失效；
- `prepared_key`：只说明不可变中间数据已经完成；
- `presented_key`：只有匹配当前请求与活动 epoch 的候选在前台成功安装后才推进。

后台 job generation 只决定晚到任务能否安装，内容 key 决定结果能否复用。Suspend 取消 layout、preview 回滚、resize/theme/selection/link 更新都通过同一个 FrameKey 入口恢复缺失工作。Paint、hit testing、accessibility 和语义事件继续读取一个完整 presented scene；accessibility 同时公开 data revision、source epoch 与 frame epoch。

性质测试覆盖：prepared revision 已完成但 candidate 未安装时 suspend/resume；同数据 preview scene 被取消后恢复 committed frame；NativeChartData source identity/revision 与 frame epoch 不混用。

## B：生命周期与活动时间

原生生命周期分为 `resume` prepare 与 `commit_resume`：只有所有 primitive prepare、脚本 resume 事务及 native commit 全部成功，公开 view 才进入 Active。Chart 在 prepare 阶段不启动 listener、候选工作或动画时间；commit 后才偏移暂停区间并恢复活动。

可补偿的单故障继续保留 Active/Suspended 并允许真实重试。若补偿本身失败，则采用报告允许的终止策略：立即 dispose 整个 view、卸载所有 retained primitive 并释放 runtime/window 资源。没有新增公开枚举变体，也不会把混合资源状态伪装成普通 Suspended。

性质测试覆盖：单次 suspend/resume 故障、补偿双故障、失败 prepare 推进 ManualRuntimeClock 60 秒、普通冻结恢复，以及 dispose 后拒绝再次恢复。

## C：typed viewport 与 link domain

`ChartLinkedViewport` 已改为坐标类型枚举：

- Cartesian：携带 region、axis ID 和逻辑 visible domain；目标只查询相同绑定的已编译轴计划。
- Geo：携带 region、map/projection identity、zoom 和按 plot 归一化的 camera pan；不同尺寸目标重新投影为本地像素。
- Polar/不兼容绑定：明确不消费，不再从第一根 Cartesian axis metadata 猜测。

Host viewport acknowledgement 只接受显式 `viewport_revision`；已删除依赖 revision 数值猜测的 legacy 双协议。

## 验证

- 核心审查原生探针：8/8 通过。
- `cargo test --workspace --all-targets --all-features --locked --offline`：561 项通过。
- 原生独立包：92 项通过，其中图表 20 项、生命周期故障注入 3 项。
- Workspace 与原生独立包严格 Clippy、fmt：通过。
- 100k debug 基线冒烟：prepare 889663 µs、首帧 427808 µs、resize 182176 µs、streaming 338653 µs，stream 后 Rhai operations 为 0。单样本只用于冒烟。
- 完整 release smoke：通过。
