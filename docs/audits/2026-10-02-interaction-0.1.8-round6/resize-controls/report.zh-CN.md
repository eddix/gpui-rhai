# 第六轮验收：PR #104 的逻辑定位与 Table resize

- 验收 HEAD：`83b19b1d2bc742523f4b44caa8dbe42f6b83637c`，对比 `c088e96f`。
- 本分支范围：#101 整合后的 `inset_start` / `inset_end`、RTL 列边界与 owner/ref、末列 keyboard/autofit、无 LocaleManager 的默认行为。
- 产品只读，编译前后核对 HEAD 与相关产品文件未改变。磁盘清理得到通知后才运行小型 target，未创建新构建目录。

**结论：原 #101 的 RTL 反例已关闭，12 个原生交互场景全部通过。** 后续补充截图核对确认了1个既有P2：loading/empty分支缺少横向viewport，宽表标题会泄出边框；见[视觉补充](visual-followup.zh-CN.md)。Score的flex塌缩与它区分处理，不据此新增P1。这个结论不替代 PR #104 的窗口生命周期和完整CI验收。

证据：[probe.rs](probe.rs)、[probe.log](probe.log)。前 7 项直接 include 当前正式 `tests/native-keyboard/tests/table_boundaries.rs`，其中包含上一轮 6 个独立回归场景及实现方新增的 RTL keyboard 检查；另外新增 5 个交叉场景。没有修改正式测试或历史证据。

## 实现核对

- `style.rs` 的新 logical inset 字段参与 `StyleProperties::merge`，Rhai builder 支持 definite、signed offset 和 auto；没有只添加 API 名字而漏掉下游。
- `renderer.rs:4649-4652` 让 inset token 值经过既有 length resolver；`:4721-4735` 使用统一 `logical_horizontal_edges` 按当前渲染方向映射物理 left/right。
- Table 内部分隔条使用下一 header 的 logical start，最后分隔条使用自己 header 的 logical end。native props 的 `column_key`、width signal、measurement ref 仍属于真正被调整的列。
- 新增 handle 自己的 element_ref 服务原生 focus；测量宽度的 ref 仍指 column header，两者没有混用。
- Table 没有调用 `ctx.text_direction()`，因此不会因修复 RTL 而给无 locale catalog 的普通应用引入 LocaleUnavailable 前提；默认 LTR 来自已有 renderer 环境。

## 前轮反例的关闭证据

同样的三个 RTL 列 A/B/C，宽度120/140/160：

| 几何 | #101 原始头 | 当前 #104 |
| --- | ---: | ---: |
| A 的 logical end / handle center | 479 / 339 | 479 / 479 |
| B 的 logical end / handle center | 339 / 179 | 339 / 339 |
| 最后 C 的 header left / handle left | 179 / 331 | 179 / 179 |

在 A 的真实边界、进入相邻 header 2px 处向左拖40：

- 前轮：没有 resize，触发 B 排序（`none|b`）。
- 当前：正确提交 `a:160.0|none`，没有邻列排序。

该行为复验使用1,000行 NativeCollection；另有 LTR Array、单列、不可调整邻列、末列 autofit 与 retained keyboard 对照全部通过。

## 新增边界

| 新场景 | 原生结果 |
| --- | --- |
| 不安装任何 locale catalog，普通 Table 的 A 边界拖动+40 | 正确 `a:160.0|none`；未抛 LocaleUnavailable |
| RTL 最后一列 C，通过新 handle ref 聚焦后按真实 Left/Right | `c:168.0` → `c:160.0`；始终属于 C |
| 上述 RTL 最后一列再 double-click autofit | `c:313.0|none`；header 与已呈现 body gridcell 宽度一致 |
| RTL 单列表格，从唯一列的 outer logical end 拖动-40 | 正确 `c:200.0|none` |
| 无 locale，300px容器，8px子项，`inset_start(offset_px(-4))` 与 `inset_end(theme_spacing("xs"))` | x分别为-4、288；默认xs解析为4px |
| 同一布局 RTL | x分别为296、4，signed start 与主题 end 均正确反转 |

最后两行由两个测试覆盖；“末列 ref/keyboard/autofit”是一个组合测试，因此新增测试数为5，总计12。

## 覆盖边界

- 原生交互测试没有新增缺陷；后续视觉补充的P2独立记录，不重复历史已关闭的RTL问题。
- `auto()`、逻辑边覆盖同向物理边、style merge 的静态链路已检查；本原生新场景主要覆盖 signed inset 和 theme length。未宣称穷举所有 auto/百分比与双侧过约束组合。
- 未新测所有 flex/percent 列宽、极窄视口和横向 overflow 滚动组合；本轮核心测量采用清楚的固定宽列，明确验证方向、owner、ref 与数据来源。
- GPUI 原生测试中的几何／命中／focus 结果不替代 macOS 实际截图。五张截图由总审查独立检查，本分支未改或重录视觉基线。
- 历史 resize、motion 等完整探针由总审查统一复验，本分支没有修改或重复声明它们的整体结果。

## 重现

需要检出对应 HEAD，从仓库根运行：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round6/resize-controls/Cargo.toml \
  --test pr104_table_round6 -- --nocapture --test-threads=1
```

独立 package 为 `audit-pr104-table-round6`，复用既有 native cache。结果：`12 passed; 0 failed; 0 ignored`。
