# Table 五状态候选截图

来源：本轮PR #104工作区的既有release `data_table` 产物，二进制SHA见[binary.json](binary.json)。本轮没有重建全部release产物，也没有以五张候选图片证明所有发布检查通过。

已采集五个独立bundle的实际窗口：Default Light data/loading/empty、Default Dark selected、Catppuccin Mocha Arabic RTL。启动参数与窗口元数据分别见[evidence/visual-session.json](../evidence/visual-session.json)、[window-metadata.json](window-metadata.json)。

为了避免tiling将窗口压缩，临时仅对本轮创建的精确bundle ID设置floating规则，未保存到用户rift配置文件。结束时已验证runtime app_rules恢复为原值，关闭本轮六个测试进程（含最初尺寸试验窗口），清理临时bundle和私有备份；没有关闭其他会话原有的测试窗口。

## 当前结果

| 用例 | 原始PNG像素 | 980×752候选 | 状态 |
| --- | --- | --- | --- |
| default-light.en.ltr | 1962×1504 | 未生成 | 窗口仍为981×752逻辑点，差1点，不拉伸伪装 |
| default-light.en.loading | 1960×1504 | 已生成 | 仅作审计候选，发现标题越出Table外框 |
| default-light.en.empty | 1960×1504 | 已生成 | 同上 |
| default-dark.en.selected | 1960×1504 | 已生成 | 数据态横向viewport对照 |
| catppuccin-mocha.ar.rtl | 1962×1504 | 未生成 | 差1点，保留原图 |

规范尺寸的三张图仅作2× Retina到1×逻辑像素的密度归一，未改变实际逻辑viewport。其余两张保留原始尺寸。没有覆盖 `tests/visual/macos/data_table/` 的既有正式基线。

窗口标题栏包含系统屏幕控制标记，不是产品UI；后续正式视觉基线应统一记录该环境差异，不能把标记当成组件内容，也不应篡改截图去除它。

## 视觉判断

**Loading和Empty存在确定的P2**：Action标题画到边框以外，data态同列声明则受横向viewport约束。三个early-return分支未复用正常Table外壳。详见[源码与截图核对](../resize-controls/visual-followup.zh-CN.md)。应先修复并重拍，不能把错误状态批准成新baseline。

Score几乎只剩省略号可由当前demo列声明解释：固定730px加25% Joined已经超过可用宽度，flex没有剩余。建议改善默认展示的列宽，或把这组数据明确作为overflow fixture；不将其误报为新的flex算法P1，也不以旧baseline所有列可见来规定任意列配置必须塞进viewport。

结论：**五张现场候选已取得，正式视觉验收尚未通过。** 剩余是修Table状态viewport、明确demo展示策略、消除两张的1点尺寸偏差，并在最终产品提交重拍。当前报告没有声称五张正式基线已更新。
