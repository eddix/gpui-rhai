# gpui-pre 0.1.6 候选实施报告

日期：2026-09-25。实现分支：`codex/gpui-pre-0.1.6`。基线：
`v0.1.5` / `120ef2239079ee5e2205c05a9b982768b096a19a`。

状态：**代码适配和自动化门槛通过；仍是发布阻塞候选，不可合并／发布。**

## 已完成

- Rust 1.95.0 稳定版直接编译 `gpui-pre 0.3.6` / platform probe，未使用
  nightly、`RUSTC_BOOTSTRAP`、Git/path patch 或依赖源码修改。
- 根 workspace、native-keyboard 与 performance 三套锁文件使用同一
  `bcf6582ce3500df93a8a39366640173e6786cea6` 快照；产品图不存在官方
  `gpui 0.2.2`、Kit/Base/Component/CE 或 `test-support` 泄漏。
- 真实平台入口迁到 `gpui_platform::application()`；公开重导出
  `gpui`/`gpui_platform`。独立 `/tmp` Host 工程只通过这两个重导出构造
  窗口并在 clean lockfile 下编译。
- Context、focus、Timer、TextStyle、WrappedLine、scroll geometry、shadow、
  image format 与窗口关闭 API 已按 0.3.6 源码适配。真实后台线程测试显式
  使用新版 deterministic scheduler 的 parking 边界，没有全局关闭断言。
- `CommittedSemanticFrame` 与 retained candidate 同事务提交。自动化与
  GPUI/AccessKit 消费同一投影，geometry 只从已呈现帧连接；未知 role 在
  commit 前拒绝。
- role、名称/描述、author ID、selected/expanded/toggled/current、value/range、
  orientation、required/invalid/disabled/read-only、placeholder、shortcut、集合
  位置与表格行列元数据均有公开投射。
- Input、Textarea、Slider、Chart 的 AX Focus/SetValue/Increment/Decrement 进入
  原 controlled/native event 路径；disabled、read-only 与 stale instance 拒绝。
- Table、Command、Combobox 使用有界 realized 语义。typed compound setter 将
  Table 第一版新增的 435 次 resize Rhai operations 降到 297 次，同时保留
  cell/row 导航。
- `gpui-rhai check` 识别旧、混装和 workspace-inherited GPUI package identity，
  只给修改建议，不写用户 `Cargo.toml`。
- macOS release 构建的 gpui-pre 产物包含 `stitched_shaders.metal`，不包含
  gpui-pre `shaders.metallib`；普通构建不调用独立 Metal Toolchain。运行期
  冷启动和失败行为仍需解锁实机确认。

## 自动化结果

- Workspace：569 项通过。
- 独立 native-keyboard：101 项通过。
- 性能结构：4 项通过；3 项显式 release benchmark 另行实际执行。
- `cargo fmt`、workspace/两套独立 workspace 严格 Clippy、warnings-as-errors
  rustdoc 全部通过。
- target manifest 覆盖 23 个 examples；38 张视觉基线库存通过。
- core 82 文件、registry 111 文件 clean package 验证通过。
- 13 个 release 二进制产物审计通过；23 个 example、Theme Studio、Table
  selected/loading/empty/grouped release smoke 通过。
- Draft PR [#78](https://github.com/eddix/gpui-rhai/pull/78) 的首轮免费 Linux
  portable CI 全通过，耗时 29分08秒。由于原 30 分钟上限只剩 52 秒余量，
  后续提交将上限调整为 45 分钟；没有删除或放宽门禁。

## 同机 A/B

机器、系统、Rust 1.95.0、release profile、5 次预热和 30 个样本保持一致。
详细方法和表格见 `docs/performance.md`。

| 场景 | v0.1.5 | pre-AX | AX 候选 |
|---|---:|---:|---:|
| Chart resize p95 | 23.87ms | 17.74ms | 17.83ms |
| Chart stream p95 | 76.00ms | 73.08ms | 74.48ms |
| Document resize settle p95 | 25.12ms | 24.04ms | 24.10ms |
| Table resize p95 | 23.67ms | 23.73ms | 26.44ms |
| Table resize Rhai ops | 3,417 | 3,417 | 3,714 |

原生 AX 未启用时不创建 GPUI semantic wrapper。Table 剩余约 2.7ms / 11.4%
差异来自可见 cell/row 的明确语义创建；root Rhai operations 保持 0，未出现
整树或全数据重建。Chart streaming Rhai operations 保持 0。

## 发布阻塞项

1. `gpui-pre 0.3.6` 不包含 Zed #64672 wrapped-line hit-test panic 修复。它只能
   用于适配，不能成为 0.1.6 发布版本。
2. 当前 macOS 桌面处于锁定状态，Component Gallery 进程和事件循环可启动，
   但不能完成 VoiceOver、真实 AX action、CJK IME、DPI、窗口关闭资源和
   runtime-shader 冷启动人工验收。
3. Linux CI 已真实编译 X11/Wayland 双 backend 并执行 headless tests，但仍需
   X11 与 Wayland 各自的真窗口、输入、文本与基础 AT-SPI smoke。
4. 约定的现有 Rust GPUI Host dogfooding 与主要 Rhai UI dogfooding 尚未接入。

## Dogfooding 必验建议

- Cargo package identity、MSRV、平台入口、HostSlot/custom primitive。
- 既有 Rhai 源不修改即可启动、热重载和更新 source snapshot。
- CJK IME commit/cancel、selection、clipboard、focus restore。
- 全部官方交互组件 role/state/action；图标按钮本地化名称；disabled/read-only
  与 stale action。
- Table/Command/Combobox 的有界导航，Chart summary/focus/selection。
- Theme 热切换、Motion active-time/reduced policy、Chart link/viewport。
- 多窗口关闭重开、suspend/resume、失败补偿与迟到 async 结果。
- release CLI init/update/check/embed 与实际打包产物。

