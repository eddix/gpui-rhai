# 050c7fa2 复验：原缺陷通过，SVG 适配仍有四项缺口

日期：2026-09-21。基线：`050c7fa2c98e0104c2afd687450f388893cf7566`，`Close the 6373e4b3 issue-fix audit gaps (#66)`。开始时工作树干净。

**上轮 I01–I04 的具体复现全部通过，可以关闭那些原始缺陷。** 新增 group 切换、虚拟项后续实现的检查也通过了。不过 SVG 适配器尚不能整体验收：本轮发现 **3 个运行验证的问题，以及 1 个源码确认的无界缓存问题**。其中异步解码把重工作放回了前台、SVG 文字消失，是本次实现带来的回归；SVG 局部 color 优先级是进一步发现的语义缺口。

本轮未修改产品代码，只新增报告、测试和证据；未创建或修改 GitHub issue。历史 API 兼容不在缺陷列表中。

## 验证结果

| 检查 | 本轮结果 |
| --- | --- |
| Workspace，all-targets/all-features，locked/offline | **509 通过，0 失败** |
| 产品 native-keyboard | **64 通过，0 失败** |
| 上轮独立原生测试 | **9/9 通过**，包括四个原失败用例 |
| 新增原生扩展测试 | **2 通过、2 失败**：两种 group 更新通过；SVG 字体、局部 color 失败 |
| performance 默认套件 | **2 通过、2 ignored** |
| fmt / 严格 workspace Clippy | 通过 |
| Release SVG drain 微基准 | 完成，单张 2048² SVG 前台 drain 中位耗时约 **18.06 ms** |
| 当前提交 CI | [CI run 35588566657](https://github.com/eddix/gpui-rhai/actions/runs/35588566657) 成功 |

R = 独立程序运行证据；S = 源码调用链确认。前三项发现有 R/S 证据；缓存项仅以 S 标记，没有声称已复现 OOM。测试平台没有被当成物理屏幕或完整应用帧率认证。

[复现入口](reproduce.md) · [新原生测试](native-probes.rs) · [release 测量程序](drain-bench.rs)

## J01 · P1 · R/S：异步 SVG 解码的重工作在前台 drain 执行

定位：[asset.rs:673](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:673)、[asset.rs:695](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:695)、[asset.rs:837](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:837)、[asset.rs:902](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:902)；生产调用点 [app.rs:4462](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/app.rs:4462)。

`start_image_decode` 的 worker 仍只对 SVG 检查 UTF-8 和 `<svg`，返回原始 bytes。新增的 usvg 解析、resvg 光栅化、PNG 编码在 `install_decoded_image → prepare_image → svg_image` 中执行，而这条路径由前台 `drain_image_decodes` 调用。App 处理异步投递时直接调用它，没有另一个后台阶段。

这改变了原来的线程边界：旧版本此处安装原始 Image bytes，GPUI 的 ImageDecoder 后续由 `App::fetch_asset` 的 background executor 执行。现在在交给 GPUI 之前，主要 SVG 工作已经在前台完成。

独立 release + thin LTO 测量，Apple M1、每个尺寸 3 次冷缓存，输入仅一个双色渐变矩形，无字体、滤镜和外部资源：

| SVG 尺寸 | start_image_decode 中位耗时 | 前台 drain 中位耗时 |
| --- | --- | --- |
| 256×256 | 0.015 ms | 0.364 ms |
| 1024×1024 | 0.017 ms | 6.015 ms |
| 2048×2048 | 0.022 ms | 18.064 ms |

这是公开异步 API 的实测，不是 debug 构建的耗时外推。`drain` 会处理本次已经完成的全部消息，多张图的安装成本还会叠加。缓存命中不能消除首次加载和颜色变体首次生成的前台工作。`render_inline_svg` 和 `image_source_tinted` 的缓存未命中也直接执行同一光栅化函数。

**建议修复：**把解析、光栅化和 PNG 编码放到受管理的后台任务中，前台只安装已准备好的结果和投递 callback。保留 generation、scope、取消和失败回滚语义；worker 携带 owned bytes / 色彩 / 渲染配置，不携带 Rhai Engine、FnPtr 或 GPUI 实体。同步 preload 若保留，应与交互期异步接口的保证明确区分。

**回归条件：**大 SVG 和多张同时完成时，前台 drain 成本主要随安装/投递数量变化，而不随图像像素数增长；过期/取消任务不能发布结果。另测 inline 与 tint 的冷缓存路径，不能只把一个入口搬到后台。

微基准没有包含真实 Window 帧调度，因此没有把这组数据直接换算为应用 FPS。证据：[drain-bench.log](evidence/drain-bench.log)。

## J02 · P2 · R/S：新的 usvg 默认配置丢失字体环境，SVG text 消失

定位：[asset.rs:919](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:919)。

原 GPUI 0.2.2 的 `SvgRenderer` 带有字体 resolver，会按需载入系统字体。新适配器每次创建 `usvg::Options::default()`，没有继承这份字体数据库/解析配置。含 `<text>` 的 SVG 可以解析成功并生成 PNG，但文字已经被丢掉，没有传播成错误。

同一份 160×40 SVG，包含 Arial、24 px 的 `Readable` 文本：

- 直接使用项目锁定 GPUI 的 SVG 路径：**893 个非透明像素**；
- 经新的 AssetRegistry PNG 适配器：**0 个非透明像素**。

控制组和适配组运行在同一个 TestAppContext、同一台机器，已排除机器没有字体的问题。黑色文字也不受旧红蓝通道问题干扰。

**建议修复：**完整迁移原 SVG 渲染器的字体环境，或复用等价的、可传给后台工作的 renderer 配置。不能只迁移“SVG→Pixmap”调用而使用不同的解析环境。字体资源与缓存失效规则也应纳入渲染 key / 生命周期设计。

**回归条件：**系统字体、generic font family、字体回退、SVG text 与普通 path 混排；inline 和 asset-backed 路径都要验证文字确实产生像素。项目支持的 Host 字体配置是否参与 SVG，应明确约定并验证。

证据：[native-probes.log](evidence/native-probes.log) 中 `svg_text_survives_raster_adapter`。

## J03 · P2 · R/S：全局替换 currentColor 仍覆盖 SVG 自己的 color

定位：[asset.rs:945](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:945)。

通道和 alpha 已修好，但 `source.replace("currentColor", …)` 仍然把该关键字当作全局占位符，跳过 SVG 自身的颜色继承。SVG 的 currentColor 应由元素的 color 属性提供间接值，外部颜色作为默认继承上下文不应覆盖内部显式 color。[SVG 2：color 属性](https://www.w3.org/TR/SVG2/painting.html#ColorProperty)

测试文档含：

```xml
<rect width="1" height="1" color="#ff0000" fill="currentColor"/>
```

- 不传外部 tint：BGRA `[0,0,255,255]`，正确红色；
- 外部环境为绿色：BGRA **`[0,255,0,255]`**，内部明确声明的红色被改成绿色。

这不是上轮固定 fill 的通道错误，而是 currentColor 的解析位置错误。它在旧的字符串 tint 中也存在，本轮扩展检查才给出明确复现。

**建议修复：**把外部 RGBA 注入为文档的默认继承颜色，让解析器处理内部 color、style 和 opacity。保留局部覆盖与原有 alpha 组合。如果项目另需强制单色重着色，应做成显式不同的操作，而不要把它隐含在环境继承中。

**回归条件：**无内部 color、根 color、子元素 color、style 内 color、局部覆盖与透明度组合；外部环境变化只影响真正继承它的部分。

证据：[native-probes.log](evidence/native-probes.log) 中 `svg_document_color_overrides_inherited_tint`。

## J04 · P2 · S：新增 inline SVG 缓存没有容量或生命周期回收

定位：[asset.rs:245](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:245)、[asset.rs:801](/Users/eddix/Codes/github.com/eddix/gpui-rhai/crates/gpui-rhai/src/asset.rs:801)。

`inline_svg_images` 用 `(完整 source String, RGBA)` 作 key，并强持有生成的 PNG Image。当前代码只有查询和插入，没有淘汰、字节预算、代次回收或存活场景回收。窗口/组件取消只处理 pending decode，namespace refresh 也没有清理 inline 表。

因此，一个仍合法的动态 SVG 节点持续改变 source，或原生色彩信号持续产生新 RGBA，都会累积历史 cache entry；旧节点不再呈现也不会让这些 entry 消失。复用 AssetRegistry 的窗口和热重载会延长这些历史条目的存活时间。

16,384 的边长限制和 16,777,216 的单图像素限制只约束单次光栅化，不能约束缓存总量。新增缓存按历史不同组合数增长，而不是按当前呈现树的大小增长。完整 source 自身也占据缓存内存，不只是 PNG bytes。

**建议修复：**引入总字节预算及淘汰/存活引用策略，核算 source 与图片成本，并定义跨 view 共享、热重载和取消任务的行为。应允许当前使用中的图像继续被其使用者持有，同时使不再需要的历史 key 可被回收。补充命中、miss、字节数、eviction 等可观测数据。

**回归条件：**单个节点反复替换 source/颜色、卸载后继续运行、重复热重载，缓存应稳定在明确上限内；共享中的图像不能提前失效。

这一项由字段及全部访问点确认，见 [cache-references.txt](evidence/cache-references.txt)。本轮没有运行高内存/OOM 压力测试，也没有给出未经测量的实际内存增长数字。

## 上轮验收状态

| 原问题 | 本轮结果 |
| --- | --- |
| I01 外层 motion_group 在子组件增量更新后丢失 | 原用例通过；新增父 group a→b 再更新子组件，通过；虚拟行后续实现及 group 切换也通过 |
| I02 SVG 固定颜色红蓝交换 | 原最终 BGRA 像素断言通过；固定色与渐变测试通过 |
| I03 语义色 alpha 丢失 | alpha=0、0.5 的产品/原生回归通过 |
| I04 独立 Host 的 resident view 缺少 container | 同 Host、不同 Host、显式 container 对照全部通过；产品测试同时验证 resident 仍呈现 |

这些原始问题可以关闭。J01/J02 是扩大适配器职责后引出的线程/字体回归，J03/J04 是继续检查语义和容量边界得到的发现，不应把它们混写成“上轮修复没有发生”。

## 后续实施建议

建议将下一步集中为一份完整 SVG 适配契约：**渲染环境（含字体）＋颜色继承＋后台执行＋有界缓存**。首先修复 J01/J02，随后处理 J03/J04；保持已经通过的 group replay 和 HostSlot 边界实现。

不要仅以缓存命中或 1×1 不透明色块代表整个适配器正确。保留本轮像素对照和异步 drain 测量，并加入长期更新下的缓存容量断言。#60 与 #65 的完整接入场景本轮没有新的外部复现资料，继续沿用上一轮的范围界定，不将它们擅自关闭或扩大归因。
