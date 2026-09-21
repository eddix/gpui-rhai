# 复验入口

基线：`23874744ccdf92c887785a13a83716b24a3925c6`。在仓库根目录执行；脚本仅创建临时 Cargo consumer 和构建缓存，不修改产品实现。

## 公共 API 与真实 ScriptLifecycle 调用链

```bash
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/probes.rs"
```

这是行为探针：基础已修行为有正向断言，其余打印 expected/actual，进程退出 0 不等于验收全部通过。尤其应看到 reload lookup error、resume_reload width=20、ghost active=0、remount handle retained、policy terminal mismatch、预算替换被拒绝。后续实施应将这些打印转换成独立的正确行为测试。

## GPUI 原生鼠标命中回归

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/run-native.sh
```

这是正确行为断言；当前基线应退出 101，`morphed_path_hit_follows_presented_endpoint` 失败。它真实 mount/布局/prepaint/paint，并发送 GPUI 鼠标输入；不依赖 automation 直接分派绕过命中路径。

## Release 采样成本

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-23874744/run-bench.sh
```

release + thin LTO，10 次预热、100 次测量。比较同数量 transition/inertia 在 1 秒和 8 秒的 `tick + snapshot` 成本。它测量 Rust 采样和 map 构造，不包含 GPU、布局、Rhai、屏幕刷新，不能直接换算应用帧率。首次 Metal 编译可能需要当前机器默认编译缓存目录的写权限。

## 产品基线检查

```bash
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

`tests/performance` 默认有 2 项 ignored，不能当作已执行的大规模性能认证。GPUI 0.2.2 test window 的 `on_request_frame` 是空实现；原生输入测试能证明布局和输入路径，但不能单独证明真实 display callback/terminal event 时序。
