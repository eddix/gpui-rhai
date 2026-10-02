# 嵌入窗口、Table 边界与节点样式存储整改

日期：2026-10-02。基线为已通过远端 CI 并合入的 PR #102（main `c088e96f`）。产品提交：`f7296538`；保留了 PR #101 作者的原始修复（本分支 `32b7efae`）。本记录是实施回归，不替代独立验收，不发布 0.1.8、不创建 tag。

## 1. #100：完整原生窗口接入

没有将 policy constructor 单独公开当作能力完成，而是提供 `PreparedScriptView::mount_window`：

工作期间又出现 PR #103（独立的 register_native_window 提案），根因和本工作包一致。其“先给 policy、再手工填 native registry”需求由同一个显式 mount adapter 完整覆盖；不额外暴露可覆盖 registry alias 的第二条接入路径。

- 普通 `mount` 继续使用 Disabled policy；同 Host 的其他嵌入 View 不继承窗口权限。
- 显式 adapter 同时建立逻辑 WindowRecord、真实 AnyWindowHandle、close interceptor 和 command-owner lease。Rust 仍拥有原生 root/layout。
- 命令 owner 按真实 GPUI WindowId 唯一，不能通过另建 Host 别名绕过；Host 本身也不能绑定另一个原生窗口。
- App 索引只持有弱 lease；失败挂载、dispose 和 native close 释放权威。dispose 不关闭 Rust 的窗口，替换 owner 可以重挂。
- 所有 View 都观察 native close，即使应用保留 ScriptViewHandle，也进入 Disposed 并清理资源。回调只保留 WeakEntity，释放更新经 defer 离开当前栈。
- focus 与 close 共用延迟 native operation 路径，修掉同窗 focus 的 GPUI 重入。成功请求表示已排队，不声称 native 操作已同步结束；异步 native 错误仍进入 last_error。

接口文档明确：选择 mount_window 会替换原先的 should-close callback。Rust 若保留该策略，应使用普通 mount + 应用自己的 Rust capability/callback，而不是让 SDK 隐式接管。

五项正式原生测试覆盖真实失焦/激活、open/close child、关闭确认、默认拒绝、同 Host/跨 Host owner 冲突、dispose/replacement、init 失败回滚、错误 native window 绑定、Rust 直接关窗及外部 retained handle 清理。

## 2. #101：逻辑列边界

保留作者的绝对定位和末列支持，补齐共享 `inset_start` / `inset_end` 样式。它们接受 definite/auto/signed offset，经现有方向解析统一映射到物理边；没有给 Table 单独添加方向分支。

内侧手柄位于下一列的逻辑 start，测量/提交仍属于前一列；末列/单列手柄位于自己的逻辑 end。手柄具有独立稳定 node key/ref，测量 ref 仍绑定对应 header。Host 可以聚焦真实手柄后走原生键盘事件。

七项新增正式原生测试覆盖 LTR Array、RTL 几何、RTL NativeCollection 拖动、末列原生 autofit、单列、不可调整邻列与末列 keyboard，以及聚焦实际 ref 后的物理 Left/Right 方向。原 PR 的三列 LTR 布局/拖动测试仍保留。LTR/RTL 实测边界误差在 0.5px 内，resize/autofit 不误触发 sort。

## 3. 默认线程栈阻断与共用样式模型

逻辑样式字段扩展后，完整 Gallery 的 viewport/locale matrix 在默认 native test 线程栈上稳定复现 stack overflow。仅扩大栈的独立 characterization 能通过，说明旧的 UiNode 内联大型 Style 已到栈压力边界；没有将扩大栈作为验收设置。

UiNode 的 Style 改为私有 immutable Rc snapshot：默认空样式每线程共享，样式合成和 ghost 修改走 copy-on-write。公共 `style() -> &Style` 和 Style DSL 保持值语义；节点 clone 不再携带大型 Style 结构穿过每层调用帧。

实测 **UiNode=592 bytes，Style=5680 bytes**。默认线程栈下原失败 Gallery 用例以及全部 154 项产品 native 测试通过。正式回归验证 clone 的样式共享、修改隔离、owned snapshot 及 ghost 不污染原节点；ghost 同时清掉物理和逻辑 inset，避免几何域混用。

## 验证

- workspace all-target/all-feature **616 项**，core all-feature **480 项**；default core **450 项**。
- 独立 native workspace **154 项**全部通过，未为正式测试设置 RUST_MIN_STACK。
- workspace/native 严格 Clippy、fmt、公开 rustdoc、20 example manifest、performance structure（2 passed，3 显式 ignored 可选性能测量）通过。
- core/registry 实际 package 解包编译验证通过；CLI 使用既有 `--no-verify` + local dependency patch，workspace/release CLI build 验证本地依赖图。
- 完整 macOS release smoke 与 artifact 审计通过。其输出及 stack characterization/回归结果在 `evidence/`。

## 尚未完成的视觉步骤

尝试刷新五张 Data Table 基线时，测试窗口被桌面窗口管理器放置为 **825×1393**，不是所需 **980×752**。应用确实可见、AX 与截图均正常；这不是“屏幕未解锁”的证据。尺寸不符的图片没有被缩放伪装成新基线，旧 PNG 未修改。

已向用户询问是否临时切换该测试窗口为 floating。固定 viewport 截图刷新仍待执行；原生几何/鼠标/键盘验证不能冒称完成了视觉基线验收。

`USER_GUIDE`、embedding/multi-window 文档及 0.1.8 release note 已同步。用户的 `2026-10-01-dogfooding-pr-triage` 目录保持未改、未提交。发布仍等待独立验收。
