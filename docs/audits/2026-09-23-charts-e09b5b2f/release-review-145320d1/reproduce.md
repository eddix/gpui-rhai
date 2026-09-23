# 复现说明

在仓库根目录运行，要求 pinned Rust/GPUI 依赖和 macOS 原生测试环境：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/release-review-145320d1/reproduce.py native --out-dir /tmp/gpui-rhai-release-review-repeat
```

Runner 创建临时独立 Cargo 包，固定 GPUI 0.2.2/test-support，以 path 引用当前产品；复用 `tests/native-keyboard/target`，离线执行，不修改 workspace manifest 或产品源码。`GPUI_RHAI_REVIEW_REPO` 由 runner 设置。`native-probes.rs` 使用 `include!` 复用上一轮未改动的 8 个对照及公共 fixture；其 SHA256 记录于 metadata。

在 `145320d1` 上预期结果：**8 passed、4 failed、退出码 101**。这是实际行为断言失败，不是编译失败：

- `failed_suspend_restores_chart_streaming_in_active_view`：R01 原生 suspend 失败入口。
- `failed_script_suspend_restores_chart_streaming_in_active_view`：R01 Rhai suspend 抛错入口。
- `acknowledged_geo_link_survives_suspend_resume`：R02 已确认联动的恢复。
- `linked_cartesian_windows_align_both_axes_with_different_full_domains`：R03 实际 X/Y mapper 窗口。

测试只调用公开产品 API、真实 Rhai Chart 组件和固定 GPUI 的输入／executor API；不读写 ChartEntity 私有字段。自定义 series 为纯 Rust 观察器／几何 fixture。生命周期日志中的 `injected ... failure` 是故障注入，产品会捕获；报告依据是后续公开状态／revision／命中断言，不把注入的 panic 本身当产品缺陷。

正式套件与发布检查命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo check -p gpui-rhai --all-targets --locked --offline
python3 scripts/verify-target-manifest.py
cargo doc --workspace --all-features --no-deps --locked --offline
bash scripts/release-smoke.sh
bash scripts/audit-release-artifacts.sh
bash scripts/audit-visual-baselines.sh
cargo package -p gpui-rhai --list --allow-dirty --locked --offline
cargo package -p gpui-rhai-registry --list --allow-dirty --locked --offline
cargo package -p gpui-rhai-cli --list --allow-dirty --locked --offline
```

`--allow-dirty` 用于只列出已有 UI 文档清理工作区的 package inventory，不执行发布。完整 smoke 会短暂启动原生示例并终止该次运行创建的进程。

Release 性能复现（30 次测量、5 次预热）：

```sh
GPUI_RHAI_BENCH_SAMPLES=30 GPUI_RHAI_BENCH_WARMUP=5 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --release --manifest-path tests/performance/Cargo.toml --locked --offline --test e2e chart_end_to_end_baseline -- --ignored --nocapture --test-threads=1
```

最终证据均为文本。已存在的视觉 baseline 仅做只读完整性检查；本轮没有生成或添加图片。
