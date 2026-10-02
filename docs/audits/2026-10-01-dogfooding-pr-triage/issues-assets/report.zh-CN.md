# Issues #98 / #83 / #89：资产刷新与交互增强分流

静态审查固定基线：`f6e936a5`。Issue 正文来自本轮 `evidence/issue-*.json`。
期间其他会话正在修改产品文件；本报告未修改这些文件。缓存探针链接 live workspace，仅刻画未变化的 AssetRegistry/CapabilityHandler API；三份相关源码与基线逐字节一致，记录于 [source-snapshot.json](source-snapshot.json)。不把这个 probe 当作其他 PR 或整份工作树的验收。

## 建议

| Issue | 性质 | 0.1.8 建议 | 后续归属 |
|---|---|---|---|
| [#98](https://github.com/eddix/gpui-rhai/issues/98) 可变图片刷新 | 确实缺少便捷 app/view 单资产刷新契约；“没有任何公开刷新路径”不准确 | 补 Host 前台桥接说明和版本化资产示例；不作为出货阻断，不必为了该场景立即新增全局 Rhai invalidation API | 核心资产维护者设计 per-asset revision/publish；Cadenza Host 负责写文件、完成消息和前台调度 |
| [#83](https://github.com/eddix/gpui-rhai/issues/83) 自定义 resize grip | 新的视觉 slot、命中与状态投影能力 | 保持独立设计，按 issue 建议排 0.1.9 | 交互 Runtime + registry 组件维护者共同实现 |
| [#89](https://github.com/eddix/gpui-rhai/issues/89) Table 行/单元格 context request | 真实缺少公开事件；菜单锚点部分已有入口 | 独立 Table 增强 PR，建议 0.1.9；不混入 0.1.8 手势修复 | Table/registry 维护者；先依赖虚拟回调生命周期修复 |

## #98：已有能力与真正缺口

### 已有公开路径

`AssetRegistry::refresh_namespace` 是 public（`asset.rs:626`）。它重新读取已缓存的 provider 条目，保留 `ImageHandle`/opaque handle 身份，替换其底层 `Arc<gpui::Image>`；SVG 变体缓存随之清理。

Host 并非拿不到 registry：`ScriptViewExtension::configure_runtime(&mut UiRuntimeState)` 是公开扩展点（固定基线 `app.rs:1363`），其中 `runtime.assets` 可 clone。受信任的前台 `NativeHandler` 也直接获得 `&mut UiRuntimeState`、Window 和 App（`native_handler.rs:133`、`:171`）。

关键线程区别：

- 同步 `CapabilityHandler` **没有 Send 约束**（`capability.rs:77`）。在 configure_runtime 注册一个持有 AssetRegistry clone 的小型同步服务，是合法现有方案。
- 只有后台 `TaskWork` closure 要求 Send（`:87`）。把 `Rc<RefCell<AssetRegistryInner>>` 搬进 TaskWork 编译失败，是正确的线程边界。
- 写音频/封面文件的后台工作应返回 AssetId、成功状态和应用修订号，在完成后的前台处理阶段刷新。Rhai 完成回调可调用已有 `ctx.call_capability(...)` 同步桥接；不要让后台线程直接碰 Runtime。

### 缓存层与重绘

1. `by_asset: AssetId -> ImageHandle` 是逻辑 ID 缓存。`load_image` 在命中时直接返回，不会读 provider。
2. `images: opaque id -> Arc<Image>` 保存当前内容。刷新时替换 Arc，因此旧帧已克隆的内容仍可安全完成，新帧再解析同一个 handle 时拿到新内容。
3. GPUI 0.3.7 的 `Image::from_bytes` 以 bytes hash 生成 image id；刷新后 bytes 不同，GPUI 缓存身份也变化。无需要求应用更换公开 opaque handle，也不应原地修改旧 Image 而保留错误的内容 id。
4. SVG tint/inline 另有有界 variant cache，refresh_namespace 会 clear/cancel 相关变体工作。
5. **缓存变化不会自己安排窗口呈现。** Runtime renderer 每次 render 通过 `cached_image -> image_source_tinted` 取当前 Arc（固定基线 `renderer.rs:3284`），Host 还需通知重绘。NativeHandler/Host GPUI 前台入口可调用 `cx.refresh_windows()`；这会使 GPUI cached views 也重新渲染。只刷新当前 window 不能保证其他共享 registry 的窗口马上更新。

文件开发模式的 `reload_assets` 已将 `refresh_namespace("app")` 和 `mark_all_windows_dirty()` 配对（固定基线 `app.rs:5486`）。这是 `dev-reload` 下的文件工作流，不是通用 production 文件监控服务，也不会自动观察任意应用封面目录。

### 不能当作刷新入口的函数

`ctx.start_image_decode` 也不是已缓存图片的 reload：它重新读取并准备数据，但 `install_decoded_image` 发现同 AssetId 已缓存就返回旧 handle，放弃安装新的内容（`asset.rs:1128`）。因此“再异步 load 一遍”不能替代失效/刷新。

### 五项公开 API 探针结果

全部断言通过，见 [probe-results.log](probe-results.log)：

1. 修改 provider 内容后再次 `load_image`，仍取得旧 GPUI image id。
2. 对同一个已缓存 AssetId 调用 `start_image_decode` 并完整 drain，仍保留旧 image id。
3. 用实际包含 AssetRegistry Rc 的同步 CapabilityHandler 执行 refresh，opaque handle 不变，而 GPUI content id 改变。该 probe 直接调用 handler 的公开 trait 方法；没有声称验证完整 capability 注册、真实多窗口像素或重绘时序。
4. namespace refresh 接受了 MIME 为 PNG、非空但实际不是 PNG 的 bytes。这条入口目前只做格式边界/空数据等验证，raster 实际解码仍在 GPUI 之后发生。
5. 对尚未安装、仅有 pending decode 的 asset，refresh_namespace 返回 0；其旧 decode 随后仍可安装旧 bytes。这个行为符合“仅刷新已缓存条目”的函数范围，不能拿来实现更强的全量失效协议。

第 4/5 项用于界定现有 API 能力，不在本次 triage 中冒充新发现的 release 阻断。但它们说明：如果新增公开的强一致 `refresh_asset`，不能只把 refresh_namespace 换一个名字暴露给 Rhai。

## #98 当前最小接入方式

0.1.8 优先提供两种明确模式：

- **内容不可变、版本化 ID**：Cadenza 已验证的方式可以继续使用。修改内容后使用新逻辑 ID，并由应用管理文件/版本保留策略。不要把它宣布成唯一永久方案；AssetRegistry 的逻辑图片映射没有按此版本流自动淘汰旧项。
- **稳定逻辑 ID、Host 前台刷新**：configure_runtime 中保留 registry clone 或注册专用同步能力；后台写入完成后回到前台，刷新允许的 namespace，再通过 Host 的 GPUI 前台入口安排共享窗口重绘。同步 namespace 刷新可能重读并处理多个资产，应说明成本；它适合小范围受控集合，不应被包装成无成本后台刷新。

如果只用同步 capability，能刷新缓存，但该 handler 没有 App 参数，不能独自完成所有共享窗口的刷新。单窗口任务回调后通常已有当前 view 通知；需要可靠跨窗口同步时，应使用 NativeHandler/Host foreground task 配对 `App::refresh_windows`。文档应分别说明这两层，不能声称调用 cache 方法就自动刷新全部窗口。

因此，**新增框架 API 不是当前应用解除阻塞的必要条件**。本轮应由框架维护者补准确文档/小例子，Cadenza Host 决定继续版本化，或接上已有前台桥。不要把 AssetRegistry 改为 Send/Sync，也不要给 Rhai 增加任意路径、URL 或磁盘 watcher 权限。

## #98 后续最小合理 API

若将稳定 ID 更新作为正式公共能力，建议独立设计 per-asset refresh/publish，而非先暴露 namespace 全清。可采用以下接口层次，名称仅为提案：

1. 核心 `AssetRegistry::refresh_image(&AssetId)` 或 prepare/publish 对，定义是否允许首次加载、失败时保留 last-good、是否真的发生内容变化，并保持 opaque handle 稳定。
2. app/view 层 `ScriptViewHandle::refresh_image_asset(...)` 将刷新结果、dispose/suspend 行为和重绘配对。该接口应明确影响共享同一 registry 的窗口，而不是假设所有 ScriptViews 都共享缓存。
3. 每个资产有内容修订/token；旧 decode、旧刷新完成消息不能覆盖更新的内容。脚本 generation 只防热重载陈旧回调，不能代替同一 generation 内的资产内容版本。
4. 重读/解码完成前保留可显示旧图；候选 MIME、非空、实际解码、资源限额都通过后才提交。原 AssetProvider 没有 Send 保证，不能声称任意 provider.load 都可以直接放后台；需要通过受信任 Host 的后台准备数据或显式 async provider 边界实现。
5. 只允许已注册逻辑 AssetId/namespace，继续保持 provider root 限制。无需暴露文件路径、GPU 对象或 RenderImage 句柄给 Rhai。

验收至少包含稳定 ID/handle、真正像素变化、损坏文件保留旧图、两次刷新反序完成、pending-only decode、同 registry 双窗口、独立 registry 隔离、暂停恢复、dispose、SVG tint 和 unchanged 内容不做多余重绘。

这套闭合能力值得做，但它超过一个方便函数；建议作为下一轮核心资产工作，不用它拖住当前 0.1.8。

## #83：保留独立 grip 设计

缺口成立：SplitPane 的 handle 直接是 SplitResizePrimitive（`split_pane.rhai:141`），布局顺序仍是 start/handle/end（`:160`）；Resizable 也只有统一 handle style，没有自定义视觉 node slot。native line 与 hitbox 按自己的 bounds 绘制，原 issue 中的 overhang、绘制次序、命中范围和状态问题不能靠任意 child node 自动解决。

建议遵从 issue 的 0.1.9 目标，由 Runtime 与组件维护者共同设计：视觉 slot 与 native gesture/focus/separator 语义分离；装饰伸出的部分有明确 hit region；hover/drag/focus/disabled 沿 native 状态驱动，不能每次 move 重新执行 Rhai；间距、圆角和线条 inset 遵循 token。状态可能同时成立，不能把 focus 与 hover 等简单当作互斥业务状态。不要为了一个 grip 在 0.1.8 顺带引入通用 z-index/任意 portal。

## #89：Table 事件缺口成立，Menu 锚点已有

Table 当前公开事件没有行/单元格右键请求；行仅在 `table.rhai:275` 接 `row_action`，调用者无法直接把每个内部 cell 包成 ContextMenu。不能用“外面包住整个 Table”当作等价方案，因为那样没有稳定 row key 与 column key。

但 Menu 已有公开 `anchor {x,y,width?,height?}`（`menu.rhai:39`），并原样传入 overlay config（`:210`）。后续 Table 只需产出清晰的语义请求，由应用控制现有 Menu，无需新增一套菜单平台机制。

建议独立 PR 定义 `row_context_request`：row key、可空 column key、window logical-pixel anchor，以及 pointer/keyboard 来源。行背景或键盘触发不一定有 column，不能强行填某一列。右键不应同时触发左键 row_click；selection 是否改变需明确。Array 与 NativeCollection 共用相同出口，不发送全量 row 数据或 stale 数字 index。键盘 ContextMenu/Shift+F10 可作为同一契约，但必须验证实际焦点行及已呈现 bounds。

优先完成 0.1.8 当前虚拟行 callback/生命周期缺陷，再做这项增强；否则会出现首屏 context menu 正常、滚动后失效。它适合 0.1.9 的单独 Table 增强，不应和 #83 或 #98 捆绑成大型组件改造。
