# 050c7fa2 复验入口

从仓库根目录执行。所有 probe 使用独立临时 Cargo package，不改产品源码。

## 原始四项整改

```bash
bash docs/audits/2026-09-21-issue-fixes-6373e4b3/run-native.sh
```

当前应为 **9/9 通过**。

## 新增正确行为检查

```bash
bash docs/audits/2026-09-21-issue-fixes-6373e4b3/acceptance-050c7fa2/run-native.sh
```

当前应为 **4 tests：2 通过、2 失败，exit 101**：

- `changing_inherited_group_does_not_keep_old_presentation` 通过；
- `virtual_items_keep_current_inherited_group` 通过；
- `svg_text_survives_raster_adapter` 失败，原 GPUI 有文字像素，新适配器为 0；
- `svg_document_color_overrides_inherited_tint` 失败，外部绿色覆盖 SVG 明确声明的红色。

字体测试先断言控制组在本机能绘制字形，避免把未安装字体的机器误判成适配器回归。

## Release 前台 drain 测量

```bash
bash docs/audits/2026-09-21-issue-fixes-6373e4b3/acceptance-050c7fa2/run-drain-bench.sh
```

每个尺寸新建 3 个 registry，调用公开 `start_image_decode`，再测成功投递那次 `drain_image_decodes` 的耗时。callback 通过 RuntimeEngine 的合法脚本 handler 取得，没有依赖私有 API。此程序不调用回调，也不把线程等待时间记入 drain 耗时。

使用 release + thin LTO。测量时避免其他编译负载。结果是前台函数成本，不是整机 FPS；未设易受机器负载影响的时间断言。

## 缓存访问点

```bash
rg -n 'inline_svg_images' crates/gpui-rhai/src
```

J04 是源码级容量/生命周期问题；本轮没有通过故意耗尽内存来复现。

## 产品基线

```bash
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

performance 的两项 ignored 未执行。本轮也没有声称完成真实屏幕、display-link 或全平台性能认证。
