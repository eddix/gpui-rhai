# 复现

仓库根目录执行：

```sh
python3 docs/audits/2026-09-24-ui-theme-e5d26877/reproduce.py previous-native --out-dir /tmp/ui-theme-previous
python3 docs/audits/2026-09-24-ui-theme-e5d26877/reproduce.py native --out-dir /tmp/ui-theme-native
python3 docs/audits/2026-09-24-ui-theme-e5d26877/scan-styles.py --out-dir /tmp/ui-theme-source
```

当前基线 previous-native 为 18/18、exit 0；native 为 0 passed/3 failed、exit 101，均为编译成功后的行为断言失败。两个测试验证正式 Tabs 的几何与合法 Host theme overrides，一个验证传入 native handler 的主题快照。临时独立 Cargo 包固定 GPUI 0.2.2/test-support，复用原生 target，产品代码与 manifests 不变。

源扫描只生成候选清单，不能将所有 literal 当成缺陷。对比计算使用当前 bundled themes 的不透明 text_muted/surface_hover，适用于正式 Tabs 的 enabled/unselected 状态，不计算 disabled 文本。

正式检查命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --all-targets --locked --offline -- -D warnings
bash scripts/release-smoke.sh
```

性能基线：

```sh
GPUI_RHAI_BENCH_SAMPLES=30 GPUI_RHAI_BENCH_WARMUP=5 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --release --manifest-path tests/performance/Cargo.toml --locked --offline --test e2e chart_end_to_end_baseline -- --ignored --nocapture --test-threads=1
```

实机使用当前 release `component_gallery` 与 `scripts/build-macos-test-app.sh component_gallery default-light en navigation` 生成的独立临时 bundle。经原生 UI 切换 Default Dark、Gruvbox 和类别，检查主题与状态一致性。没有把第三方参考图或新截图写入仓库；此次视觉结果用文字记录，不能冒充新的 PNG baseline。
