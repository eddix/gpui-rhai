# 第五轮：PR #101 Table 列边界与末列 resize 评审

- 主项目基线：`23fce23829c6694c3e077a61a20a9038b87f0759`。
- 本次实际测试对象：PR #101 head `f9203250609dd2c2bc0696a70e277753dbdb65e1`，其 base 为 `815bb82b`；**不是已合入的 main 功能**。
- macOS aarch64 / Rust 1.95.0 / gpui-pre 0.3.7。
- 证据：[PR 元数据](../evidence/pr-101.json)、[完整 diff](../evidence/pr-101.diff)、[原生 probes](probe.rs)、[结果](probe.log)。

## 合入决定

**作者小改后合入，建议包含在 0.1.8；当前头不能直接合。** 需求和基本实现方向合理：修正真实边界 hit target，并使最后一列／单列表格可调整宽度。原生 resize 机制、owner/ref、preview signal 不需要重写。

必须先修正 RTL 下物理 left/right 的定位假设并补方向测试。该问题已经在固定 PR 源码上复现；CI cancelled 本身不是代码失败。

## 合入阻断：RTL 的分隔条落到错误列边界，拖 resize 变成邻列排序

位置：**PR head `registry/components/table.rhai:400`** 固定使用 `left:-4` / `right:0`；`:453-459` 把列 i 的 handle 挂到后一个 header。这种 sibling 组织可以保留，但 anchor 应解释为逻辑 start/end，而不是物理 left/right。

Table 的 row 会随 RTL 改成 `flex_row_reverse`；`ColumnResizePrimitive` 的 pointer delta 本身也已理解 RTL。此次修复把原来的 logical margin 改成绝对物理 inset，却没有同步方向语义。

### 固定 PR head 的实际几何

三个固定列 A/B/C，宽度 120/140/160，官方 Arabic locale：

| owner | 正确的 RTL 逻辑结束边 | 当前 handle 位置 |
| --- | ---: | ---: |
| A | header 左边 x479 | handle 中心 x339 |
| B | header 左边 x339 | handle 中心 x179 |
| 最后一列 C | handle 应沿 header 左边 x179 | handle 左边却在 x331（自己的右侧） |

这是整列宽度级别的错位，不是半像素或 border 测量误差。

更直接的行为：在 A 的真实边界往相邻 header 内 2px 处按下，向左拖 40px。数据来自 **1,000 行 NativeCollection**，预期 A 从120增加到160并且不排序；实际结果为：

```text
resize event = none
sorted column = b
```

也就是原本想拖分隔条，却触发了 B 列排序。对应两个失败探针：

- `rtl_handles_follow_the_owned_columns_logical_end`
- `rtl_dragging_a_boundary_resizes_a_not_its_neighbor`

### 修订边界

1. 内部分隔条仍可放在下一 header 的**逻辑起始边**：LTR 左侧、RTL 右侧。
2. 最后一列的 handle 放在自身**逻辑结束边**：LTR 右侧、RTL 左侧，并保持 outer-edge hit area 的有意内收规则。
3. 保留 owner 的 `column_key`、source width、signal 和 header_ref；不要把它们改成“handle 所在的邻列”。当前这些绑定在 LTR 中是正确的。
4. 方向方案还应支持没有加载 locale catalog 的普通应用。若用 `ctx.text_direction()`，注意当前 API 在无 LocaleManager 时会返回错误，不要为一个 resize handle 无意新增“必须安装 locale”的前提。优先复用框架的有效方向／逻辑边表达，必要时只补很小的公共能力。
5. 补 RTL Array、RTL NativeCollection、RTL 单列／末列的行为检查。不要只检查 fluent style 是否写了 `.left(-4)`；PR 修改后的结构单测恰好固定了这个错误的物理边假设。

不需要为了这项修订引入新的 column resize coordinator、第二套信号模型、列拖放或重新实现 Table。

## 已确认通过的边界与交互

新增 6 个独立原生探针，**4 通过，2 失败（同一个 RTL 根因）**。

| 正向场景 | 实际结果 |
| --- | --- |
| LTR Array，前两个 handle 中心与自己列的结束边比较 | 误差小于0.5px |
| LTR Array，从 A 边界右侧2px开始拖动+40 | 回调 `a:160.0`，没有触发邻列排序 |
| 同场景拖最后 C 列+40 | 回调 `c:200.0`，没有排序 |
| 单列 Table | 唯一末列的 handle 存在，160→200 |
| 1,000 行 NativeCollection，最后 C 列 double-click autofit | 回调 `c:313.0`，没有排序；header 与已呈现 gridcell 宽度一致 |
| NativeCollection，B 不可调整但 A/C 可调整 | B 没有 separator；挂在 B 内的 A handle 仍回调 A；最后 C 的公开 Automation `key:right` 回调 `c:168.0`，没有排序 |

这些对照说明：header refs 预声明、相邻 header 中的 owner 绑定、末列 autofit 的 body 测量、共享 header/body width signal 和 keyboard retained handler 在已测 LTR 条件下工作。缺陷集中在方向定位，不应把整个 PR 判成需要主开发重写。

## 范围、集成与 CI

- PR 只调整 Rhai composition 和测试，不改 native `ColumnResizePrimitive`。宽度 override 的既有 renderer 会清除 flex basis/grow/shrink，因此其“调整 flex 列后进入固定宽度”在机制上有支撑；本分支未新增 flex/percent 完整布局矩阵，不能把静态判断写成原生测试通过。
- 没有改变前几轮受控／非受控 resize 取消的验收标准。PR head 基于较旧 `815bb82b`，最终应在当前 main 上更新分支并重跑已有 interaction 回归；不得用本报告去证明旧 base 已包含后来的修复。
- [CI 证据](../evidence/ci-101.json)显示 portable 在 **Install native Linux test dependencies** 阶段被取消，后续 fmt/check/clippy/test 都 skipped。准确结论是“尚未获得 CI 代码验证结果”，不是“代码 CI 失败”，也不是“CI 已通过”。修订后重跑正常流水线即可。
- 作者已明确说明 visual baselines 尚未重新捕获，当前 inventory audit 只确认文件存在。RTL 修正后再更新受影响的真实 Table 视觉 baseline；本次没有添加参考截图或把几何测试冒充截图验收。
- 本分支没有覆盖所有皮肤 part overrides、任意缩放、列动态增删或所有横向 overflow 情况；没有为找到更多问题继续扩大不相关构建。

## 重现

```sh
python3 docs/audits/2026-10-02-interaction-0.1.8-round5/resize-controls/run-probe.py
```

runner 从固定 `f9203250` 归档源码至本目录下唯一临时 snapshot，只修改临时 workspace members 以排除无关 CLI；产品代码原样编译。使用唯一 package `audit-pr101-table-boundary`、test target `pr101_table_boundary`，复用 `tests/native-keyboard/target`，退出后删除临时源码快照。

主目录、正式测试、历史审计与 GitHub 状态均未修改。
