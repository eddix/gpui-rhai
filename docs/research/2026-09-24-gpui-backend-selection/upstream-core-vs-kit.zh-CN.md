# 上游 GPUI、gpui-pre 与 Kit/Base：收益拆分与现行建议

日期：2026-09-24。对照 gpui-rhai 0.1.5 / `120ef223`。

## 1. 现行建议

用户已明确：3D 引擎可以由有需要的 Rust Host／扩展作者实现，gpui-rhai 不必内置它。因此当前应优先选择**官方上游 GPUI 路线，通过 gpui-pre 获取较新的可追溯快照；不以引入整个 gpui-kit／gpui-base 为升级前提。** 官方完整 crate 家族恢复及时发布后，再验证并切换发行来源。

我前面把两件事绑定得过紧：

1. 从旧 GPUI 0.2.2 更新到底层新能力与修复。
2. 采用 Kit/Base 的控件、输入和应用行为体系。

第一项对我们有明确价值；第二项需要按具体模块判断。gpui-rhai 本身已经是 Runtime 和组件体系，不是一个尚缺基础控件的终端应用。保持状态、主题、生命周期的单一权威，是效果与架构边界的判断，不是为了节省迁移工作量。

**Kit/Base 仍然有价值。** 将来若需要更完整的编辑器、富文本、Dock 等，可以单独评估是否采用对应基础能力。直接使用 gpui-pre 不会永久关闭这条路线。

## 2. 三层分别新增了什么

| 层 | 实际内容 | 对我们的意义 |
|---|---|---|
| Zed 最新 GPUI 核心 | 新语义 API、平台／窗口／文字／帧调度能力、底层 bug 修复 | 这是大部分本次底层升级收益的来源 |
| gpui-pre | 将指定 Zed commit 重打包成可发布、对齐版本的 crate 家族 | 获取上游新代码的发行渠道，不是额外一套控件行为 |
| gpui-kit／gpui-base | 对齐依赖与入口；Base 的 Input/Editor/TextView、焦点／弹窗／选择／虚拟化、系统偏好等；可选 Component 外观层 | 只有实际采用这些模块，才获得相应额外收益 |

`gpui-kit` 不是“只把 GPUI 升了个版本”，但也不能把 Zed 原生已有的能力都归功于 Kit。`default-features=false` 关闭 Kit 的 styled Component 和默认 assets，**并不会移除其必需的 gpui-base 依赖**。纯底层需求可以直接消费 gpui-pre 家族。

源码：[Kit crate](https://github.com/longbridge/gpui-kit/blob/9765ae2c9a5eccfa13891248a445991e6f6a09d8/crates/kit/Cargo.toml)、[Base 架构](https://github.com/longbridge/gpui-kit/blob/477a8d90bb318cfbf0bbdc0f29517d437ad003e1/docs/ARCHITECTURE.md)。

## 3. 从我们锁定的 0.2.2 到现在，上游哪些变化最有用

本轮直接比较了发布的 GPUI 0.2.2 源码与 gpui-pre 0.3.6（Zed 2026-09-21 快照），并查询若干重要 PR 的真实合并状态；不是把所有 Zed 编辑器更新都算成 GPUI 能力。

| 上游变化 | 重要性 | 对 gpui-rhai 的具体收益 | 0.3.6 状态 |
|---|---|---|---|
| 公开 role、aria label/description、selected/toggled/value、synthetic AX children、AX action | 很高 | 把内部 AccessibilityTree 连接到真实平台语义与系统操作 | API 已确认存在；仍需我们做映射 |
| 缓存 View 内 Input 导致 paint-range 越界崩溃的修复 | 高 | 原生输入与 GPUI cached view 的跨帧正确性，尤其涉及嵌入 Host 缓存 | 已包含修复后的 handler 提取路径 |
| macOS fallback font 继承原字体粗细／斜体 | 高 | CJK／混合字体的标题、加粗和强调不因 fallback 丢失 | 对应上游修复早于快照 |
| Window 可见性与帧唤醒／调度能力 | 高 | 隐藏窗口工作管理、避免回调／呈现被停帧状态滞留 | 已确认公开 API 和相关回归测试 |
| macOS 关闭窗口释放 accessibility adapter | 高 | 多窗口长期运行中，原生 view／Metal layer／queue 的资源释放 | 09-20 合并，早于快照 |
| TestWindow 模拟真实 scale-factor 变化 | 高 | 1x/2x、像素对齐、resize 后 DPI 状态的可靠原生测试 | 已包含 |
| 动态字体安装和 font generation／cache invalidation | 中到高 | Web 按需字体、缺字上报与安装后正确重排 | 已包含；字体获取策略仍由 Host 负责 |
| gpui_platform／各 OS／Web crate 分层；Linux WGPU 路线 | 中到高 | 独立平台选择、后端演进与未来 Web 接入 | 已有；不能据此直接宣称性能更快 |
| App reduced-motion flag 与底层动画遵循 | 中 | 统一底层动画政策 | 核心有 flag；读取 OS 偏好属于 Base／Host 额外适配 |
| wrapped-line 超出行宽命中 panic 修复 | 高，尤其 Linux/Web 文本 | 零宽字符、emoji selector 等边界的命中安全 | **09-23 合并，晚于 0.3.6，不在此快照中** |

几个值得特别关注的已合并修复：

- [#50665：cached view＋Input crash](https://github.com/zed-industries/zed/pull/50665)，2026-05-14。旧 0.2.2 的 `draw()` 会 pop input handler、改变数组长度；新路径保持槽位，防止复用旧 paint range 时越界。本轮核对了两边源码，但没有在我们的现有应用中再次触发该 panic。
- [#56771：fallback font 粗细与样式](https://github.com/zed-industries/zed/pull/56771)，2026-06-11。修复跨字体 fallback 丢失 bold/italic，对我们的中文／拉丁混排直接相关。
- [#63719：测试平台 DPI 模拟](https://github.com/zed-industries/zed/pull/63719)，2026-09-07。测试平台原先固定 2x；现在能通过实际 resize 通道模拟 scale 变化，避免只在 1x 出现的问题被测试漏掉。
- [#64143：macOS AX adapter 释放](https://github.com/zed-industries/zed/pull/64143)，2026-09-20。解决 native view 持有链中的循环引用；我们自己的 Runtime teardown 正确不能替代这个平台层修复。
- [#64672：wrapped-line hit-test panic](https://github.com/zed-industries/zed/pull/64672)，2026-09-23。说明选择候选时必须看精确源码日期，不能把“最新上游已经修复”写成“已发布快照全部包含”。

合并状态与 commit 记录见 [verified-upstream-prs](evidence/upstream-delta/verified-upstream-prs.json)。例如“嵌入 macOS event loop”的 #63077 是 closed 但 **merged=false**，本报告没有将它当作上游已具备能力；closed 不等于 merged。

这是一次能力与关键修复梳理，不是过去 11 个月所有 diff 的穷举审计。重点结论已经明确：**升级底层有实质收益，尤其是系统语义、文本与原生资源生命周期；这些收益不要求先采用 Base 控件。**

## 4. Kit/Base 额外带来的，主要是应用层能力

Kit/Base 的增量包括：

- Input、Textarea、Editor 状态与较完整编辑能力、选择／历史／剪贴板／触控行为。
- Markdown/HTML TextView、文本选择与相应布局基础。
- 焦点 trap、Popup/Popover/Dialog、选择组件、虚拟列表、Dock 等行为模型。
- Base motion primitives、系统 Reduce Motion 读取以及部分原生平台适配。
- facade 的一致入口、精确底层 pin、应用示例与测试工具。

这些确实不只是新版 GPUI 本身。但是我们已有很多同类模型；加入 Base 后若并未替换或复用它们，并不会自动提高现有 Rhai 组件质量。反而要警惕两套 Theme、focus、selection、motion 或生命周期事实同时生效。

一个明确的归属例子：新 GPUI 提供 `App::reduce_motion()`，但不会自动将 OS 偏好写进去；Kit Base 的 `reduce_motion` 模块负责平台读取。macOS/Windows 是初始化／显式重读，Linux 有 portal change 监听。直接用 pre 时，我们可以保留现有 Host MotionPreference 并补这一项适配，不必为一个 flag 导入全部应用行为层。

因此后续采用 Base 的依据应是“这个模块让目标行为更完整、更可靠”，而不是“为了得到最新 GPUI 必须经过 Kit”。

## 5. 直接使用 gpui-pre：已做独立编译验证

我在临时目录创建了一个不依赖 Kit 的小工程，直接使用：

```toml
[dependencies]
gpui = { package = "gpui-pre", version = "=0.3.6", default-features = false, features = ["font-kit"] }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.6", default-features = false, features = ["font-kit", "runtime_shaders"] }
```

这是本轮 **macOS 编译 probe** 的依赖，不是已完成的 gpui-rhai 迁移补丁；Linux/Windows/Web 应按目标选择对应 platform features。

probe 直接使用 `gpui_platform::application()`，并构造带 role、accessibility_id、aria_label/description/selected 的 GPUI 元素。结果：

- 普通依赖树没有 gpui-kit、gpui-base、gpui-component 或 gpui-shell。
- 在当前项目使用的 Rust **1.94.0** 下，依赖编译因 `std::hint::cold_path` 失败。
- 换用本机已安装的 **1.96.0-nightly（2026-04-10）** 后，`cargo check --locked --offline` 通过。没有改上游源码、没有 RUSTC_BOOTSTRAP、没有修改全局工具链。
- `cold_path` 自 Rust **1.95.0** 稳定；这至少说明当前 1.94 MSRV 不能原样保留。没有据此宣称必须使用 nightly，也没有假装已验证所有稳定工具链／平台。[Rust API 记录](https://doc.rust-lang.org/std/hint/fn.cold_path.html)

证据：[probe 源码](evidence/upstream-delta/direct-probe/src/main.rs)、[manifest](evidence/upstream-delta/direct-probe/Cargo.toml)、[Rust 1.94 日志](evidence/upstream-delta/direct-pre-check.log)、[通过日志](evidence/upstream-delta/direct-pre-check-nightly.log)、[完整依赖树](evidence/upstream-delta/direct-pre-full-tree.log)。

这个验证证明“直接依赖和调用新核心 API”可行，不证明整个产品已经迁移、平台无障碍已接通或原生渲染已验收。Kit 0.6.6 同样依赖该 pre 快照；工具链条件不是绕开 Kit 造成的。

## 6. 将来回官方 crate 的路线合理，但应按发布事实选择

截至本轮查询，官方 `gpui` 仍为 0.2.2；单独的 `gpui_platform` crates.io API 查询返回 404。官方 main README 展示新分包用法，不能代替这些包已经发布的证据。

关于用户提到“官方承诺提高发布频率”的表态：本轮检索没有找到可核验的具体原文／周期承诺，已向用户异步询问链接。因此不将这条消息写成已确定的时间表，也不否定可能存在该表态。未来选择应看实际发行结果。

回官方 crate 的条件建议是：

1. 官方已经发布我们需要的 core／platform／macros 等完整依赖家族。
2. 官方版本对应源码具备我们已经使用的 API 和所需修复，没有倒退到更旧快照。
3. 我们的语义、中文输入、主题、生命周期、图表、虚拟化、导出与性能检查通过。
4. Host 和库的依赖图一起对齐，不能混用两套同名但不同 package 身份的 GPUI `App/Window/Entity`。

直接 pre 的便利在于源码/API 路线仍跟着 Zed 上游。可以在 Cargo 中保持 `gpui`、`gpui_platform` 这些使用名，把社区 package 名限制在依赖声明里；未来切换发行来源时更容易审查。但这不构成“未来必定只改一行”或 Rust 类型可跨发行包直接互换的承诺。

“官方”本身不让同一份源码天然更快或更正确。官方发行的实际优势是责任链直接、减少社区重打包这一层；而长期冻结的官方旧版本也会保留已被上游修复的问题。我们应优先选择**可追溯且经验证的较新上游代码**，然后再选择合适发行渠道。

## 7. 当前项目的建议边界

```text
gpui-rhai 自有 Runtime / Rhai 组件 / 主题 / 生命周期 / 数据与图表
                               ↓
                  较新的官方上游 GPUI 核心
                 当前可通过 gpui-pre 家族获得
                               ↓
            官方发行恢复并通过验证后，切换发行来源
```

需要 Base 的具体能力时再引入并确定状态所有权；3D 由 Rust Host 实现，我们维护扩展接缝与必要的能力／生命周期契约。暂不为这些可选需求切换到独立核心分叉，也不默认套用 Kit 的全部应用基础设施。

这不是因迁移成本而保守，而是让依赖层级与我们的产品职责相匹配。本轮未修改产品 Cargo.toml、未升级项目工具链、未执行产品迁移。旧的 Kit／CE 优先结论保留为对应目标条件下的比较；本文件代表用户最新范围下的现行建议。
