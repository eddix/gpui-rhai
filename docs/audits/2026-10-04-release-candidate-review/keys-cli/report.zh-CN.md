# RC D2 / CLI 定向复审

基线 `736025a8`（PR #108）。本分支没有确认新的运行时缺陷或发布阻断。
新增 4 项原生按键交叉探针全部通过；从当前 CLI 库源码重新生成的空 capability 和两个 capability consumer 均通过严格编译及 manifest 断言。

未修改产品、正式测试或旧审计，未做 GitHub 写入，未分析 DiskTree。源码散列、实际依赖版本与 feature 记录在 [baseline.json](baseline.json)、[gpui-features.log](gpui-features.log)。核对的是 GPUI pre 0.3.7、Rhai 1.26.0 的实际源码，不采用旧版 GPUI 行为推测。

## 按键契约与新增验证

读验 `tests/native-keyboard/tests/node_key_contract.rs` 及本轮 D2 文档。现有 suite 已覆盖所有 Rhai 注册入口、大小写规范化、标点、原生 phase 顺序、Enter/Space fallback、disabled tab target、raw key/key_char 区分、已提交 IME 文本和 active-gesture Escape；主审查统一复跑完整 native suite，这里不重复运行整套。

新增四项通过：

| 探针 | 实际结果 |
|---|---|
| 同一 native phase 注册多个 handler：`stop` 与 `stop_immediate` | Target/Capture 两种情况下，stop 仍运行同 phase 的下一项，trace=`ST`；stop_immediate 为 `I`。后续 bubble/祖先均被阻断 |
| `on_capture("key:A")`、`on("key:a")`、`on_key_value("A",…,7)` 混用 | 共享规范化 event 与 payload，capture/target trace=`C7T7`，value handler 累加 7 |
| disabled 祖先上有 capture/bubble，内部启用的节点获得焦点 | 只运行子节点 target，trace=`T`，disabled 祖先不截获 |
| Host chord 与 raw node capture 的真实优先级 | 无匹配 chord 的普通 k 到达 capture；匹配 ctrl-k 的 Host action 正常消费，不先经过 raw capture。对应 trace=`CA` / `PA`，详见下节 |

`node.rs` 的四个 Rhai 入口统一通过同一 grammar：key segment 长度 1–60，ASCII 名称或单个非冒号标点，ASCII 大写归一化，小写 `key:` prefix，拒绝 whitespace/chord/多标点/非 ASCII。Rust `NativeHandlerDescriptor` 的事件协议没有因此扩展成 raw-key 语法，这一点文档已经区分。

原生派发使用 GPUI `Keystroke.key`，不使用 `key_char`；modifier 忽略是节点匹配契约，精确 chord 仍由 Host keybinding 负责。`dispatch_ui_handler_phases` 在同 phase 中区分 Stop/StopImmediate，再把结果应用到原生路由；不是只在 Automation 中模拟阶段。

本次不宣称验证了 OS IME preedit 候选窗口或所有键盘布局。已提交文本与实际系统 IME 的人工 gate 仍应分别记录。

## 非阻断文档澄清：Host action 可以先于 raw node capture 消费

`docs/actions-and-keybindings.md:34`–`:35` 的一般描述为：

> GPUI first offers keyboard input to the focused native primitive; unhandled input then reaches the action system.

该描述容易被理解为所有 raw node handlers 都优先于 Host action。实际 pinned GPUI `window.rs` 的 `dispatch_key_event` 顺序是：keystroke interceptor、keymap/action 匹配与分发、在未消费时进入 raw key capture/bubble。`prefer_character_input` 与输入处理器另有分支，不能简化成所有情形都由 raw callback 先处理。

原生对照：普通 k 能进入捕获并返回 Stop；随后 ctrl-k 命中已绑定的 Host action，仍由 action 消费，原 capture 不运行。Stop 并没有被忽略，而是该次输入根本未进入 raw-key 阶段。

这属于 GPUI 既有顺序，**不是 D2 的运行时回归，也不能通过改变运行时优先级来“修好”测试假设**。第 105–109 行的 D2 明确承诺是 capture 阻断后续 raw target/bubble，并没有承诺抢在 Host action 之前。因此这里不列缺陷或阻断，仅建议加一句明确：已消费的 Host keybinding 可能不会进入节点 raw capture；需要更早的策略应使用合适的 Host action/context/interceptor。

## CLI：fresh source、空/非空 manifest、owned main

没有使用之前 `/tmp` 中的旧 release binary 或旧生成文件。本次独立 generator 链接当前 `gpui-rhai-cli` 库，调用实际 `Project::plan_init` / `plan_embed`，生成两个新项目：

- 空 capability：只有初始 immutable `let manifest`。
- 非空 capability：`app.alpha = ^1.0`、`app.beta = *`，分别使用 `let manifest = manifest.with_capability(...)`，不丢弃前一次结果。

两个项目在 init 之前都已有手写 `src/main.rs` sentinel；init/embed 后逐字节验证保持不变。生成代码没有 `let mut manifest`，没有 `allow(unused_mut)`。

consumer 的库根使用 `#![deny(warnings)]` 并公开包含实际生成模块。两个 consumer 都进行了实际编译和 `manifest_contract` 执行，分别确认 0/2 个 requirements 与 `main` entry；各 **1/1 通过**。这不是仅匹配生成文本，也没有用全局 allowance 遮掉 unused_mut。

范围仅为当前源码生成与 consumer 编译/manifest 结果；没有再次做 crates.io 无本地 path 安装，也没有把非空 manifest 当作已注册/已授权 capability 的运行验证。Host 仍需按既有契约注册并激活服务。

## 证据与命令

- [native-probe.log](native-probe.log)：4/4 原生交叉探针。
- [cli-generation.log](cli-generation.log)：当前 CLI 库生成及 owned-main 检查。
- [cli-empty-strict.log](cli-empty-strict.log)、[cli-nonempty-strict.log](cli-nonempty-strict.log)：实际严格编译和两个通过的 manifest 断言。
- [cli-compile-results.json](cli-compile-results.json)：精确命令与退出码。
- [probe/src/lib.rs](probe/src/lib.rs)：原生 probe；[generate_consumers.rs](probe/src/bin/generate_consumers.rs)：fresh fixture 生成器。

```sh
cargo test --manifest-path docs/audits/2026-10-04-release-candidate-review/keys-cli/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline --lib -- --nocapture
```

CLI fixtures 已保存在本轮目录，可按 JSON 内命令直接重跑。共享现有 target 缓存，未建立新的构建 profile；全套 208 native 与最终 RC 结论由主审查汇总。
