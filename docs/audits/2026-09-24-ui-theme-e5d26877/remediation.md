# 第九轮 UI／主题一致性整改说明

本轮按 `report.zh-CN.md` 的 U01–U05 收口，不将像素清单机械替换成另一批
常量。原报告与证据保持不变；修复性质已进入正式测试。

## U01：Tabs 自适应几何

Tabs track 不再声明固定高度。Slot 使用 resolved `body` line box、上下
`spacing.xxs`、保留边框和 26px 最小下界；track 高度由 slot 测量结果加
两侧 inset 得出。横向／纵向及 content／equal 共用该关系。图标独占项加入
零宽 line-box spacer，因此字体放大仍能扩展行高。

审计的 8px inset 与 36px line-height 两项原生探针均通过；正式原生测试
覆盖两项同时变化时的 8px 四边 inset 与 54px slot。

## U02：完整主题快照

`ColorResolver` 现在可以生成完整的不可变颜色快照。`ThemeVariant` 枚举
基础颜色和所有已验证 namespace color；`PrimitiveTheme` 与 virtual/overlay
使用的 `OwnedColorResolver` 复用同一接口。每个虚拟集合捕获一次，不在每
个 realized row 复制主题，也不向后台传递可变 ThemeManager 或 Rhai 值。

自定义 `brand.tint` 原生探针通过；正式测试同时验证普通快照、virtual
snapshot、基础 selection、`table.selection`、`spacing.xxs` 和自定义
namespace。

## U03：组件间距尺度

视觉密度值已按统一角色迁移到 `xxs/xs/sm/md/lg`：控件紧凑 inset、图标与
文字、普通内容 padding、panel content 和 dialog/empty outer space 分别
使用共享尺度。迁移覆盖 Button/Badge/Tag、Tabs、表单、菜单／Command、
容器／浮层、状态组件和 Table。

重跑审计扫描：固定间距从 **35 文件／85 行** 降到 **3 文件／4 行**；剩余
仅为 IconButton 的零 padding、Switch 结构 inset、TitleBar 零 gap 和平台
安全 inset。正式源码测试锁定这份例外清单，防止视觉密度重新散落。

## U04：Tabs 可读前景

主题加载阶段派生 `tabs.foreground`：若 `text_muted` 在实际
`surface_hover` 上已达到 4.5:1，则原样保留；否则仅向 `text_primary`
混合到达到阈值。主题可显式覆盖该 namespace token。15 个官方主题的实际
配对均由正式 contrast test 验证；Gruvbox 实机检查通过。

## U05：Chart typography

Chart label 明确区分 title 与 label role。标题消费 `title`；轴刻度、轴标题、
legend、值和 tooltip 消费 `body_small`。GPUI 标签与 SVG/PNG 共用 resolved
family、fallbacks、size、line-height 和 weight。Plot margin 随 line box 和
字号增长，居中标签宽度按文本与字体大小估算，不再回到固定 12px／80px
假设。导出测试覆盖 JetBrains Mono＋CJK fallback、24/18px 两级字体和扩大
后的 plot margin；Chart Gallery 实机通过。

## 文档与参考图

维护规格、组件目录、视觉验收矩阵已归并为 token／几何关系契约。旧临时
规格及 `button-density-reference.png`、`badge-density-reference.png`、
`tabs-track-reference.png` 一并删除；这些第三方参考不作为仓库产品资产。

## 验证

- 第八轮 viewport 探针：18/18 通过。
- 本轮 UI 审计探针：3/3 通过。
- Workspace 全目标、全 feature：565 项通过。
- 原生独立包：101 项通过。
- fmt、Workspace/native 严格 Clippy：通过。
- 间距扫描：3 文件／4 行结构例外；35 个有视觉 spacing 的组件使用 token。
- Default Light/Dark、Gruvbox Navigation & Data，以及 Chart Gallery 实机通过。
- 100k release（5 warmups / 30 samples）：prepare 144.166 ms、首帧
  122.793 ms、resize p50/p95 26.800/28.718 ms、128 行 sliding update
  p50/p95 81.511/98.171 ms；stream 后 Rhai operations 为 0。
- 完整 `scripts/release-smoke.sh` 通过，包括 Chart/Component Gallery、全部
  examples、Theme Studio 与 Table 状态矩阵。
