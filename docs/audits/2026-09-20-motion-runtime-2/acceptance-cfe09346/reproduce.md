# cfe09346 复验入口

从仓库根目录执行。基线为 `cfe09346c19074c5180cc7a723e5bc87a375f881`，依赖版本由仓库锁文件确定。程序放在临时 Cargo consumer 内，不修改产品源码。

## 本轮行为探针

```bash
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-20-motion-runtime-2/acceptance-cfe09346/probes.rs"
```

这是 characterization 程序，退出 0 表示运行完毕，不表示产品正确。当前基线应打印：

- target replacement：旧 NodeId 卸载，但 old_sample=125，new_sample=None。
- headless direct/timeline keys_equal=false。
- budget 顺序 ba 失败，ab 成功。
- inertia 9999→10000 ms 跳跃约 367.916。
- suspended window release/reopen：width=0、needs_frame=false。
- release w：w2 ghost 从 active=1 变为 0。

## 原生正确行为测试

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-cfe09346/run-native.sh
```

当前基线应有 **1 通过、1 失败，退出 101**。Canvas morph 的原始回归应通过；`timeline_rebinds_a_replaced_child_in_presented_frames` 因 width 持续为 100 而失败。它真实挂载并执行多次 GPUI frame，使用 ManualRuntimeClock 决定时间。测试中的 Rhai action 已使用合法的 `audit.*` ID。

## 上轮探针复跑

```bash
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/probes.rs"
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/run-native.sh
```

不要把行为探针的旧 expected 文本当成当前的断言测试，阅读实际输出并对照报告。上轮 Canvas 测试是正确行为断言，当前应通过。

## Release 成本

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/run-bench.sh
```

本轮使用完全相同的 benchmark 源码和 release + thin LTO 配置；先单独构建，待其他检查结束后再运行。当前 1000 个无边界 inertia 在 1 s/8 s 的 tick+snapshot p50 均约 0.6 ms。不要从这一微基准推断整体应用帧率。

## 产品基线

```bash
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

默认 performance 命令包含 2 项 ignored；本轮没有执行这些被忽略的测试。GPUI test platform 也不能代替真实屏幕的 display-link 提交顺序验证。
