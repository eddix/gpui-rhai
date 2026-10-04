# R8 Table native 列计划验收

- 候选：PR #108，`736025a8b2b1ac44d41d893bc9a929c990b97532`。
- 对比入口：R7 `cda11ce7`；已阅读 release-convergence 的 checkpoint C 和 final-review，区分历史初版失败与后续修正。
- 本分支只新增本目录，未改产品／旧审计／GitHub；使用既有 native cache，没有重新跑全矩阵。
- 证据：[原生日志](probe.log)、[新探针](probe.rs)、[原字节 R7 探针副本](round7.rs)、[来源 SHA256](provenance.json)。

## 结论

**原 R7 的“宽表只能裁剪、无法横滚”P1 已关闭。** 原始4项探针未修改断言或输入，新候选全部通过。新的中间percent列resize与viewport/state交叉对照也通过。

未确认新的P1。发现一个较小的**方向切换策略不一致（P2）**：LTR→RTL会清零非零逻辑offset，RTL→LTR则保留；这是新跨入口对照，不是重报已修复的extent或首帧排程问题。

本次合计6个native测试：5通过、1因方向切换对照失败。它不是整个候选的测试计数；208项产品native与五张截图由总审查负责。

## R7 P1 的关闭证据

完全相同的300px视口与宽列输入：

- header/可见row布局宽度从此前298px变为实际428px。
- LTR水平wheel -80后A.x从1变为-79；loading、原始empty、投影empty、data之间保持这个非零offset及已接受A宽128。
- RTL的C.x从-121滚到1，再能滚回-121；四态均可到达隐藏末列。
- 越界header点击仍被屏蔽，可见header排序仍可执行。
- 固定body height180时Table总高212，fill_height对照总高310，跨状态不跳变。

复跑副本与历史原文件字节相同，SHA256写入provenance；历史失败日志未被覆盖。

## 新列计划与首次测量的核对

`table_layout.rs` 确实建立一个viewport-based plan：fixed、viewport百分比、带权flex以及本帧native override进入同一求解器；实际scroll直接子track使用extent，header和已实现virtual cells应用同一个结果。百分比没有改成对scroll extent取百分比。

`node.rs:1858-1905` 给header/body传递同一plan，并在嵌套table处停止向下传播；`renderer.rs` 包装实际track，解决了R7的“孙级列溢出不计入GPUI直接子content_size”。padding/border与组件min/max进入plan，而不是只扩大clip框。

特别核查了旧 C-TABLE-01：当前 `table_layout.rs:445-453` 在viewport/border测量变化时调用 `request_animation_frame()`。不再使用prepaint中无效的refresh。已有正式TABLE-03测试检查实际跟进帧、0 Rhai和有界idle；父审查复跑该suite。本分支没有把旧失败再当当前缺陷，也没有用强制refresh来声称旧问题仍存在。

## 新正向交叉：中间percent列的本地resize

覆盖两个不同组合，避免重复完整矩阵：LTR Array 与 RTL NativeCollection。

- 500px外视口、实际viewport498；A fixed120、B percent50、C flex1：header/body的B均为249。
- 拖动中间B的handle 40px，B预览均为289；采集到的Rhai operations为0。
- 不提供resize callback，释放后本地accepted override保留。
- 将外视口扩大到700，B保持289，没有再次按50%变为349；C获得剩余289，header/body一致。
- 此后loading / 原始empty / projected empty / data四态，B与C均保持上述正确宽度。

此用例同时检查了“percent descriptor暂被native override固定”与“其他flex列重新分配剩余空间”，不是只验证首列fixed或末列fixed。

## R8-T1 / P2：LTR→RTL清零offset，反方向却保留

### 实测

相同固定宽列、相同300px视口，先滚到距逻辑起点80px，再使用公开Host `select_locale` 切换方向。驱动正常调度及已请求的native follow-up：

| 切换 | 切换前逻辑距离 | 切换后逻辑距离 |
| --- | ---: | ---: |
| LTR → RTL | 80 | 0 |
| RTL → LTR | 80 | 80 |

没有改变列、数据、viewport或controlled width。位置是按实际header几何相对于Table的逻辑起点计算，不是只看内部state变量。

### 原因

`table_layout.rs:422-428` 进入RTL时仅对“之前也为RTL”恢复logical distance；**之前为LTR与首次无状态被一起当成0**。`:429-430` 的反向分支却正确使用旧RTL距离。

### 修订建议与优先级边界

内部注释 `:415-417` 表达的是保留距logical start的距离，反向分支也实现了它。建议先按旧direction统一解码logical distance：None→0、LTR→-old_offset、RTL→old_maximum+old_offset；对新range clamp后再按新direction编码。不要改已通过的extent和native follow-up机制。

公开USER_GUIDE目前明确承诺viewport与四态变更保留logical offset，没有逐字承诺语言方向切换。因此这里按**较小的一致性问题**报告，不把它升级为P1或声称隐藏列再次不可达。若产品确实决定切换方向应回起点，也可以明确文档并统一两向行为；当前单向重置缺少明确依据。

探针：`changing_direction_preserves_distance_from_logical_start`。反向切换是同一测试中的通过对照。

## 边界与复现

本分支没有新增全量theme/min-max/flex参数矩阵，没有重复实际截图，也没有宣称OS手工门槛通过。新增交叉只覆盖上述代表组合；原生autofit、single-column、insets与更完整Array/Native×LTR/RTL矩阵由当前正式suite及总审查结果提供，不冒称本分支另跑。

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-10-04-release-candidate-review/table/Cargo.toml \
  --test table_round8 -- --nocapture --test-threads=1
```

运行前后应核对产品HEAD为上述候选。当前结果：`5 passed; 1 failed`。
