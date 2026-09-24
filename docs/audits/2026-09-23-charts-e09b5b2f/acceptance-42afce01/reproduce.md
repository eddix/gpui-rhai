# 复现与证据

从仓库根目录运行：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-42afce01/reproduce.py previous-native --out-dir /tmp/chart-42afce01-previous
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-42afce01/reproduce.py native --out-dir /tmp/chart-42afce01-current
```

在 `42afce01` 上：previous-native 为 **12/12、exit 0**；native 为 **15 passed、3 failed、exit 101**。

Runner 使用临时独立 Cargo 包、固定 GPUI 0.2.2/test-support、charts-enabled 产品 path dependency，复用原生测试 target；不修改产品与 workspace manifest。新附件 `include!` 复用上一轮的不可变对照及辅助 fixture，两个依赖附件 SHA256 记录在 metadata。

三个失败测试：

- `linked_target_can_zoom_in_from_its_displayed_window`：R01，联动目标放大 preview 与最终逻辑窗口。
- `changing_link_group_does_not_alias_same_numeric_version`：R02，两个组都 version 1 时直接换组。
- `resize_keeps_an_active_unlinked_preview`：R03，Started→Host 改组件宽度→Ended。

三个新增正向控制：

- `compensation_commit_failure_disposes_chart_resources`：补偿 commit 失败后处置、卸载、不再 layout。
- `geo_camera_pan_uses_plot_size_before_and_after_target_resize`：异尺寸 Geo camera pan 和目标再次 resize 的真实命中。
- `source_unmount_releases_its_link_projection`：源卸载撤回组投影。

探针只使用公开组件、Host extension、AX 几何和 GPUI 原生输入 API。Custom series 读取框架提供的真实 mapper 或给出纯几何标记，不修改产品内部状态。生命周期日志中的 injected failure 是受控故障输入；判断依据是其后的 View state、资源卸载和布局计数。

本轮正式检查命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --all-targets --locked --offline -- -D warnings
python3 scripts/verify-target-manifest.py
bash scripts/release-smoke.sh
bash scripts/audit-release-artifacts.sh
```

性能命令运行两次（第二次用于核对首轮较高的 p95，两个日志都保留）：

```sh
GPUI_RHAI_BENCH_SAMPLES=30 GPUI_RHAI_BENCH_WARMUP=5 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --release --manifest-path tests/performance/Cargo.toml --locked --offline --test e2e chart_end_to_end_baseline -- --ignored --nocapture --test-threads=1
```

全部附件为代码、文字或 JSON；本轮未添加图片。报告只引用最终正确构造 fixture 后的结果，不将探针开发过程的编译或语法错误计作产品缺陷。
