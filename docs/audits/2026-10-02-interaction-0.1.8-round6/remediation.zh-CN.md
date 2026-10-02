# 第六轮框架整改

日期：2026-10-02。基线 `83b19b1d`（PR #104），仅修改 gpui-rhai；disktree-rhai 产品、依赖、配置与文档均未修改。原始第六轮报告、探针、候选截图与 SHA256SUMS 不变。本记录是实施回归，不替代独立验收；#104 暂缓合入，不发包、不创建 tag。

## 工作包与提交

| 项目 | 提交 | 模型边界 |
| --- | --- | --- |
| queued native authority | `c29753ae` | 每次 View mount 有独立 lease；Handle/Entity 清理按同一 mount 匹配。命令执行前验证 source 与 target 的 lease 仍是实际 WindowId 的当前授权，不以 Entity 临时 Active 代替权限 |
| virtual batch + dependency contributions | `1c682c7e` | scope 只合入自己的 state candidate delta；统一最终 invocation manifest，最后一次清理 incarnation/event/resources。读取分别记录 executable owner 与 component/virtual-item contribution |
| Table state viewport | `eaedccad` | data/loading/original-empty/projected-empty 共用 table_shell，保留横向 viewport、边框和 height/fill_height 规则；Score 展示改为可读 fixed 宽度，核心合法 overflow 契约不变 |

### 授权

Host view registration 和 window command owner 共享同一次 mount 的 lease。App 的 owner 索引与原生目标 registration 只保存 weak identity。最后一个 Handle drop 会撤掉该 lease，即使旧 GPUI frame 持有 Entity；旧 Entity 随后清理也只能删除自己的 registration，不能通过重复 view_id 清理新 owner。

Focus/Close 在 defer 执行时再验证 source、target、实际 WindowId；force-close 标记只在有效操作执行时建立。Open 在 command pump 中检查 source 权威。仍持有一个 Handle clone 的正常 owner 不会被误撤权。

原始三项 GPUI 探针 **3/3 通过**。新增正式 window_authority suite **5/5**，覆盖 dispose/drop、新/同 Host、更换/复用 view_id、Close/Focus、clone 正控和 replacement 确认；已有 window_ownership **6/6** 保持通过。

### 批次

没有交换提交顺序或放宽 stale 检查。`StateStore::commit_render_batch` 合并各 scope 负责的候选实例，保留 scope 外最新写入/删除，最后按最终 active 集合统一裁剪。Engine 的最终 invocation/collection 图决定存活关系，不再消费祖先的中间 active 清单逐次删除后代。

原始父子/并列 probe 中，同批与顺序结果一致：新内层 Counter 的自有 callback 返回7；移出的 A-row0 不存在，重挂为默认7。正式三项 virtual_transactions 覆盖两种顺序/同批、effect/timer/signal/ref Counter、父裁剪及子过期请求、跨 scope 候选失败回滚。StateStore 定向测试确认其他 view 最新写入与删除不会被旧 scoped snapshot 覆盖。

### 贡献存活

共享 ReadDependency 模型区分 owner 与 contribution，virtual item 按 collection identity + stable data key 标识。Retained item 保留贡献；完整 target 在准备及最终提交边界释放移出的贡献；失败候选随 runtime snapshot 回滚。不清空 root 所有 reader，不提高64上限。

Store（含 exact path）、NativeCollection、NativeTextDocument、theme/locale/viewport，以及已解析/待解析的 geometry/ref reader 使用同一规则；对外 invalidation 仍返回 executable owner。

原始 raw/formal 66项程序均只保留当前一个缺失名字，source65 注册能触发刷新并显示 ready。正式两项 virtual_read_contributions 验证80项有界滚动、移出名字不误唤醒、并列集合同名贡献、root direct 读取、共享名字保留、失败回滚。共享 registry 单测覆盖 store/environment/geometry 贡献释放与 owner 唤醒。

### Table

没有只改窄例子掩盖问题。原生 state-shell 测试使用真实宽表，覆盖 data、loading、rows=[]、query 后投影为空；四种状态高度一致，viewport 外不能排序，内侧可见标题仍可排序。原生 table_boundaries **8/8**，既有 RTL、拖动、autofit 与 keyboard 正控保持通过。

## 验证与证据

- workspace all-target/all-feature **623 项**；其中 core all-feature **482 项**，默认 core **452 项**。
- 独立 native workspace **161 项**，默认线程栈；工作区和独立 native 严格 Clippy、fmt 通过。
- public rustdoc、performance structure（2 passed，3 显式 ignored 可选测量）、20 examples manifest 与既有69张 PNG 文件审计通过。
- core/registry package 实际解包编译验证通过；CLI 使用现有 no-verify/local patch 边界，完整 workspace/CLI release build 验证依赖图。
- 完整 macOS release smoke 与 release artifact audit 通过。
- 原始报告探针运行结果、前后对照、正式测试及 gate 输出保存在 `remediation-evidence/`。原始 SHA 校验全部通过。

## 仍未声明完成的事情

固定尺寸 Table 视觉基线尚未替换。新的产品 binary 已 build，但重拍需要仅对测试 app 临时 floating；已单独向用户请求该临时操作，未改 Rift 或其他应用设置。原候选图包含旧缺陷，不能移作修复后证据；原 PNG 文件审计也不是实机重新验收。

接手清单中的 #96/#97/#99 是下一组0.1.8集成工作，当前未宣称修订/合入。#100/#101/#103 等待 #104 最终验收后再按已覆盖状态处理；#14、#98及0.1.9增强项未关闭。用户排除的 disktree-rhai 不纳入实施范围。
