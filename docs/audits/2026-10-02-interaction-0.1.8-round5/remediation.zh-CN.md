# 第五轮整改记录

日期：2026-10-02。基线：`23fce238`；产品整改提交：`01489c31777f`。本记录是实施与回归结果，不替代独立验收；仍不发布 0.1.8 或创建 tag。

## 修复边界

| 项目 | 统一后的约束 | 验证 |
| --- | --- | --- |
| R5-1 透明组件组合 | 保留清单从可见调用根沿已提交的 invocation parent 图闭包展开，不因父组件覆盖显示根标记而丢掉后代 | 直接 Counter、透明 Wrapper、三层 Wrapper、容器 Wrapper 均保留 state/effect/timer/signal/ref；滚动后的后代 state 更新仍可执行；纯裁剪释放资源，失败候选回滚 |
| R5-2 路径依赖 | app/window 的 path 与 whole-field 读取使用同一 executable callback owner | 原始六种 store 对照通过；正式测试确认 sibling path 不误刷新、目标 path 只失效真实 owner |
| R5-3 嵌套虚拟目标 | 查找与替换覆盖 realized 子树和 Custom Node/Nodes 槽；内层提交同步更新包含它的组件 snapshot；native slot runtime 继续携带 retained links | 内层延迟 row10、回调 owner、无关父更新后的 snapshot reuse、外层保留/纯裁剪、失败回滚通过；同批父裁剪丢弃过期子请求；替换仍 move map 而非逐层复制 |
| R5-4 内部遮挡 | 当前帧 viewport hitbox 在子内容 prepaint 前插入，延续滚动仍要求真实祖先和当前会话已接受的目的地 | 开放空隙继续；内部 occluding 拒绝区停止；新会话不继承取消会话的目的地，三项通过 |
| R5-5 PanZoom 锚点 | 外框用于捕获命中，Canvas 的实际 drawable rectangle 用于滚轮/键盘缩放 | 原始 GPUI CPU Scene recorder：对称 `(106,66)` 和非对称 `(126,66)` 锚点均零漂移 |
| R5-6 非阻断数值边界 | 逆矩阵奇异判定相对线性尺度，不把很小但良态的矩阵当成奇异 | `1e-9` native selection 通过；`1e-200…1e200` CPU roundtrip 与真奇异矩阵拒绝通过 |

`VirtualCollectionRecipe` 删除重复的 event owner/schema 副本，延迟 callback 直接消费保留的 UiContext 的 executable owner/incarnation/schema。没有放宽 generation、incarnation 或 effect activation 校验。

## 回归结果

- 前四轮固定源码 **89/89 GPUI、11/11 async** 探针通过；Canvas CPU recorder 通过；schema **12,870** 组一致。
- 第五轮原始 collections/tree 程序通过所有 store、Wrapper、rollback、nested 对照。drag **3/3**、transforms **4/4**、当前 checkout 上的 async/default **4/4** 通过；PanZoom 原始 paint recorder 两个锚点对照通过。
- workspace all-target/all-feature **615 项**，其中 core library **479 项**；默认 feature core **449 项**。
- 产品 native **141 项**全部通过，包含新纳入的 drag continuation 三项、transformed selection 四项。原始审计探针没有被修改。
- performance structure **2 项通过、3 项显式忽略的可选性能测量**；workspace 和独立 native workspace 严格 Clippy 通过；fmt、公开 rustdoc、20 个 example manifest、69 张视觉基线文件审计通过。
- core 与 registry package 的实际解包编译验证通过；CLI package 使用与 CI 一致的 `--no-verify` 加本地 dependency patch，完整 workspace/CLI release build 验证本地依赖图。没有将 CLI package 的 `--no-verify` 描述为解包验证。
- 完整 macOS release smoke（20 examples、Theme Studio、Gallery、Table 四状态）和 release artifact 审计通过。未把 smoke 或历史视觉文件检查当成新的人工作业验收。

新增正式测试覆盖 owner 闭包、精准路径失效、嵌套 snapshot/rollback/prune、Node/Nodes 槽遍历和相对尺度逆矩阵；README 式使用说明更新在 USER_GUIDE、ADR 0022 与 0.1.8 release note。

新的输出在 `remediation-evidence/`；原始 `baseline/` 和 `SHA256SUMS` 保持不变且校验通过。历史 runner 只在临时工作区执行原探针，旧 async 特征用例继续使用前轮明确记录的 corrected-cancellation runner。

## 新 PR 的边界

- **#100 尚未合并**：仅将 `new_with_policy` 公开不能建立原生窗口 handle 的注册/清理关系。需要单独补齐嵌入窗口的显式 native 接入与所有权，再验证 close/focus、多个嵌入 View、生命周期清理、默认 Disabled policy。此次没有把 constructor visibility 当成能力已经交付。
- **#101 尚未合并**：LTR 改善有价值，但 physical left/right 在 RTL 仍不正确。需逻辑边缘定位和 RTL 几何/拖动/键盘回归，再更新对应视觉基线。
- 未改 #96/#97/#99 的旧 PR head，也没有对其整改完成作出新声明。

发布仍须独立复验本轮提交；开放 PR 的后续取舍与实现不能由这份本地回归绿灯自动替代。
