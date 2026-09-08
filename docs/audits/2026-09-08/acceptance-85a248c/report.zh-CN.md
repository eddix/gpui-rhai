# 85a248c 复验与 issue #40 确认

基线：`85a248c4cb5e1d38797579312d33c40b9949b077`。产品工作区干净；本轮没有修改产品代码或向issue发送消息。

**上一轮验收中的C01–C06通过本轮定向复验，真实GitHub CI也已通过。issue #40确认是仍未修复的独立问题，并且不只影响数组：NativeCollection的Command路径同样存在。** 此结论不替代未重新执行的真实IME/AX/120Hz平台认证。0.1.1历史API兼容继续不在验收范围内。

## 1. 本地修复的复验结果

| 检查 | 结果 |
| --- | --- |
| workspace/all-targets/all-features，locked/offline | 443 passed |
| native-keyboard独立工作区 | 49 passed |
| fmt、严格全目标全特性Clippy | 通过 |
| C01 / #42 | 嵌套模板、模板后真实import、插值内import、嵌套注释探针均正常 |
| C02 | 相同增量工作在轻/重父render下均成功，计量均为300,015；多dirty组件共享总预算和延迟虚拟项的回归测试通过 |
| C03 | 旧generation带缓存订阅被回收，重复drain后active=0；满队列及limit=0测试通过 |
| C04 | 安装依赖的顺序已调整，完整提交对应的[GitHub CI成功](https://github.com/eddix/gpui-rhai/actions/runs/34213075003) |
| C05 | sensitive state经语义事件后的trace payload为None，事件/action完整路径测试通过 |
| C06 | close和capacity wait现使用同一mutex协议；阻塞producer的cancel/stale唤醒测试通过 |

证据：[workspace](evidence/workspace-tests.log)、[native](evidence/native-tests.log)、[Clippy](evidence/clippy.log)、[旧失败探针复跑](evidence/regression-probes.log)、[元数据](evidence/metadata.json)。上一轮的失败报告保留历史原状。

## 2. #40 的直接原因

[issue #40](https://github.com/eddix/gpui-rhai/issues/40)对“高亮来自roving行背景而非sticky层”的判断正确。

关键代码链：

1. `registry/components/command.rhai:192–226`：数组模型把group标题和item放进同一数据序列；`render_command_row:154–186`为标题设置`role=heading`，只为真正的item设置活动样式和交互。
2. `command.rhai:274–278`：构建virtual_collection时传了data与reveal_key，没有单独的导航资格信息或受控行焦点。
3. `virtual_list_element.rs:266–276`：`install_keys`仅根据`sticky_headers`排除行，然后在无内部focused key时`focus_first()`。
4. `virtual_list_element.rs:446–476`：row wrapper只比较内部focused key是否等于data key；相等就画整行`surface_hover`背景。这个背景不是标题本身的Style，也不以当前系统键盘焦点是否位于列表为前提。

所以标题即使没有on_click、也不是Command当前active item，仍会获得外层wrapper画出的整行背景。这里的“focus_first落到标题”准确地说是**VirtualListState的内部行高亮状态**，不表示macOS系统焦点移动到了一个heading文本节点。

## 3. 影响范围比issue中的数组分支更宽

数组路径确实为空：`native_collection.rs:1468–1472`中的`VirtualCollectionData::Values`返回空sticky集合。

但Native Command也一样：`NativeCollection::fuzzy_view:229–236`显式设置`sticky_headers = empty`。具有派生sticky headers的是另一条Table/grouped_order路径；不能把它推断成所有NativeCollection都自动排除group标题。

本轮将**当前官方Command/Input/Kbd源码实际交给ScriptLifecycle执行**，得到完整小规模virtual collection快照，再按当前`install_keys`的相同排除条件调用公开VirtualListState。8个场景覆盖array/native × 分组/不分组 × 首项enabled/disabled。小fixture全部实现，避免把“没有实现的行”误当成缺失数据。

关键结果：

| 场景 | sticky集合 | 内部first | Command活动项 |
| --- | --- | --- | --- |
| Array，分组 | 空 | group:G1，role=heading | item:b |
| Native，分组 | 空 | __gpui_rhai_group__:command:G1，role=heading | b |
| Array，不分组、首项禁用 | 空 | item:a，disabled=true | item:b |
| Native，不分组、首项禁用 | 空 | a，disabled=true | b |

完整[探针源码](issue40-probe.rs)和[输出](evidence/issue40-probe.log)已附。Native fixture的原始data key是a/b/c，fuzzy投影给实现后的option node加了`item:`前缀，探针对此做了明确区分，没有错误地把实现节点key等同于native data key。

证据边界：这是生产Rhai数据投影＋公开roving策略的复验，并结合真实renderer代码确认绘制条件；本轮没有新采集GPU截图或驱动真实桌面焦点操作。

## 4. 不能只过滤heading就宣告修复完成

Command已经用`active_value`、`enabled_values`、`model.next/previous/first/last`控制语义活动项；通用VirtualListView又有独立的`VirtualListState.focused`。`reveal_controlled_target:225–264`只负责滚动，没有同步内部focused key。

因此还有两个直接相关的问题：

- 即使没有group，若active_value为b，内部状态仍默认选a，外层背景与真实活动项仍可能各画一行。
- disabled item并未被通用roving序列排除。Command自己的enabled序列会跳过它，但虚拟列表内部序列不会。

当前的真实GPUI测试`grouped_command_initial_reveal_keeps_its_first_header_natural`（`keyboard.rs:4759`）验证了Native Command的初始滚动、无sticky overlay和heading可见性；没有验证row wrapper背景、导航资格或它与active_value的一致性，所以通过这个测试不能关闭#40。

## 5. 推荐整改边界

保持之前natural-origin/reveal/sticky修复，不需要撤销它们。把**呈现行为、可导航资格、活动项所有权**分开：

1. 对已经由Command接管键盘与高亮的场景，优先让virtual_collection只负责虚拟呈现、滚动和reveal，停用它独立的roving选择/行背景；或提供明确的受控active_key模式，使同一个活动状态驱动所有行呈现。
2. 如需保留通用键盘导航，提供明确的可导航数据/索引策略。对Command排除heading和disabled item，Array与Native使用同一语义。sticky_headers只决定固定标题的呈现，不再充当交互资格的替代信息。
3. 只根据当前realized node的role排除行，需要处理视口外尚未实现的行；否则导航资格会随虚拟实现窗口变化。更稳妥的是从完整数据投影提供资格信息。
4. 不建议仅给Command传sticky_headers来消掉背景，因为那同时改变了分组标题的滚动吸顶行为；也不要只把标题背景设透明，隐藏了错误的内部导航状态。

这些是同一版本内部的行为修正，不需要历史兼容层。

## 6. 关闭#40前的验收矩阵

- Array/Native均测分组、不分组、无结果和全部disabled。
- active_value初始指定第二或后面的项目；只呈现它对应的活动行，不出现另一条默认first高亮。
- 搜索输入拥有系统焦点时、Tab进入列表后、鼠标hover/click后分别验证活动项和背景一致。
- Up/Down/Home/End只在该组件允许的项目之间移动；group与disabled不进入Command活动序列。
- 受控active_value改变、查询过滤删除原活动项、滚动超过首屏后仍一致。
- 原始位置、sticky自然边界、reveal、滚动保持和已有CommandDialog行为不回退。

复跑本轮issue数据探针：

```sh
GPUI_RHAI_ACCEPT_REPO="$PWD" bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/acceptance-85a248c/issue40-probe.rs"
```

该探针打印当前缺陷的可达状态；正式修复应新增真实GPUI背景/活动行断言，而不是长期保留“标题被focus才通过”的断言。
