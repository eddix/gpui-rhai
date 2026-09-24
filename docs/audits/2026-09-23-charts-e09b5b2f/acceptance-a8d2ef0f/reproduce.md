# 第三轮图表验收复现

基线：`a8d2ef0f8b90df4915d3150941ee49baa9dd23bd`。
在仓库根目录执行，依赖须已缓存；原生测试使用 macOS GPUI 0.2.2 test-support：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-a8d2ef0f/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-a8d2ef0f/reproduce.py native
```

脚本创建独立临时 Cargo 包，复制当前 workspace lock，以 offline 模式复用构建缓存，
不改产品或工作区清单。可用 `--out-dir /absolute/path` 保存输出；默认新建临时输出目录。
请顺序执行，避免构建锁和同名测试二进制干扰。

- `public`：特征探针，退出 0 表示程序完成，不表示行为全部正确。复验前两轮输入，并增加
  CATEGORY_THRESHOLD、ANNOTATION_RENAME、EMPTY_AXIS_GROUP、PAN_DIRECTION、EXPORT_SELECTED_LEGEND_ELEMENT。
  旧导出探针还会在输出目录生成产品 SVG/PNG；本轮报告目录只保留文本证据与代码。
- `native`：9 项原生测试，基线 **6 通过、3 失败**，Cargo 返回 101。
  正式 BarChart 拒绝 viewport_revision；旧确认丢弃新手势；明确 Started 的手势在 Ended 前已提交。
- 联动探针保留上一轮的 64 次 render / renderer 保护，当前不触发；启用和关闭 link 后
  layout count 都稳定为 2。数据来源卸载后，目标图的 linked selection 也正常撤销。
- 前轮缩放 fixture 已按有意变更后的新协议回写 viewport_revision；旧 mark key 的查找
  改为按 role/series/datum 定位。这些必要的探针适配不作为 API 兼容性缺陷。
- 延迟确认测试用一个公开 Rhai 状态动作确定性地模拟 Host 晚到的确认，避免墙钟竞态。
  顺序是 proposal 1 → 新手势 Started → 确认 proposal 1 → 新手势 Ended；期望不能丢掉第二次输入。

既有验证命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

结果：workspace 555；native 80（其中 chart 11）；performance 4 通过/3 ignored；fmt/Clippy 通过。
v2 100k debug 单样本通过，包括 revision 断言，不属于 release 性能认证。
本轮未重跑完整 release smoke、30 样本 release、实体窗口主题/RTL 截图及 VoiceOver。
