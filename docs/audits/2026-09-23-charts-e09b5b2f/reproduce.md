# 独立复现说明

审计基线：`e09b5b2febe1b20aed3f471ce5b7f841eb2980c6`。
脚本只创建临时独立 Cargo 包，复用本仓库构建缓存，不修改产品或工作区清单。
依赖从当前仓库 `Cargo.lock` 的副本开始解析，并使用 `--offline`。
需要当前仓库依赖已缓存；原生测试使用 macOS 上的 GPUI test-support。

在仓库根目录执行：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/reproduce.py native
python3 docs/audits/2026-09-23-charts-e09b5b2f/reproduce.py bench
```

可用 `--out-dir /absolute/path` 保存日志、SVG 和 PNG；默认输出到新的临时目录。
请顺序运行，以免同一构建目录上的 Cargo 锁影响耗时。

## 如何读结果

- **public**：特征探针，不是全部带期望断言的测试。退出 0 表示探针运行完成，**不代表产品通过验收**。比较 `public.log` 与报告中的预期；现有基线输出见 `evidence/probes.log` / `evidence/public.log`。
- **native**：5 个测试断言期望行为，在审计基线均失败，Cargo 返回 101。这是预期的缺陷复现结果。测试通过正式 Rhai Chart 入口挂载，模拟原生输入，读取公开状态和语义树；不是直接改私有内部状态。
- **bench**：release 数据转换微基准。预先构造 1,000 / 10,000 行，5 次预热、20 次采样，测量行克隆、类型列转换、NativeChartData 构造；分位数采用 nearest-rank。它不测完整绘制、脚本耗时或实际屏幕输入延迟。
- `FILTER_TYPO` 是额外的错误诊断特征记录，不作为报告中独立的发布阻断项。
- 原生测试使用 `MotionPreference::None`，避免动画中间帧影响命中。缩放测试每次输入后显式发出 `TouchPhase::Ended`；刷选、特殊数据键均有同坐标的正常对照组。
- PNG 样例来自 public 探针，SVG 包含标题、轴和图例文字；PNG 缺少这些文字。改变标题会改变 SVG，但基线 PNG 字节完全相同。

## 本轮已有测试

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

最后一条仅是 debug 冒烟跑通：一个样本的 p50/p95 没有统计意义，不能据此宣称满足 10k / 100k 帧预算。既有 native-keyboard 独立包没有启用 `charts`，故其 65 项通过不能作为本轮原生图表验收。

本目录 `.rs` 文件是复现附件，不属于工作区 Cargo 测试目标。修复时应将行为断言迁入产品测试，并将图表原生测试加入明确启用 charts 的 CI 目标。
