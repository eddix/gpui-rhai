# 0.1.8 收敛候选交付

日期：2026-10-03。状态：**产品契约与自动门槛已收敛，候选尚未完整就绪**。
授权边界保持：不合入、不关闭线上事项、不发布、不打 tag；disktree-rhai 未修改。

产品源码冻结于 `53a62e3f2be05e5ae7a961936648195bc4fe9477`；
完整测试源码候选为 `be7a8eb83f5c97b9932661d74e5c3b7c684be4ce`。
二者 core/CLI/registry 与 performance harness 字节一致，后者只追加原生
生命周期探针和文档。后续证据归档提交不是新的产品实现。
最终整合候选在 [draft PR #108](https://github.com/eddix/gpui-rhai/pull/108)。

## 实施结果

| 工作包 | 结果与统一边界 |
| --- | --- |
| A–C | 窗口命令在入队时固定 origin/目标 reservation，pump 不授予权限；Table 共享原生列计划/真实 extent；ElementRef 保留逻辑订阅并迁移当前几何目标。保留最终 invocation manifest、合批 delta、贡献读取与 Style COW，核心检查点独立通过。 |
| D1 | 接管 #96，可信 Host quota 与 parser depth 独立配置，默认不升限；candidate Cell 独立，诊断区分 timing span 和累计回合。 |
| D2 | 接管 #97，统一单键名称及 Capture/Target/Bubble 原生路由，不把 modifier chord 或 key_char 混入契约。 |
| D3 | 接管 #99，轻量 resolved metadata，native 外观先于 primary/secondary init；最终以弱订阅接入真实外观通知，defer 正常前台事务，暂停只保留 dirty，释放时取消订阅。 |
| E | #98 的有界后台数据→前台资产刷新桥与共享窗口重绘已验证；Gallery 增宽表/四态/主题读数；公开指南、最终模型、合同索引、release notes、处置表与许可证矩阵更新；空/非空 capability 生成 consumer 均消除 unused_mut。 |
| F | 全目标工作区、默认 feature、原生、工具链、Clippy、文档、打包、release 启动、产物审计、性能与 CLI consumer 门槛通过。远端完整 CI 以精确源码/最终 head 的实际结果为准。 |
| G | **已完成并独立通过**：用户另行授权原生定位/捕获后，取得五张干净 1960×1504 PNG，仅按 2×→1× 更新正式 980×752 基线。AX/CG 前后均 980×752@(100,100)；原 runtime 配置完整恢复，自有实例关闭。见 [原生捕获复验](final-review/table-native-capture-dface4b4.zh-CN.md)；[早期失败尝试](final-review/table-capture-attempt-dface4b4.zh-CN.md)保留。 |
| H | 独立代码/行为及 G 视觉复验已完成；整体验收仍受既定 OS 人工、120Hz 与最终 head CI 门槛约束，未授权发布。 |

最终模型与正式测试对应关系见 [合同索引](../../runtime-contract-tests.md)、
[ADR 0022 补充](../../adr/0022-final-runtime-invariants.md)。
原作者提交映射及原 8 个 PR、5 个 issue 的处置见
[事项处置表](dispositions.zh-CN.md)：可关闭不等于已关闭，四项 0.1.9 设计保持 open。

## 必须保留的失败与修正

- 核心独立检查首先发现 prepaint 中 `Window::refresh()` 无效；改用有界原生
  follow-up frame，保留旧失败探针，再独立通过 `99f77b1a`。没有用额外刷新掩盖。
- 全工作区/线上 `33d60119` 失败于两个过时的 Table 叶样式断言。没有恢复重复
  宽度逻辑；测试改守单一消费者，并新增原 `120/flex(2)/30%/90` 的原生几何正控。
- 实机主题检查最初两个非 idle run 为假正向。加入 750ms 真正静默后双向复现：
  native 已变而 Host/script 仍旧。修复共同窗口 ingress 后，原探针原样转绿；
  新增独立 Runtime、fixed、暂停恢复、旧 Handle 重建和 token-only 原生呈现正控。
  全部前后日志都保存，旧通过不再代表 idle 契约。
- CLI 干净 consumer 发现生成码无条件 `let mut manifest`。生成器改 immutable
  shadowing，没有 `allow(unused_mut)`；empty/two-cap 分支严格编译并实际 prepare。

这几项分别补足真实帧提交、最终布局表示、环境事件与生成源边界，不新增兼容层、
持续重绘、额外安全额度或组件专用绕行。

## 验证摘要

- 产品源码 53：workspace all-target/all-feature **656**，默认库 **473**；
  Rust **1.95.0 / stable 1.99.0**，fmt、严格 Clippy、两份 rustdoc 通过。
- 独立 native 套件 **208**，默认线程栈；新 actual MacPlatform 二进制的 harness
  `0 tests` 不计入原生行为数。真实 app run 另按 JSON 断言及 cleanup 判定，
  不能仅依赖 AppKit 可能返回的 exit 0。
- Core/Registry package 完整 verify **95/146 文件**；CLI **12 文件**仅按批准边界
  `--no-verify` + local patch，尚未声称无 patch crates.io 重建/安装。
- release 的 20 个 examples、Theme Studio、Gallery、四个 Table 状态共 26 次
  启动（25 个长驻、一个正常短驻 performance probe）；产物路径/inspector 审计通过。
- actual macOS 初始 Light/Dark 两向切换、primary/secondary init-once、真实 idle、
  同窗口多 View/fixed 对照、暂停/恢复、同 ID 重建和纯 token 呈现通过；所有自有
  app 窗口关闭，清除本进程 override，未改 OS 偏好、Rift 或别的 app。
- 761 个锁定 all-features Cargo package 的 metadata license/source 矩阵无缺项；
  package 清单/生产图没有审计文件、GPUI test-support、dev-reload、gpui-component
  或 Rhai fork。逐项证据与命令见 [verification.json](verification.json)。

### 性能解释

主参考为 `f53-quiet-observed-benchmark.json`：5 warmups/30 samples 的 Table/Chart，
Document 30 samples 无专门 warmup。进程采样排除了并行编译/链接；已有旧测试窗
PID 90509 的约 2.3–3.0% CPU 噪声保留，没有越权关闭，故不称完全空机。

Table cold prepare/mount 9.596/47.858ms；p95 unchanged/reverse/selection/resize
为 11.129/21.078/27.244/22.366ms。Unchanged reuse=5，virtual operations=0；
resize 根 operations=0，但新实现行可有 8169 virtual operations，realized 19–26。
Chart streaming operations=0。不要把 resize 总 operations 或整个 GUI 帧时间写成 0。
这些是 TestPlatform 多步场景耗时，不是 120Hz 帧率认证，也不是受控跨提交 A/B。

## 未完成门槛与下一步

G 已完成：五张 Table 原图、窗口测量、binary SHA、差异说明与恢复证据已归档，
正式基线文件审计 **69/69** 通过。原生接口定位仅作用于五个精确测试 bundle，
没有修改产品布局、OS 外观、其他 app 规则或持久 Rift 配置；原有 PID 90509 保留。
默认六列 fixture 本次不溢出，照片不能代替正式 native 的 LTR/RTL 横滚断言。

剩余门槛：

1. OS 输入/辅助功能人工矩阵：IME preedit、剪贴板、VoiceOver、真实焦点/输入等
   变更相关组合尚未本轮重验。模拟输入和 AX tree 不是这些人工结果。
2. 物理 120Hz：当前 Macmini9,1/M1/16GiB、macOS 27.0.1 26A434、AC Power 的
   显示器为 60Hz，无硬件门槛通过证据。需适用硬件/人工验收，不自行降级为 N/A。
3. 精确归档提交 `dface4b4` 的 [完整 CI](https://github.com/eddix/gpui-rhai/actions/runs/37041257700)
   已通过，包含 Linux X11/Wayland smoke。后续提交需核对对应 head，不沿用旧绿灯。

完成后按固定合同再交独立复验，给用户精确候选与外部操作列表；只有用户明确批准
后才能合入、关闭替代事项、按依赖顺序发三个 crate、干净安装和创建 tag/release。
目前状态保持“候选未完整就绪”，不是库已发布或 main 已合入。
