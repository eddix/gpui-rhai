# 29413b9d 验收复现

在仓库根目录运行。当前基线的以下命令均应成功退出。

## 上轮四项行为与原生回归

```bash
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-20-motion-runtime-2/acceptance-cfe09346/probes.rs"
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-cfe09346/run-native.sh
```

第一条是历史 characterization 程序，应查看实际输出，退出 0 本身不是正确性证明。第二条包含真正的 GPUI 正确行为断言，当前应为 2/2 通过，宽度 `[100, 125, 150]`。

## 本轮扩展正向断言

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-29413b9d/run-acceptance.sh
```

检查 idle/paused/completed rebind + reload、6 种预算排列、超限回滚和真实限额保留、56 组 inertia 参数，以及三次暂停后销毁重建。任一错误会使进程失败。

本目录 runner 使用独立 Cargo 包名，避免与历史 probe 在共享 target 下的同名可执行文件串扰。历史 `run-probes.sh` 的不同输入应串行运行。

## 性能

```bash
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-29413b9d/run-bench.sh
```

该脚本使用上一轮未修改的 `bench.rs`，release + thin LTO。测量时避免同时执行其他编译或负载测试。

## 产品检查

```bash
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

performance 默认命令仍有 2 项 ignored；本轮未将它们记为已执行。
