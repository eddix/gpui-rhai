# 第四轮图表验收复现

图表修复提交：`7d66a14d`；审计 HEAD：`243b0098`。二者之间仅有 Gallery 背景归属调整。
在仓库根目录执行，依赖须已缓存；原生用例使用 macOS GPUI 0.2.2 test-support：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-7d66a14d/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-7d66a14d/reproduce.py native
```

脚本创建独立临时 Cargo 包，从当前 workspace lock 副本开始，以 offline 模式复用构建缓存，
不修改产品或工作区清单。默认写入临时输出目录，可用 `--out-dir /absolute/path` 指定。
请顺序运行。

- public：执行前几轮特征探针及 `CROSSED_PRIMARY_AXES`。退出 0 表示程序执行完成，不代表全部结果正确。
  交叉轴用例的预期 annotation center 为 `(325,185)`，当前为 `(54.73,185)`。
  旧导出探针会在指定输出目录生成产品 SVG/PNG；本轮报告只保存文本日志和代码。
- native：新增两个断言均失败，Cargo 返回 101。一个测挂起期间是否继续为流数据做布局；
  一个对照无活动手势/有活动手势两种 suspend-resume，再验证新的普通滚轮提交。
- Host 严格按公开文档在 Suspended 时不调用 `view.element()`，而以空元素替换；
  计数不是由 Host 继续绘制已挂起视图造成。
- 两个原生用例使用 MotionPreference::None，排除动画中间帧和动画调度干扰。
  suspended stream 用例在正式 suspend 返回、executor 空闲之后才发送三个新 revision，
  不将挂起前已在途的一次计算误当成新订阅工作。

上一轮原生 9 项可原样复验，在当前 HEAD **9/9 通过**：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-a8d2ef0f/reproduce.py native
```

常规验证：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

结果：workspace 559；native 82（chart 13）；performance 4 通过/3 ignored；fmt/Clippy 通过。
100k v2 debug 单样本通过，包含目标 revision 断言，不是 release 性能认证。
