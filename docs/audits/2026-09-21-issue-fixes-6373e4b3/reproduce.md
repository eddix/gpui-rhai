# 复现与回归入口

基线：`6373e4b3016fc4da2560fe7f106feec812912dde`。在仓库根目录运行：

```bash
bash docs/audits/2026-09-21-issue-fixes-6373e4b3/run-native.sh
```

使用临时独立 Cargo package 和项目锁定的 GPUI/Rhai 依赖，不修改产品代码。当前基线应为 **9 tests，5 passed，4 failed，exit 101**。

四个失败的正确行为断言：

- `motion_group_survives_incremental_component_updates`：none / explicit group 成功，外层 inherited group 点击失败。
- `svg_fixed_colors_preserve_bgra`：currentColor 控制组通过，固定红色像素通道错误。
- `svg_semantic_color_preserves_alpha`：alpha=0 被丢弃，实际 alpha=255。
- `host_slot_preserves_independent_script_view_host`：同 Host 与显式 container 控制组通过，不同 Host 的便捷 helper 失败。

五个通过项：

- HostSlot native click 与 Rhai capture 隔离；slot 外点击验证 capture 控制组确实有效。
- HostSlot 填充剩余 flex 高度。
- 固定父高度下的 Table height / fill_height 对照。
- 百分比父高度下的同样对照。
- caller-owned node prop 外的 keyed wrapper 使用 opacity / translate_y enter_motion，子组件点击仍正常。

可在 runner 最后一条 cargo test 中追加测试名以单独运行某一项。测试采用 GPUI test platform，可无物理截图验证布局、交互和 SVG decoder 输出；不认证真实 display-link 或 GPU 帧率。

产品检查与之前的 Motion 回归：

```bash
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo test --manifest-path tests/performance/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
bash docs/audits/2026-09-20-motion-runtime-2/acceptance-29413b9d/run-acceptance.sh
```

performance 的 2 项 ignored 未执行，不计作性能认证。本轮 GitHub issue 只读取，未评论、关闭或新建 issue。
