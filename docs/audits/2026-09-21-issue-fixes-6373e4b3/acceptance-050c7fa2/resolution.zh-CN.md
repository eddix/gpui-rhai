# 050c7fa2 SVG 扩展审计整改记录

本文件记录 J01–J04 的产品整改。原报告、探针和证据保持不变。

## 结果

- J01：`start_image_decode` 的 worker 现在完成 SVG 解析、字体解析、光栅化、PNG 编码和 `Image` 准备；foreground drain 只安装准备好的结果。原 release 测量程序在同一台 Apple M1 上得到 256/1024/2048 三档中位数约 0.001/0.006/0.010 ms，2048² 原值约 18.06 ms。
- J02：适配器采用与 GPUI 0.2.2 `SvgRenderer` 相同的延迟系统字体数据库、默认字体选择器和 fallback 选择器，并将 generic family 映射到当前平台实际存在的 Arial/Helvetica/DejaVu/Liberation/Noto 候选。原 Arial 探针恢复为 893 个可见像素；产品原生测试另覆盖缺失命名字体到 generic `sans-serif` 的回退及 path/text 混排。
- J03：外部颜色不再替换所有 `currentColor` 文本，而是作为 SVG 根元素的继承默认值注入。元素或祖先的 `color` 属性/内联样式继续按 SVG 级联覆盖外部值。若所有 `currentColor` 已由文档内部解析，asset-backed 图像直接复用预载基图。
- J04：inline 与 tint 共用后台 SVG variant cache。默认上限为 256 项、128 MiB，字节数包含源文本和 ready BGRA 像素；LRU 淘汰会取消未发布工作，token 阻止旧结果回填。namespace refresh 和 registry drop 会清理或取消变体。`AssetRegistry::svg_cache_stats()` 暴露 entries/bytes/limits/hits/misses/evictions。

## 边界

- 声明式 preload 仍是同步准备期 API；交互期 inline/tint miss 与公开异步 decode 不在前台执行重工作。
- SVG text 使用 GPUI 0.2.2 的系统字体数据库。Host 通过 `FontSource` 注册的内存字体目前只进入 GPUI 文本系统，不会隐式进入 SVG 的独立 `usvg` 字体库。
- 已在当前帧取得的 `Arc<RenderImage>` 不受随后淘汰影响；超过预算且持续呈现的工作集会按 LRU 重新生成，而不会突破硬上限。

## 定向复验

- 原扩展原生探针：4/4 通过。
- 产品 SVG 单元测试：10/10 通过。
- 产品原生 SVG 测试：BGRA、alpha=0/0.5、内部 color 优先级、系统字体与 fallback 通过。
- 原 release drain 测量程序：前台成本不再随 256→2048 像素规模增长到毫秒级。

上轮总报告中的历史 9 项 runner 有两项把公开返回值结构硬编码为
`ImageSource::Image`；外部 `currentColor` miss 现在有意返回异步
`ImageSource::Custom`，因此这两个旧 probe 会在像素断言前停止。其 BGRA 与 alpha
断言已等价迁入产品原生窗口测试并通过，其他 7 项历史 probe 继续通过。审计原件
未为适应新实现而改写。
