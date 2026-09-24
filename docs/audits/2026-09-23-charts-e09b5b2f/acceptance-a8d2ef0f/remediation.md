# 第三轮图表验收整改说明

本轮按 `acceptance-a8d2ef0f/report.zh-CN.md` 的 T01–T06 整改。原审计报告、探针和证据保持不变，关键性质已迁入产品测试。

## 已关闭

- T01：自动 LTTB 根据 X 列类型选择采样坐标。String/Bool 类别使用稳定行序作为临时几何坐标，输出仍保留原始类别值、key 与 null gap。Line/Area 在 3999、4000、4001 和 10000 行均有回归覆盖。
- T02：viewport proposal 保存 revision、输入 generation、zoom 与 pan 快照。确认旧 proposal 时，仅在 acknowledged base 上重放其后的 preview delta；显式 Started/Moved/Ended 手势不再使用 80ms 静默提交，只有无可靠结束阶段的输入使用 timer。
- T03：正式 Bar/Line/Pie/Map source adapter 的严格 schema 均声明并转发 `viewport_revision`，真实 mount 测试覆盖四个入口。
- T04：Cartesian 先过滤空轴组，再按轴声明顺序选择有有效数据的主 X/Y 轴。annotation 绑定该确定性轴对，不再由 BTreeMap key 排序、series 顺序或空组消费决定。
- T05：viewport domain 与 linked viewport 均携带 `ChartAxisDirection`；normal/reversed X/Y 的正向屏幕 pan 使用一致方向。
- T06：native 与 export 共用 `apply_chart_selection`，只匹配 Data role 的 `DatumRef`。业务键 `legend` 不再选中图例控件。

## Component Gallery 滚动背景

真实 release 窗口复现确认，Gallery 的滚动内容节点同时绘制整页 surface，导致该背景随内容 offset 移动。现在滚动内容保持透明，固定 scroll viewport 持有 surface 背景；标题栏、类别栏、内容背景和状态栏的所有权分离。已在真实窗口滚动前后目视复验。

## 验证

- 第三轮审计 public 探针：全部预期结果通过。
- 第三轮审计 native 探针：9/9 通过。
- `cargo test --workspace --all-targets --all-features --locked --offline`：559 项通过。
- 原生独立包：82 项通过，其中图表 13 项。
- fmt 与严格 Clippy：通过。
- 100k debug 基线冒烟：prepare 891434 µs、首帧 425705 µs、resize 180266 µs、streaming 341190 µs，stream 后 Rhai operations 为 0。单样本只用于冒烟。
- 完整 release smoke：通过，包含 Chart Gallery、Component Gallery 和 Theme Studio。
