# PR #97：标点键 handler 命名评审

日期：2026-10-01。PR：[#97](https://github.com/eddix/gpui-rhai/pull/97)，head `31df838797767811ee29fa2385ec17a298bdf53d`，原 base `815bb82b`。本轮阅读证据中的原始 diff，并通过 `refs/codex-review/pr-97` 固定校验函数原文。GPUI 锁定为 `gpui-pre 0.3.7`。

## 处理建议

**适合纳入 0.1.8；请作者小幅修订后单独合并，不建议原样合入，也不需要主开发重做跨平台键盘层。**

这是有真实接入反馈的现有能力补全：renderer 已经可以按字面 key 分发标点，Rhai 注册边界却禁止它。修复注册规则的方向正确，变化范围也小，和本轮强调键盘交互的目标一致。独立提交便于审阅与回退，不应把它并入交互 Runtime 大整改后隐藏边界。

主审核实的 CI 失败是 Format；后续检查没有执行。修好排版是必要条件，但不能因此忽略下面两个规则问题，也不能把未执行的后续 checks 当成已经通过。

## 确认需要修订的两项

### 1. generic `on("key:Escape")` 新变为注册成功，但不会触发

PR `node.rs:2174–2181` 对 `key:` 后的内容仅检查 printable ASCII；大写 `Escape`、`A` 都能通过。generic `on`/`on_capture`/`on_bubble` 会原样保存事件名，没有做 `on_key_value` 的 trim/lowercase。

renderer 按 `event.keystroke.key` 精确查找 `key_handlers`；GPUI 常规 named key 为 `escape`，字母也有其规范化规则。原生 characterization 中，同样的 escape 事件：

| 保存的 handler key | 实际触发次数 |
| --- | --- |
| `Escape` | 0 |
| `escape` | 1 |

这是新校验放宽后形成的静默无效配置。最小修复可保持 generic 入口要求规范小写名，只让 `on_key_value` 保留现有 lowercase sugar；或者两个入口都显式使用同一规范化函数。两种策略均可，但不能“接受大写、原样保存、永不匹配”。

### 2. 宣称接受“单个标点”，实际接受任意 printable ASCII 序列

PR `node.rs:2205`、`:2215` 的两个 helper 都逐字符检查，没有限制标点分支只能有一个字符。直接调用 PR head 原文 helper，以下值全部为 true：

```text
cmd-s
shift-/
??
```

这些名称不表示 renderer 能匹配的一个 raw key；文档同时明确 chord 应走 Host actions。注册成功但永远不触发会延长接入排错。

建议只有一个 key-name validator：保留现有规范 named identifier 规则，额外接受一个允许的 ASCII 标点字符。不要增加一份语义相同但稍后会漂移的 `is_printable_key_character`。同时明确长度上限是完整 `key:...` 事件名还是 key segment；现在 generic 分支最多可接受 68 字节，但错误消息仍说事件名 1–64 字符。

以上都是作者可以在本 PR 内完成的小修订，不涉及换 GPUI 或设计新的快捷键系统。

## `?`、Shift 和布局：核对后的真实语义

锁定依赖源码证明，常见 Shift 标点应绑定最终字面 symbol，而不是自己把 `shift-/` 存为 key 名：

- macOS：`gpui-pre-macos-0.3.7/src/events.rs:470–478` 对非字母的 shifted key 使用 shifted 字符，并清除 key modifiers 中被折入符号的 Shift。普通美式布局的 Shift+/ 因而为 `key="?"`。
- Linux X11/Wayland 公共转换：`gpui-pre-linux-0.3.7/src/linux/platform.rs:1112–1113` 明确把 slash/question keysym 映射为 `/`、`?`；`:1187–1193` 对符号清除 Shift。
- Windows：`gpui-pre-windows-0.3.7/src/keyboard.rs:148–158` 将需转换的 OEM/数字键映射到 shifted symbol 并清除 Shift；`:175–201` 包含 slash 对应的 `VK_OEM_2`。

原生聚焦节点接受 `key="?"` 后确实调用 handler 一次。这支持 PR 的主要用例，**没有发现“? 在正常 GPUI key 流上根本不能用”的问题**。

但不能把它宣传成所有布局的字符快捷键：

- GPUI `Keystroke` 明确区分 `key` 与 `key_char`。例如 Option/Alt 或 dead-key 布局可能生成另一个可输入字符，`key_char` 才是该字符。
- `Keystroke::should_match` 的 Host keybinding 路径会考虑这层差别；node renderer 的 key handler 仅查 `key`，没有调用该 matcher。
- 模拟 `key="q", key_char="?", alt=true` 时，node `key:?` 不触发；这是现有 raw-key 契约边界，不是本 PR 新引入的布局 bug。
- `Keystroke::parse("shift-/")` 只是构造测试输入，不执行 macOS/Linux/Windows 的硬件布局转换。用它测试 `?` 会得出错误结论；需要构造平台转换后的 `key="?"`，并对真实布局单独实机验收。

没有运行 Linux/Windows GUI 或所有键盘布局。以上跨平台结论是固定版本源码核验，不冒充实机覆盖。

## modifier、payload 与文档需要说清楚

node key matcher 当前**忽略 modifiers，按 key 名匹配**；它不是“只有不按任何修饰键时才匹配”。原生对照确认 `ctrl-/` 仍会命中 node 的 `key:/`（在没有更前置处理器消费事件的测试场景里）。这是现有分发行为，不能为加入标点而偷偷修改。

建议将文档中的 “Modifiers are not part of node key handlers” 写得明确：名称不解析 chord，分发不筛选修饰键；需要精确 chord、布局字符匹配的应用使用 Host actions/keybindings。`on_key_value` 传入的是声明 payload，generic key handler 默认也不是完整的原生 keyboard event，不能假设脚本回调可以自行读 modifier 字段。

还有两个边界应限定表述：

1. `enter`/`space` 是**没有显式相应 key handler 时**回退到 click handler，不是总会额外执行 click。renderer 使用 `key_handlers.get(...).or_else(click...)`。
2. 本 PR 改的是 Rhai 节点命名入口。`NativeHandlerDescriptor::new` 目前另有 snake-case event 名限制，generic `.on("key:...", native_handler(...))` 不能据此推定已整体支持。这个既有 native-handler 命名问题可单独跟进，不必在本小 PR 中扩展整个事件协议。

`:` 被保留是本 PR 选择的 API 范围，不是 GPUI 无法产生该字符。文档应写成“此 API 暂不接受冒号”，不宜说成键盘或平台的限制。本轮不将预 1.0 范围调整列为兼容性 bug。

## 作者最小修订及验收清单

1. 统一命名规则，限定 named identifier 或单个标点；解决 generic 大小写静默失效。覆盖 `on_key_value`、`on`，以及它们共享校验的 capture/bubble 注册入口。
2. 补回归表：`? / [ ] - =`、named key、空串、空白/控制字符、冒号、非 ASCII、过长、`cmd-s`、`shift-/`、`??`、大写 named key，期望与文档一致。不要用“compile/render 成功”代替 handler 身份正确。
3. 在独立 native suite 加实际聚焦后 `?`、`/` 的事件分发；包括不相关键不触发、焦点不在目标路径时不触发、disabled；对 Shift 使用平台转换后的事件。保留一项 modifier 行为测试，防止未来顺手改变现有语义。
4. 修正文档的 raw key / key_char、modifier 和 Enter/Space fallback 说明。布局无关 character binding 或 script chord API 另立需求，不在 0.1.8 临近出货时顺带重设计。
5. `cargo fmt --check` 后让完整 CI 从头通过。PR 的两项边界单测和作者之前的局部测试不能替代当前主干集成验收。

## 独立证据的性质与限制

材料：[probes.rs](probes.rs)、[PR 校验 helper 原文](pr97_validation.rs)、[日志](probes.log)、[dispatch-basis.json](dispatch-basis.json)。

```sh
python3 docs/audits/2026-10-01-dogfooding-pr-triage/pr-97/run-probes.py
```

4 项 characterization：2 PASS、2 个正确契约断言 FAIL。两个失败分别证明多字符标点/chord 误接收、generic 大写名被接受却不分发；两个通过证明 literal `?` 的真实聚焦分发及现有 key/modifier/key_char 边界。

**这不是 PR 全量或端到端构建。** 校验 helper 从固定 PR head 原样提取。原生部分在正在被另一会话修改的本地 workspace 编译，通过 Rust `UiNode::with_handler` 绕开 Rhai 注册边界，仅刻画 PR 没有修改的 renderer 分发链。测试期间 renderer 文件其他位置发生变动，已保存 [live-renderer.diff](live-renderer.diff)；被测 key-down 分发代码块与 `f6e936a5` 的 SHA-256 一致：`f0b60930d25a0dada53445bae9fd95719fde732f78e21ad0fb0e78da535c2963`。没有将 live workspace 结果表述为 PR suite 通过。

运行器只操作自己的临时目录和现有依赖缓存；未切分支、未修改产品、未评论或合并 PR。
