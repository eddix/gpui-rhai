# 第五轮验收复现

基线：`30c29929 fix(charts): complete axis and lifecycle contracts`。
在仓库根目录执行；依赖须已缓存，原生用例使用 macOS GPUI 0.2.2 test-support：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-30c29929/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-30c29929/reproduce.py native
```

脚本创建独立临时 Cargo 包，以当前 workspace lock 副本和 offline 模式复用缓存，不修改产品清单。
可通过 `--out-dir /absolute/path` 指定输出，默认使用新的临时目录。顺序运行。

- public 退出 0 表示特征探针运行完成。`EMPTY_SCHEMA_AXIS_FACT` 是本轮新增：相同 X=10 的数据
  和标注分别位于 x=461.5 与 x=598，公开轴 metadata 为 Category。前轮其他探针继续记录修复效果。
- native 在审计基线 **3 项失败**，Cargo 返回 101。两个是通用 primitive 生命周期故障注入；
  一个是正常 chart motion 的挂起时钟测试。
- 生命周期 hook 签名不返回 Result。探针在第二个 hook 中主动 panic 一次，触发产品已有的
  panic guard；框架会捕获它并返回 Err。日志中的 injected panic 是测试条件，实际缺陷断言是
  随后的视图状态、原生状态及重试行为，不是把故意注入的 panic 本身算成产品缺陷。
- 三个 lifecycle primitive 按顺序记录 active/resume/suspend 状态，提供无失败的前后实例。
  挂起失败允许整体回滚为 Active 或整体进入 quiescent 状态，断言拒绝半活跃状态；恢复失败要求
  保持 Suspended、停止已经恢复的同伴并允许真正重试。
- Motion 测试使用 ManualRuntimeClock。初始动画先完成，再触发有稳定 key 的位置变化，验证
  旧位置可以命中；只在 Suspended 阶段推进 60 秒，恢复后不推进活动时钟。旧位置不再命中而
  终点能命中，证明动画追赶了挂起时间。此用例使用 Normal motion，与前轮 None 测试互补。

第四轮旧探针可原样执行，当前 native 2/2 通过：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-7d66a14d/reproduce.py public
python3 docs/audits/2026-09-23-charts-e09b5b2f/acceptance-7d66a14d/reproduce.py native
```

常规命令：

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

结果：workspace 560；native 84（chart 15）；performance 4 通过/3 ignored；fmt/Clippy 通过。
100k v2 debug 单样本通过，包含 revision 断言。不是 release 性能认证。
旧导出探针会在输出目录生成产品 SVG/PNG；报告目录本轮只保留文本和代码。
