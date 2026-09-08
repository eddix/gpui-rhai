# 0788e3c 验收整改记录

本文件记录对[验收报告](report.zh-CN.md)中 C01–C06 的后续整改。原始报告、
探针与 evidence 保持原样，继续作为 `0788e3c` 未通过验收的不可变证据；本文件
不回写或弱化该结论。

## 整改结果

| 项目 | 修复后的契约 | 自动回归 |
| --- | --- | --- |
| C01 / #42 | import 提取不再维护 Rhai 模板字符串的平行 lexer 状态机，而是解析未优化 AST 并遍历 `Stmt::Import`；死分支 import 仍被发现，动态或越界 import 仍被拒绝 | 9 类嵌套模板/注释/import 集合用例；真实 `gpui-rhai check` 项目包含中文、多层模板和模板后 import；参考探针验证全部 50 个 registry 组件 |
| C02 | 每次进入保存的 `NativeCallContext` 前只对齐其历史绝对计数，不重置当前 execution session 的累计值 | 相同 100,000 次增量组件工作在 0/250,000 次父级历史下都记录 300,015 次；两个 dirty 组件仍共享并触发总预算；延迟虚拟项同样不重复计父级历史 |
| C03 | generation 失效立即丢弃其缓存值、关闭并移除注册项，不受 drain limit 或缓存是否为空影响 | 满队列、`limit=0`、旧 generation 和阻塞 producer 组合测试 |
| C04 | Linux 原生依赖安装移到第一个可能链接 GPUI/X11 的 Cargo 检查之前 | 本地 workflow 顺序检查；真实 GitHub `portable` 结果以更新 PR 的运行结果为准 |
| C05 | 语义 event/action trace 只保存 kind、scope 与名称，不保存 payload | sensitive state → event/action → TraceBuffer/Inspector 数据链测试，合成 token 不出现在 trace 中 |
| C06 | close predicate 的改变与 Condvar capacity wait 使用同一 queue mutex；所有 close 路径在释放锁后唤醒等待者 | 测试等待 `blocked_producers > 0` 后才触发 cancel 或 generation stale，避免依赖概率调度 |

## 本地验证

- `cargo test --workspace --all-targets --all-features --locked`：443 项通过
  （core 343、downstream primitive 1、formal component 21、registry 43、
  examples 10、CLI 24、registry crate 1）。
- 原生 GPUI 工作区：49 项通过。
- performance 工作区：2 项结构测试通过，2 项显式 benchmark 默认忽略；随后以
  release、3 次 warmup、10 次 sample 显式运行 Table/Document 两项 benchmark。
- 严格 workspace、native、performance Clippy，fmt，core/CLI rustdoc 与
  `git diff --check` 通过。
- package：core 与 registry 完整验证通过；CLI 使用本地 0.1.1 patch 生成包并按
  发布顺序暂不 verify。
- verification manifest 覆盖 21 个 example，38 个 PNG 完整校验通过；release
  smoke 覆盖全部 21 个 example、Theme Studio 与 DataTable 四种视觉状态。
- 原验收 `probes.rs` 已重新执行：#42、操作预算、订阅回收与 trace 泄漏均翻转为
  预期结果；`issue42-reference.rs` 的两种参考策略仍与生产 AST 方案一致。

## 性能观察

同机 release 结果没有显示系统性回退：Document direct-string prepare 为
47.328 ms，native-document prepare 为 1.872 ms；Table unchanged/reverse/
selection/resize p95 分别为 5.300/11.319/21.786/13.070 ms。该组数字用于发现
数量级回退，不替代相邻二进制、受控机器上的正式 A/B 认证。

## 尚待外部完成

更新分支推送后，必须等待 GitHub `portable` 完整通过，才能关闭 C04。真实多窗口
输入、IME/AX、全量 GUI 启动 smoke 与 120 Hz 帧认证不因本轮整改而自动获得认证。
