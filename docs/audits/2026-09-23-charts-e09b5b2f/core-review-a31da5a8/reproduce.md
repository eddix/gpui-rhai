# 核心模型审查复现

基线：`a31da5a8 fix(charts): unify lifecycle axis and motion state`。
在仓库根目录执行；依赖须已缓存，原生用例使用 macOS GPUI 0.2.2 test-support：

```sh
python3 docs/audits/2026-09-23-charts-e09b5b2f/core-review-a31da5a8/reproduce.py native
python3 docs/audits/2026-09-23-charts-e09b5b2f/core-review-a31da5a8/reproduce.py previous-native
python3 docs/audits/2026-09-23-charts-e09b5b2f/core-review-a31da5a8/reproduce.py previous-public
```

脚本建立独立临时 Cargo 包，复制 workspace lock，以 offline 模式复用缓存，不改产品清单。
默认输出到新临时目录，可传 `--out-dir /absolute/path`。请顺序运行。

## 结果与保护条件

- 本轮 native 共 8 项：**3 通过、5 失败**，Cargo 返回 101；三个通过项是第五轮原始故障注入及普通动画冻结对照。
- previous-native：第五轮原始附件 **3/3 通过**。
- previous-public：程序执行成功，原空 schema/交叉轴坐标均一致；exit 0 不是全面正确性的断言。
  旧 public 探针会在输出目录生成产品 SVG/PNG，本轮报告不保存图片。
- `resume_finishes_a_prepared_but_unpresented_revision` 使用固定版本 GPUI 的公开测试
  `BackgroundExecutor::tick()` 一次推进一个 runnable。在纯 layout 已返回、前台安装尚未执行时停止，
  先断言公开 presented revision 仍是旧值，再 suspend。没有给产品源码插入测试 hook、sleep 或私有状态写入。
- `canceling_preview_invalidates_presented_viewport` 比较真实的 committed/preview 命中位置；
  开始手势但不发 Ended，随后通过正式 suspend/resume 取消。恢复后应能命中 committed 位置。
- `failed_compensation_does_not_claim_quiescent_view` 注入两个一次性 panic：原生 resume 失败，
  然后已经恢复的同伴在 suspend 补偿时失败。panic 均被产品 guard 捕获；缺陷断言是剩余活动状态，不是注入本身。
- `failed_resume_does_not_consume_chart_motion_time` 只用 ManualRuntimeClock 推进时间，
  不等待真实 60 秒。排序在 Chart 后面的测试 primitive 在 resume prepare 中推进时钟并失败，
  视图没有成功 Active；第二次正常 resume 后，旧位置丢失命中而终点可命中。
- `geo_link_group_synchronizes_acknowledged_viewport` 在同一个 view 内放置两个相同 map/projection 的 Geo 图。
  custom renderer 只返回位置已知的 Data 矩形，复用产品 Geo viewport、命中和事件路径。
  先验证双方初始命中，再验证源图接受了实际 zoom>2；源旧位置失去命中，目标旧位置却仍可命中。
- fixture 保留 64 次 render 保护，当前本轮用例不触发；它用于阻止既往联动循环回归时测试失控。

## 常规验证

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
GPUI_RHAI_BENCH_SAMPLES=1 GPUI_RHAI_BENCH_WARMUP=0 GPUI_RHAI_CHART_BENCH_POINTS=100000 cargo test --manifest-path tests/performance/Cargo.toml --locked --offline chart_end_to_end_baseline -- --ignored --nocapture
```

结果：workspace 561；native 87；performance 4 通过/3 ignored；fmt/严格 Clippy 通过。
100k v2 debug 单样本通过，包含数据 revision 断言。没有完成 30 样本 release、实体屏幕主题/RTL 或 VoiceOver 认证。
数据 revision 不能证明 resize/viewport 等不改变数据的帧已提交，这正是本轮模型审查的一项结论。
