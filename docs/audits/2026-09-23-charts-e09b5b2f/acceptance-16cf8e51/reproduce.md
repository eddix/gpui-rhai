# 16cf8e51 图表验收复现

在仓库根目录执行，要求本机已有依赖缓存；原生测试使用 macOS GPUI test-support：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-16cf8e51/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-16cf8e51/reproduce.py native
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-16cf8e51/reproduce.py linked
```

可用 `--out-dir /absolute/path` 指定输出，否则写到新的临时目录。顺序运行。
脚本创建独立临时 Cargo 包，复制当前 workspace lock，并使用 offline；不修改工作区清单或实现。

- `public`：打印旧问题复验和新增组合用例；退出 0 表示探针执行完成，不表示全部行为正确。SVG/PNG 同时写入输出目录。
- `native`：4 个真实窗口断言，在审计基线均失败。三个检查有效滚轮变化/跨帧手势/无 phase 滚轮，一个检查 Gauge 的业务 key。
- `linked`：先挂载非联动对照，再挂载两个共享 group/domain 的图表。非联动图布局保持 2 次；联动图无法空闲，探针在 **64 次宿主 render** 触发保护，测试进程退出 70，本机 Cargo/复现脚本也返回 70。这是审计程序的保护，不是产品 panic。另有 renderer 调用计数保护，但本轮实际命中的是宿主 render 保护。请不要删除保护后在主工作进程中运行这个已知循环。

旧原生 5 项探针仍可运行：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/reproduce.py native
```

它们在当前基线 5/5 通过，但原缩放用例没有断言提案必须包含非零输入，只比较 first/last，因此 `1.0 == 1.0` 也通过；新增 native 用例修正这个盲点。

`public-probes.rs` 复用原始输入，仅为新公共 API 补上 ChartMark role/region/datum 字段，并更新生成 key/label 的定位方式。没有把 API breaking change 当成缺陷。新增用例由 `extra_findings` 和 `auto_category_locale` 标识。

常规检查结果与命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

分别为 workspace 548 通过、native 70 通过、performance 4 通过/3 ignored、fmt/Clippy 通过。
最后一条 v2 benchmark 的 debug 单样本通过目标 revision 断言；不是 release 性能认证。
本轮没有运行 30 样本 release、实体屏幕全主题/RTL 或 VoiceOver 验收。

审计期间另一会话开始修改 Tabs/Button/Badge 等 UI；这些未提交改动不属于图表验收范围。
结束前已确认 chart 目录和 chart registry 的产品代码仍与 `16cf8e51` 一致。
