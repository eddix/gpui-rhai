# 第七轮 Table 共享外壳验收

- 基线：`cda11ce7d80f817e5675543cc7a95f577b977320`。
- 范围：data / loading / original empty / projected empty 的 live transition、固定height / fill_height、受控列宽、横向viewport和RTL。
- 本轮只新增本目录，未改产品／历史证据／真实视觉基线；未启动或截图任何floating应用，未涉及disktree。
- [探针](probe.rs)、[日志](probe.log)：4项，2通过、2失败；两个失败属于同一个横向内容范围问题。使用共用native cache，没有新建大型target。

## 已关闭的部分

`table_shell` 将之前3个early return和data分支收敛为相同的边框／overflow外壳，这个方向正确。原R6指出的“loading/empty标题在表格外仍可绘制／交互”的状态分叉，在本轮原生几何和点击验收中已关闭：

- 在同一个挂载实例中依次 data → loading → 原始空数组 → 查询后空结果 → data。
- Table的固定body height=180时，总高度每次均为212。
- 将A列的受控width从120接受为128后，所有状态都保留128，没有因为body分支更换而回到旧值。
- 每个状态中，对C列位于viewport外的部分点击都不触发排序；可见部分点击仍正确排序C。
- fill_height在固定360px父布局中，Table各状态均保持310px，没有跳回紧凑文本高度。

上述检查是live transition而非分别新建四个窗口。在LTR探针中所有这些断言均通过，最后才因下述横滚检查失败。没有把未获本次许可的实际截图捕获算作验收通过。

示例Score改为固定96px、Name180px、Joined128px、Email承担剩余flex。静态列分配比前轮更适合作为默认可读示例；这不替代专用宽表overflow场景。

## R7-T1 / P1：宽表只有裁剪，没有可用的横向滚动范围

### 实测

合法输入：Table viewport宽300px，固定列宽120/140/160（总420px）；LTR接受A=128后总宽428px。NativeCollection包含1,000行。

- LTR：水平wheel -80px并显式refresh后，A header的x仍为1，没有移动。
- RTL：C列x=-121，依次尝试+160px和-160px的水平wheel，x始终=-121。data、loading、原始empty、projected empty四种状态都一样。
- 正向对照：同一wheel driver、同一native窗口设施，在普通300px横向scroll容器里放一个500px宽的直接子节点，x从0准确变为-80。

因此失败不是wheel方向猜错、输入没有送到窗口或需要再刷一帧。宽表的隐藏列不能通过声明支持的横向滚动访问；只做裁剪，会让数据／操作列在这个视口宽度下无法通过横向滚动访问。

失败探针：

- `round7_live_states_preserve_height_scroll_and_controlled_width`
- `round7_rtl_shared_viewport_can_reveal_overflowing_last_column`

正向驱动对照：`round7_wheel_driver_control_scrolls_a_wide_direct_child`。

### 原因与位置

`registry/components/table.rhai:482-489` 的scroll根直接放入header与body，但没有提供覆盖实际列总宽的横向content track；`:628-633` 的body也保持viewport宽。

运行时几何里，header row与可见body row的布局宽都是298px（300px减边框），但C列在LTR达到x269+160=429。也就是列在行的内部溢出，行自身的布局宽仍没有增加。

本项目锁定的 `gpui-pre 0.3.7` 在 `src/elements/div.rs:1984-2007` 通过**直接子布局bounds**计算scroll content_size；`:2467-2477` 再由它计算scroll_max并clamp offset。给一个仍只有viewport宽的直接子节点加上“孙级列溢出”，不会自动变成较宽的scroll content。

这里的几何日志来自AX投影，所以报告没有把AX树中的每个直接语义子节点误称为GPUI布局的直接子节点；header/body的结构由Rhai源码确认，GPUI计算规则由锁定依赖源码确认。

这个结构问题在原data分支就已存在，不是声称本轮新helper首次引入；共享外壳消除了状态分叉，但没有完成横向内容范围的建模。当前默认示例变得较窄，可以掩盖它，因此保留专门的宽表probe很必要。

### 必要整改与验收

- 为横向viewport提供包含实际已解析列宽的content track，让header和body共享相同的横向范围、列宽和offset。
- 不要只改成overflow_hidden，也不要只把示例或内容宽写死为一个更大的常数。
- 处理RTL的初始逻辑起点和可滚动到另一端的offset方向，不能只修LTR。
- 保留本轮已经通过的边界裁剪、height/fill_height和受控width跨状态行为。
- 用fixed混合percent/flex、resize后超宽、LTR/RTL及四种body状态验证“既裁剪又能实际滚到被遮住的列”。横滚范围正确后再断言非零offset跨状态保持；当前零offset不动不算这一项通过。

这是一个共同范围问题，不建议为四个状态分别打补丁。

## 当前可确认与未覆盖

| 内容 | 结论 |
| --- | --- |
| 原loading/empty越界点击 | live transition中均被阻止 |
| 可见header的排序 | 四状态仍可执行 |
| 固定height / fill_height | 212px / 310px稳定 |
| 受控width在body状态切换间保留 | A列128px保持 |
| 实际横向滚动 | LTR/RTL均失败 |
| 五张真实截图 | 本分支未重拍、不作通过声明 |
| 161项产品native与整体覆盖范围 | 以总审查报告为准，不重复计数 |

## 复现

先检出基线，再从仓库根运行：

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test --offline --locked \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round7/resize-controls/Cargo.toml \
  --test table_round7 -- --nocapture --test-threads=1
```

唯一package为`audit-table-round7`；当前结果`2 passed; 2 failed`。日志包含4态完整几何及普通scroll容器的正向对照。
