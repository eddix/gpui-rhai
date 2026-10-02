# Issue #95：Typography mono face、Table 列字体与 inline span 分流评审

输入：[issue #95 快照](../evidence/issue-95.json)。产品基线固定为 `f6e936a509d9aaf75f2e28836bedd439fe83887e`；本地另一会话的未提交修复不计入本结论。GPUI 接口另见 [0.3.7 缓存源码证据](upstream-evidence.json)。[固定提交的源码证据](source-evidence.json)由 `git show` 导出。

## 决策

**a（主题 mono/code 字体）和 b（Table 按列文本样式）是真实且有关联的能力缺口，适合主开发统一设计、独立实现。默认放下一版本 0.1.9；不建议 dogfooding Agent 用指定字体名或仅修改 Rhai Table 的小补丁来关闭 issue。**

技术上可以主动扩展 0.1.8 范围来做 a+b，但它涉及主题 schema、role 白名单、Theme Studio round-trip、NativeCollection 投影和测量一致性，不能作为当前 interaction 整改中的顺手改动。用户已有按行 OpenType feature 的临时做法；#95 属于增强，不应伪装成此次发布新增 P1 回归。

c（span 字体/背景）明确另拆，按原 issue 的 nice-to-have 处理，不阻塞 a+b。背景本身较小，但混合字体涉及 run 边界、继承和排版能力边界，不能只新增两个 builder 方法就宣称完整 inline code。

## 当前能力与真实缺口

| 问题 | 固定基线的实际状态 |
|---|---|
| 全局字体与 fallback | **已有**：`ThemeTypography.family/fallbacks`，Host `ThemeTypographyOverrides`，字体资源 `FontSource`；组件可以显式使用 `style().font_family/font_fallbacks`。 |
| OpenType features | **已有**：`style().font_feature(tag,value)` 校验四字符 tag 与数值，renderer 写入 GPUI text-style refinement，后代文本可继承。 |
| 主题级 code/mono face | **缺少**：`TypographyToken` 仅含 size/line_height/weight；`resolve(role)` 对每个 role 都复制同一全局 family/fallbacks。 |
| 新增 `code` role | **还被双重拒绝**：`ThemeTypography::validate` 和 `Style::typography` 都只允许 8 个固定 role。只增加 Token.family 字段还不能实现 issue 示例。 |
| Table 每列字体 | **缺少**：列 schema 是严格对象，没有 typography/font_features/cell_style；Array 与 native 两条投影都不保留这些字段。 |
| Table 的渲染路径 | 两种来源最终共用 `render_table_row`。当前 body cell 默认 `typography("body")`；row/row_selected part style 被应用到 cell 容器及整行，适合统一行样式，不是按列入口。 |
| span family/background | **缺少公开 API**：Span 只有 text/key/color/bold/italic/motions；renderer 仅构造 color/weight/slant/fade highlights。 |
| 原生底层限制 | gpui-pre 0.3.7 已有 HighlightStyle.background_color、TextRun.font/background，以及 StyledText family overrides；缺口在 gpui-rhai 暴露、继承与 run 构造，并非必须更换 GPUI。 |
| CodeViewer / DiffViewer | **已有原生 monospace 默认与 part override**，内部选择平台候选字体；但它是文档控件自己的 resolver，不等于统一主题 code face。 |

关键位置：

- `theme.rs:220–243,449–511`：类型、严格 role 校验和 resolve。
- `style.rs:1681–1766`：role 白名单和已有字体/feature builder。
- `renderer.rs:4529–4555,4977–4998`：显式字体字段优先于 role；GPUI 字体样式继承。
- `registry/components/table.rhai:46–77,229–253,289–316,535–542`：列 schema、公共 cell 渲染、两种 projection。
- `native_collection.rs:216–227,885–892,1076–1108,1285–1349`：UiValue 边界、TableColumn 严格 decode 与 cell projection。
- `node.rs:122–225`、`renderer.rs:2951–2985`：Span 与当前 highlights 路径。
- `document_view.rs:225–289`：已有平台 monospace resolver。

Issue 对 SF 特定 feature tag 的观测属于应用侧字体验证。本评审确认 feature 能通过 Style/继承链进入 GPUI，但没有重新验证指定操作系统字体文件的 GSUB，不把 `cv08/ss06` 当作跨字体、跨平台保证。`tnum` 与 monospace 也应分别表达：数字等宽不等于所有字符等宽，更不自动保证 I/l/1 易辨认。

## 建议的最小 API 与边界

### a. 扩展现有 typography，不增加另一套主题系统

1. 为 `TypographyToken` 增加可选 family 与 fallback 覆盖，让同一个主题能为 body 和 code 选择不同字体。
2. 增加正式 `code` role，同时修改两个 role 白名单、bundled themes、Host overrides、文档和 Theme Studio 导入/导出。当前 Studio `theme_studio.rs:679–707` 只输出三个 metrics 字段；漏改这里会在保存主题时丢失新配置。
3. 明确定义缺省/继承/清空：建议 role `family=None` 使用明确的角色默认，其他普通 role 可继承全局；fallback 用 `None` 继承、`Some([])` 明确清空，避免 empty 与 omitted 混为一谈。已有 Style fallback builder 不允许空列表，若支持显式清空，需要对齐 resolver 与 renderer 的语义，不能只改 serde。
4. **code 默认必须真正有 mono 策略**。不能把 `family=None` 一律当成全局正文 family 后又声称默认 code 是等宽。集中复用已有平台 mono 解析作为默认策略，允许主题/Host 替换；禁止在各 Table/Span/脚本里硬写某一个平台字体名。不应凭空使用 GPUI 未实现的 CSS generic-family 名字。
5. `ResolvedTypography` 是统一解析结果。显式 `style().font_family/font_fallbacks/font_weight` 仍优先；新角色不应绕过现有 Host/window/subtree theme 作用域。

角色扩展采用一次明确的 pre-1.0 schema 更新即可；不需要双轨旧 schema 或历史兼容垫片。字体加载依然由 Host `FontSource` 管理，不新增脚本字体下载或文件读取权限。

CodeViewer/DiffViewer 的默认 mono 应在这次设计中给出统一归属：可以消费同一个 code face resolver，同时保留显式 text/gutter part overrides。至少不能新增第二个相互竞争的默认 mono resolver。它们是否同步改造可由本次实现范围明确决定，不能让文档暗示已经统一而源码仍各取自己的默认。

### b. Table 初版优先采用可序列化的列文本配置

建议最小字段：

```rhai
#{ key:"pod", title:"Pod", width:#{kind:"flex",value:1.0}, typography:"code" }
#{ key:"count", title:"Count", width:#{kind:"fixed",value:100.0},
   align:"end", font_features:#{tnum:1} }
```

- `typography` 引用已有主题 role；`font_features` 复用 Style 相同 tag/value 校验。
- 明确字段只影响 **body cell 的主文本**；header 保留其 header role，adornment/Badge 保持自身语义样式，不应无意变成等宽代码外观。
- Array 的 `table_row_item`、Rust 的 `TableColumn::decode/project_row` 与公共 renderer 一起更新。NativeCollection 必须保留纯数据投影、虚拟化和无逐帧 Rhai 计算的边界。
- `cell_style:style()` 可以留作更一般的后续 API；如果选择立即支持，不能把 Style 对象原样送进 `native_table_view`。该配置先转为 `UiValue`，不是可传任意 Rhai 自定义对象的通道。应把 typed style 留在渲染配置并按稳定 column key 查找，或有明确的受限文本样式数据结构；不要让 Rust projection 持有 GPUI/Rhai runtime 对象。
- 不应让任意 cell_style 改写列的宽度、flex、min-width、resize 或虚拟行布局。若采用 Style，应限制为文本能力并明确错误处理。

**测量是同一功能的一部分。** 当前 `IntrinsicTextMeasurePrimitive` 在 `column_resize.rs:405–426` 读取继承的 window.text_style，而可见主文本与 measure 是 siblings。若只给 `text(cell.text)` 设置 mono、不给 measure 相同 resolved text style，auto-fit 仍按正文宽度测量。应让两者共享字体/feature/metrics，header 测量保持其自身 header 字体。

### c. Span 单独设计；不要误判上游 helper 的能力

gpui-pre 0.3.7 的 `StyledText::with_font_family_overrides` 会在 layout 时沿用 parent TextStyle，但 **不会自动拆开已有 runs**：只有一个完整 run 落在 override range 内才改 family。当前 gpui-rhai 还会过滤 default highlight，因此一个只有 family 变化的中间 span 可能没有自己的 run。直接添加 family override 可能编译成功却不生效。

实现必须建立完整、相邻、不重叠且覆盖 UTF-8 全文的 run 边界，并保留 parent 样式、显式 bold/italic/color、背景、fallbacks、features 和 selection/copy 语义。

最小 span 扩展应先限定为 font-face 与 background；若引用 code role，要明确这是字体属性解析，不能暗示已支持同一行内任意 role size/line_height。当前 GPUI TextRun 携带 Font/颜色/背景，并不携带每 run 的 font-size/line-height，直接暴露完整 `span.typography("code")` 会产生契约歧义。最终命名和支持边界应在独立规格中确定，而非 dogfooding 临时拼接。

## 验收建议

1. 两个不同主题为 body/code 配置不同字体；切换主题时普通文本、单列 code、离屏后重新实现的行都更新，明确 Host overrides 与 subtree 主题优先级。
2. 相同数据分别走 Array/NativeCollection，只有指定 body column 换字体，header、其他列、Badge、selected/hover 状态不变；源码样式值测试不足，检查实际文本 run/绘制宽度。
3. Table auto-fit、截断、复制、排序、选择、滚动和虚拟化不退化。相同字体/feature用于可见文本和 intrinsic measurement；主题/font generation 变化后相关缓存失效。
4. Theme Studio 导入→编辑→导出→重载保留新 family/fallback/role 字段。非法 role/tag/value、重复/超限 fallback 给出既有风格的有界诊断。
5. 使用 Host 提供的不同字体以及缺失字体、CJK fallback 验证。不要把某一机器恰好安装特定字体当作产品验收前提。
6. span 后续独立验证中间一段 mono、相邻不同字体、仅 family 没有 color/bold 的 span、多字节字符、换行、背景与 selection 叠加、复制原文和 byte range 边界。

## 本次评审范围

只读源码和固定 issue 快照，未执行大编译、未修改产品、未切分支、未提交或对 GitHub 发言。设计建议不等于实现已经通过验收。主报告统一确定排期；默认建议 a+b 作为下一版本主开发增强，c 另拆。
