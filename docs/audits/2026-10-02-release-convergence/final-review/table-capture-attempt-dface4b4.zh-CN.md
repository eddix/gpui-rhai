# G 五态捕获尝试与 Rift 恢复记录

日期：2026-10-03。捕获时 HEAD `dface4b456eb1867e7273d75cd4bd2425d315d2a`，
产品源仍为 `53a62e3f2be05e5ae7a961936648195bc4fe9477`。用户明确授权临时修改 Rift
配置；授权没有扩大到其他应用、持久配置、合入、发布或 tag。

## 合同与本次结果

使用既有测试 bundle 和 release `data_table`，没有改产品代码或示例布局来适配截图。
二进制 SHA256：`5ca976270f2387e7c6f4b2e336a000821d1973a7d8c7621f6839bc4bb82b95c1`。
合同为窗口捕获 **980×752 logical**，其中内容 viewport **980×720**；显示密度 2×，
合规原图应为 **1960×1504 physical**。CUA 返回的是 JPEG 字节，不是原生 PNG。

| 状态 | 本轮原图尺寸 | 原图 SHA256 |
| --- | --- | --- |
| default-light / en / default | 1960×1504 | `e802d186b9e0e11dc9443f36cfe03374d3b7597f8e12d1543ed184205f0a683e` |
| default-light / en / loading | 1962×1504 | `0a312bba1965951161d477d834266023e19fda89296dfa25db9ea43ba2ceb2b5` |
| default-light / en / empty | 1962×1504 | `13162ea988e750c704094194f2ce3e32994ae4265fc860429d3a837e5865dde2` |
| default-dark / en / selected | 1962×1504 | `58c1f83b0b620e6ad2e2a6c18fe3fae6ea3829e982ece14a8a3db4a2d4fdcbf6` |
| catppuccin-mocha / ar / default | 1962×1504 | `adf07c94b15dfbb87be7e6244fe2ff2e5aeae777865da75a5a69e6f259017b03` |

原图、未采用的归一候选及私有配置备份位于本机临时目录
`/tmp/gpui-rhai-018-table-capture.8muIUl`，没有将私有配置或不合格图片提交仓库。
不能把四张 981 点宽度的图压缩成 980 点。尺寸合规的一张也不能独自代表五态通过；
捕获中还观察到系统 screen-control 标记和指针光晕。

Rift 已安装版本为 `0.6.2+58697235b26e.dirty`。只读核对其源码后确认，
`position/size` 是 fresh window discovery 的初始放置，不会因热更新对已有浮窗重新应用。
最后一次规则对五个精确 bundle 使用 `floating:true`、`manage:true`、`focus:false`、
`position:{x:0,y:0}`、`size:{w:980,h:752}`，并重新启动自有 Loading 实例；
新实例仍报告 981 点宽。另一次 Dark 检查在 Rift 报告 980×752 时取得 1962×1504 原图，
随后又报告 981 点宽。单凭这些观察不能认定是哪一层重新放置了窗口；不能宣布截图
导致移动，也不能据此判定 Table 产品缺陷。

[独立 Loading 诊断](visual-loading-diagnostic-dface4b4.zh-CN.md)确认该原图 footer 没有
越出窗口，壳层高度与源码的 470+30+2 一致；这不是五态视觉批准。

## 配置与实例恢复

只临时追加下列五个 app ID，没有调用持久配置保存、全局布局恢复、daemon restart 或
修改其他应用规则：

- `com.eddix.gpui-rhai.visual-test.data-table.Yt6e0M`
- `com.eddix.gpui-rhai.visual-test.data-table.R6pvav`
- `com.eddix.gpui-rhai.visual-test.data-table.56OPI1`
- `com.eddix.gpui-rhai.visual-test.data-table.XF7Ffu`
- `com.eddix.gpui-rhai.visual-test.data-table.cOKIQI`

结束后恢复 `virtual_workspaces.app_rules`，重新读取完整 runtime config，并以 JSON
结构比较原备份和恢复结果：

```json
{
  "rules_restored": true,
  "other_config_unchanged": true,
  "original_rule_count": 11,
  "restored_rule_count": 11
}
```

规则经 `jq -S -c '.virtual_workspaces.app_rules'` 规范化后的恢复前/后 SHA256 相同：
`8a4f0e994584a7f71887c0e689d12c9e44d381041248c74e7001e9f209c634aa`。
五个精确 bundle 的 Rift window 查询返回空数组，进程路径查询也没有本轮实例。
关闭前逐一核实了 PID 的测试 bundle 路径；没有按应用名字批量终止。
原有 `wJJ5TX` 测试实例 PID 90509 保留，不声称其他窗口的位置在浮动期间没有自然变化。

**结论：恢复完成，G 仍为 pending_capture_calibration。** 正式五张 PNG 未替换。
下一次必须取得真实且干净的合规原图；不得以修图、改变产品布局或复用旧缺陷截图
代替这个门槛。其余 OS 人工矩阵和 120Hz 门槛仍分别保留。

后续状态：用户另行明确允许原生窗口定位和截图后，取得了新的合规 PNG；本页保留
先前 CUA 失败尝试，不作为当前 G 状态。最新记录见
[原生捕获独立复验](table-native-capture-dface4b4.zh-CN.md)和
[五态捕获 manifest](../../../../tests/visual/macos/data_table/capture.json)。
