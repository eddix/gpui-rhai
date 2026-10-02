# data_table 五状态截图的补充核对

本补充仅使用当前 `83b19b1d` 源码及总审查已捕获的图片，没有再编译或重录截图。

## Score 只显示省略号：当前合法列声明的结果，不据此报告新 P1

`examples/data_table.rs:117-125` 的声明是：Name固定220、Email固定320、Score flex1、Joined25%、Status固定110、Action固定80。固定列合计730px。

示例外层 `:175` 固定920px且两侧各有 theme lg padding。截图中的 Table 内容 viewport 约886px，Joined再占约221.5px；固定+percent已达约951.5px，没有剩余宽度供 Score 的 flex 分配。`table.rhai:148-155` 明确让 flex 使用0 basis、grow weight和min_width0，符合 USER_GUIDE 的“flex分配剩余空间”契约。因此 Score 的内容区几乎为0、只剩 padding/省略号可以由当前规则解释。

宽表横向 overflow 本来就是合法且有性能场景覆盖的行为（`docs/performance.md:82`）。旧 baseline 的所有列可见，不能成为当前任意声明下“必须自动均分／全塞进去”的保证。

**建议**：这是示例可读性需要打磨，不需要为了截图修改核心 flex 算法。可以给默认展示的 Score 一个实际可读的固定宽度（例如80–96px），保留横向滚动；或使用较温和的默认列组合，把现有过宽组合保留为明确的 overflow fixture。若将来需要 flex 列的最低可读宽度，应单独明确 layout minimum 契约；当前列 `min_width` 主要供 resize 约束使用，`column_style` 并没有将它作为 flex layout floor，不能只加该 prop 后假定已经解决。

## P2：Loading/Empty 丢失 Table 的横向 viewport，标题画出边框

**这是实际的状态一致性缺陷，不是有意允许宽表的合理结果。**

已查看三个精确980×752 PNG：

- [Loading](../visual/default-light.en.loading.png)：表格右边框约x903，Action标题绘制在约x913以后，位于边框外。
- [Empty](../visual/default-light.en.empty.png)：同样的标题泄出。
- [Selected/data 对照](../visual/default-dark.en.selected.png)：同列声明的右侧溢出被限制在Table viewport中，没有把Action标题画到右侧留白区。

根因明确在 public Table composition：

- `registry/components/table.rhai:499-504`，loading early return，仅设置flex_col和border。
- `:509-514`，原始 `props.rows.len == 0` 的 empty early return，同样缺少viewport。
- `:609-614`，查询／分页／投影后 `data.len == 0` 的 empty early return，同样缺少viewport。
- 正常数据分支 `:631-636` 则有统一的min_width0及`overflow_x_scroll()`。

因此同一个合法宽表在data态有横向viewport，在loading/empty态变成默认可见overflow。画面泄出已由截图证明；无需推断未经实测的P1级数据损坏或点击安全问题。

**建议修法**：共享Table外壳、边界与横向滚动区域，只在里面替换data/loading/empty内容。覆盖以上3个分支，避免只给某一条empty路径加clip。不要仅把当前demo改窄来隐藏这个缺陷。`height`/`fill_height`在early return也被绕开，团队可同时明确是否要保留状态切换前的占位尺寸；本补充只把已证实的overflow/clip缺失列为finding，不擅自规定empty必须与满表等高。

该缺陷在PR104之前已经存在，并非新增logical inset引入；当前真实宽表截图使其显现。建议在本轮Table视觉baseline确认前做小范围修正并复拍loading/empty。原#101的RTL修复与12项原生通过结果仍然成立。

## 截图证据边界

上述判断没有依赖尺寸未完全对齐的LTR/RTL raw图，也没有拉伸图片比较旧baseline。980×752的三个规范尺寸图已经足够判定状态外壳的差异。本补充不是对其余截图的完整视觉验收。
