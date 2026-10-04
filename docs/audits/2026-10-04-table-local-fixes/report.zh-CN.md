# R8 两处 Table 局部遗漏：修复交付

日期：2026-10-04。仍为 [draft PR #108](https://github.com/eddix/gpui-rhai/pull/108)，
不合入、不关闭线上事项、不发布、不打 tag。disktree-rhai 未修改。

产品修复提交 `3fd6157272b3b133b40c76bf76e03653bb81c844`；最终源码/测试冻结为
`43732b89c48ec6add95cc341d9e1090fe862cec4`。后者只清理新增测试夹具的 strict Clippy
问题，没有改变产品修复、场景或期望。后续视觉/归档提交不冒充新的产品版本。

## 局部修复与正式反例

| 项目 | 修复与证据 |
| --- | --- |
| R8-T1 | 旧方向解码逻辑距离→新范围夹取→新方向编码；只把未初始化当作起点。保留真实 extent、共用列计划和原生跟进帧。修前正式新测试测得 LTR→RTL 的 80→0；修后 Array/Native × 两向保持 80，范围缩小时夹到 72，再切回仍保持 72，表头/首行对齐。 |
| R8-T2 | track/column 进入既有 `NodePresentationMutation::apply` 与 replay。caller 装饰仍和组件自有快照分开；没有另一套 metadata registry。修前两个新 core 测试分别丢失 track/column；修后内部/外部、连续子组件增量更新、width=123 正控、失败保留 last-good 与重试都通过。内部值随新 render 更新，不被旧内部标记冻结。 |

新增正式测试对应 TABLE-04 / NODE-01，见维护的
[契约索引](../../runtime-contract-tests.md)。修前/后的真实日志分别保留在
[evidence](evidence/)，原审查目录和探针未改、未放宽断言。

源码改动仅两个原有入口：`table_layout.rs` 的 offset 转换及 `node.rs` 的两个 builder
和既有 mutation 分支。没有重做 Runtime、虚拟贡献、权限、Ref 或 width 求解。
键盘的非阻断建议只澄清文档：匹配 Host chord 可先被 action 消费，不再承诺一概
“native input 后 action”。GPUI 路由本身未改。

## 验证结果与归属

- 冻结源码 437：workspace **658**、默认库 **475**、默认线程栈 native **209** 全过。
- MSRV 1.95 / 最新 stable 1.99、两套 fmt、严格 workspace/native/performance Clippy、
  public rustdoc、目标清单与性能结构检查全过。最初的新测试 Clippy 失败日志保留；
  修正为 helper 拆分和安全转换，没有加 lint allow。
- Core/Registry package 完整 verify；文件仍为 **95/146**。CLI **12** 文件依旧仅
  `--no-verify` + local patch 的候选边界，不冒称 crates.io 干净安装。
- 完整 release smoke **26 次启动**及产物审计通过。脚本只终止自己启动的 PID。
- release 性能三项 **3/3**，Table/Chart 5 warmups / 30 samples；Document 沿用其
  30-sample/no-dedicated-warmup 模式。metadata 精确记录 437、release 与 dirty=true；
  审计/视觉输出不是生产源码变更。不是跨提交受控 A/B、无噪声机器或物理 120Hz 认证。
- [独立代码复验](independent-review.zh-CN.md)对精确 437 又执行 core **2/2**、native
  **1/1**，限定这两个问题 APPROVE；这些和完整套件重叠，不额外相加。

具体命令、原始输出与边界见 [verification.json](verification.json)；不将通过 source437
的日志改贴到后续归档 head。后续 head 的 CI 必须独立查询并结束，旧 736 的绿灯不能代替。

## 同源视觉复核与恢复

437 的新 release `data_table` binary SHA256：
`4281282285f395906ecbb7e099257a66bb5cab58663f14c42ed6243ea3bb4991`。
五态全部重新原生捕获：AX/CG before/after 一致为 `(100,100,980,752)`、DPI2，raw PNG
**1960×1504**，仅按密度归一为正式 **980×752**。没有压宽、拼补、裁图或 retouch。
[视觉复验](visual-review.zh-CN.md)再次批准这五张；文件完整性审计 **69/69**。

当前真实 OS 标题栏为 Light（上次 Dark），本轮没有更改 OS appearance。用 sips 解码
BMP 比较得到五张所有差异均限于 y=0…33，控件所在 y=34…751 每个像素一致。
这个范围比较只是分析，没有裁切输出图。原图、frames、hash、比较结果及恢复记录归档，
[公开捕获 manifest](../../../tests/visual/macos/data_table/capture.json)同步到437。

仅将原授权的精确五个 bundle 临时排除 Rift 管理；结束后读取并确认原 **11** 条规则及
其他 runtime config 完全相同。未保存持久配置，本轮实例全部退出，旧 PID90509 保留。
快照不替代真实滚动行为；方向保持来自正式 native 帧测试，不是这五张零 offset 图。

## 剩余发布门槛

两项代码 P2 已修并限定独立通过，无本轮新代码阻断；整个候选仍不自动变成 release ready。

1. 新整合 head 的完整 CI：推送后查其实际状态，不沿用736已通过的 CI。
2. 实际 OS 输入/辅助功能：真实 keyboard/focus、clipboard、IME preedit、VoiceOver 与
   适用窗口交互，仍 pending；模拟输入与 native appearance override 不替代。
3. 物理 120Hz：当前记录为60Hz，仍 pending hardware，不自行标 N/A 或降低门槛。
4. crates.io 无 patch 消费在获准按依赖顺序发布后执行；当前没有发包/tag授权。

既定 #83/#89/#91/#95 等后续增强不倒灌0.1.8；PR/issue处置沿用原计划，尚未合入/关闭。
