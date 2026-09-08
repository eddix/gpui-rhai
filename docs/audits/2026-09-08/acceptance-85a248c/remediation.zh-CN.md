# 85a248c 复验与 #40 整改记录

本文件记录对[复验报告](report.zh-CN.md)中 issue #40 的后续整改。原始报告、
探针和 evidence 保持原样，继续描述 `85a248c` 的真实行为。

## 架构结论

问题不是 Array 或 NativeCollection 缺少一组特殊 sticky 索引，而是 generic
`virtual_collection` 不应拥有第二套活动项状态。旧实现的内部 `VirtualListState`
既不向组件发出 active change，也不能表达 disabled/structural 资格，却会自动
`focus_first()`、进入 Tab 序列并绘制整行背景；这与 Command、Combobox 已有的
受控 active 模型重复。

整改后的职责是：

- `virtual_collection` 只负责实现、测量、滚动、controlled reveal 和 sticky
  presentation；不选择活动项、不进入 Tab 序列、不处理方向键，也不绘制通用
  active 背景。
- Command/Combobox 从完整 Array 或 NativeCollection 模型推导 enabled 序列，
  由同一个受控 active 值驱动键盘、语义状态、组件 part style 和 `reveal_key`。
- `sticky_headers` 只表示固定标题的布局行为，不再承担交互资格。
- Command 在全 disabled 时不再构造非法的空 active reveal key；没有活动项时
  完全省略 `reveal_key`。

因此不需要为 Array 标记 heading 索引，也不需要让 Native fuzzy projection
伪造 sticky metadata。已有的 sticky push-off、natural origin、reveal 和手动滚动
修复均保留。

## 自动验收

- 新增 Array/Native × grouped/ungrouped × first enabled/disabled 矩阵：
  `active_value = b` 时恰好只有 B 的 option 语义为 active；Home/End/Up/Down 的
  payload 只来自 enabled command item，group 与 disabled item 不可达。
- 新增 Array/Native 的无结果与全 disabled 矩阵：没有 active option、方向键/
  Home/End/Enter handler；全 disabled 但仍有展示行时不会生成 reveal target。
- 现有真实 GPUI 测试继续覆盖搜索输入持有系统焦点时的 Down/Enter、鼠标 hover、
  CommandDialog、Native grouped natural origin、wheel scroll、受控 active reveal 和
  手动滚动保持。
- workspace/all-targets/all-features 共 443 项通过（core 341、downstream
  primitive 1、formal component 21、registry 45、examples 10、CLI 24、registry
  crate 1）；独立原生 GPUI 49 项通过；performance structure 2 项通过。
- workspace、native 与 performance 严格 Clippy、fmt、`git diff --check` 通过。
- verification manifest 的 21 个 example、38 个 PNG 完整校验、全部 release
  example、Theme Studio 与 DataTable 四种状态 smoke 通过。

旧 `issue40-probe.rs` 会手动重新构造已经从生产 renderer 删除的
`install_keys + focus_first` 逻辑，因此它在整改后仍能打印旧状态，但不再描述
可达的产品路径。该探针保留为根因证据，正式防回归由上述 source-model 与真实
GPUI 测试承担。

## 性能观察

同机 release、3 次 warmup、10 次 sample：Document direct/native prepare 为
46.636/1.817 ms；Table unchanged/reverse/selection/resize p95 为
5.747/12.089/21.566/13.985 ms。相较上一轮未出现数量级或一致性回退；该组短样本
仍不替代受控相邻二进制 A/B。

## 外部验收

提交推送后仍需等待 GitHub `portable` 完整通过，并由 dogfooding 视觉确认分组
标题不再出现整行活动背景。真实 IME/AX 与 120 Hz 平台认证不因本轮自动获得。
