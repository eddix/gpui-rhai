# 0.1.7 Gallery / Acceptance Application 验收记录

日期：2026-09-26。分支：`codex/acceptance-app-0.1.7`。PR：#79。

本记录区分已验证与待完成项；它不是发布批准。#79 堆叠于仍为 Draft
的 gpui-pre PR #78，不能绕过其上游与平台门槛。

## 已验证产品面

- `gpui-rhai gallery` 正式入口包含 Explore 与 Operations Workbench。
- `--list` 不初始化窗口；story/case/theme/locale 非法值返回明确错误。
- registry 随包分发 13 个实际 source 文件，catalog 稳定列出 11 个
  stories；公开 Component/Motion/Chart module coverage 由枚举计算。
- Explore 支持搜索、category、case、Reset、主题、locale、
  Normal/Reduced/None Motion、Auto/Compact/Regular/Wide viewport、精确运行
  source、fixture/module/platform/test/docs metadata。
- 所有 story/case 均真实 prepare、mount、draw，并提交至少一个非零几何
  的 semantic node。全部 15 个 bundled themes 在同一 design story 上热切换
  并真实 draw；中文与 Arabic story、三档 viewport 有原生 frame 回归。
- Operations 的 normal/cancel/failure、loading、empty、streaming、large、
  config diff、theme override cases 均通过。Rust 提供 hosts/events
  NativeCollection、metrics NativeChartData、config NativeTextDocument 和有界
  subscription；Rhai 持有业务状态、页面、确认和呈现。
- Hosts 的 1,000 行 search/sort/page/group 在 NativeCollection 中排序、过滤、
  切页后再虚拟投影；Rhai 不取得完整行数组。
- HostSlot acceptance story 在两个独立 ScriptViewHost 间使用
  `with_script_view`。中文 native input 回归通过；parent/resident 作为一个
  Gallery cache 生命周期组处理。
- #68、#70、#71、#73–#77 对应正式实现、原生回归和可见 story/case 均已
  落地；issue 仅在最新远端 CI 通过后回复关闭。

## 自动验证证据

- workspace/all-targets/all-features：438 核心单元测试、21 Chart contract、
  21 formal-component、50 official-registry，以及全部 20 Cargo examples、CLI
  和 registry tests 通过。
- core default：408 核心测试、21 formal、50 registry 通过。
- core charts：435 核心、21 Chart contract、21 formal、50 registry 通过。
- `tests/native-keyboard/tests/gallery.rs`：14 项 source-backed acceptance tests
  （all cases、normal/cancel/failure、Host fixtures、overlays、HostSlot IME、
  theme/locale/viewport、Tabs geometry、Chart invalid semantics、suspend/resume）
  通过。
- workspace 与独立 performance workspace 严格 Clippy `-D warnings` 通过；
  gpui-rhai 与 CLI warnings-as-errors rustdoc 通过。
- 三 crate package 通过；registry package 列表包含全部 13 个 story files。
- release smoke 通过 20 examples、Theme Studio、Gallery Operations large case
  和 Data Table 4 个状态；Gallery 从空临时 cwd 启动且零文件写入。
- release artifact audit 通过；38 张既有 macOS PNG 与 29 张新 Gallery PNG
  的格式、CRC、尺寸与数量 audit 通过，共 67 张。
- release benchmark（Rust 1.95.0，Macmini9,1，macOS 26.6.2，5 warmups /
  30 samples）三组通过。100k `charts/streaming` 每次 revision 真正呈现，
  streaming Rhai operations 为 0。完整数字记录于 `docs/performance.md`。

## 本轮真实 macOS 窗口观察

通过 release `.app` bundle 与 Accessibility tree 检查：

- 搜索过滤不切换当前 story；category 过滤不重置当前 story。
- Button/Table/Component/Chart/Motion/Operations/HostSlot stories 可切换，source
  pane 随运行文件更新。
- Default Dark→Light、en→zh-CN 不清空 Button/Workbench 状态。
- Auto→Regular 真实改变 `ctx.viewport_class()`；固定 viewport 改为上下预览/
  source 布局后 source 保持可见。
- Operations failure 展示 48% progress、Failed Badge、危险 Toast，并明确旧
  configuration 仍活动。
- HostSlot resident form 在 release 窗口内真实呈现，parent/resident 同步主题。
- macOS AX tree 可见 Button、TextField、Tab、Table、Figure、Dialog、Toast、
  Progress、HostSlot resident semantics。Chart invalid case由原生测试进一步验证
  invalid description。
- VoiceOver 首次实机激活暴露并稳定复现了一个原生崩溃：普通文本 retained
  wrapper 被发布为无 `value`/character lengths 的 AccessKit `TextRun`，
  `accesskit_consumer::text::Range::traverse_text` 在生成节点 value 时解包失败。
  修复后普通文本按 GPUI 原生约定发布为带直接 value 的 `Label`。重新启动的
  release Gallery 可完整枚举 Component Catalog、Operations 的 Table/Figure、
  CommandDialog 与 HostSlot 语义；主题切换、Command 项原生 `Pick` 动作和
  HostSlot 输入均成功，进程持续存活且未新增 crash report。
- 当前实机的 `system_profiler SPDisplaysDataType` 报告 LG HDR 4K 为
  5120×2880 物理像素、2560×1440 逻辑分辨率、60 Hz。release Gallery 在该
  2× HiDPI 屏幕完成全屏切换与 resize 观察；这项证据不覆盖 1× DPI 或
  120 Hz。
- 视觉捕获为每个状态创建并首次启动独立 release `.app` bundle；29 次冷启动
  均成功进入真实窗口并绘制完成，覆盖正式启用的 runtime-shader 路径。

CUA 的按键注入绕过输入法候选窗，因此不把 `zhongwen` 当作候选提交证据；
维护者已确认中文输入不是本轮剩余发布问题。HostSlot 的同源 native
`simulate_input` 与 VoiceOver 开启后的真实 TextInputClient 输入均通过。

## 尚未完成／仍阻塞

- macOS：VoiceOver、Overlay、原生 `Pick`、HostSlot 输入、2× resize 与
  runtime-shader 冷启动已通过。1× DPI 与 120 Hz 当前硬件未覆盖，作为平台
  矩阵缺口记录，不阻塞 0.1.7。
- Gallery 新 surface 的 29 张 checked-in PNG 已完成逐组 contact-sheet 复核，
  覆盖 Component、Chart、Motion、HostSlot 与 Operations 代表矩阵。
- Linux：X11、Wayland、输入、滚动、Overlay 和基础 AT-SPI 真实窗口矩阵
  未执行；portable CI 不能代替。
- 两套约定 dogfooding 迁移尚未在本记录中取得最终签字。
- `gpui-pre 0.3.6` 尚未包含 upstream wrapped-line hit-test #64672。该路径在
  Linux selectable/Code/Diff 文本理论上可达，但需要零宽换行字符与约
  1e-5 px 命中缝隙；按维护者决策降级为 Linux 已知上游限制，不阻塞整个
  0.1.7。下一个包含修复的完整 pre 家族发布后立即升级。

在这些门槛完成前，PR #79 必须保持 Draft，不得合并、发布 crates.io 或创建
0.1.7 release。
