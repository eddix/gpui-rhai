# 第八轮整改说明

本轮按 `report.zh-CN.md` 的单一工作包整改：有效 viewport 的身份、手势
基准与布局重投影由同一状态模型负责。原报告和复现证据保持不变，失败
性质已迁入正式测试。

## 有效 viewport

- Chart 保留 coordinate-typed effective viewport。Cartesian 直接保存每个
  region/axis 的逻辑窗口；Geo 保存 map/projection identity、zoom 与按 plot
  归一化的 camera pan。
- wheel/pan 从当前 effective viewport 开始，而不是从过时的 Host scalar
  重新计算。手势 preview、proposal、Host acknowledgement、link broadcast
  与 scene layout 因而表达同一个结果。
- `zoom`/`pan_x`/`pan_y` 仍是单相机便利输入；正式受控协议新增 `viewport`
  值，`zoom_change` 同时返回 exact typed viewport。不同 X/Y 窗口不再为了
  适配一个 scalar 而丢失信息。
- bounds 变化只把当前逻辑 viewport 重投影到新 plot；它不确认、拒绝或
  取消活动手势。Suspend 仍按既有契约取消 transient preview 并恢复最后
  committed/link 状态。

## 联动身份

LinkRegistry 为每次 viewport 提交分配注册表级单调 commit，并将完整
`(group, domain)` 保留在投影身份中。换组、来源替换和同号旧 counter 不会
再命中新组的去重条件；来源卸载仍撤回它拥有的投影。

## 同轮人工验收修复

- Chart Gallery 根节点现在受窗口高度约束并拥有本地纵向滚动 viewport。
- Chart label 使用真实 Start/Center/End 文本盒；删除固定 `-20px/-52px`
  字符宽度猜测，Y 轴多位数字不再被裁到窗口外。
- 主题 spacing 新增 `xxs`（官方主题为 2px）。Tabs 的 track padding/gap
  使用 `spacing.xxs`，track/thumb 使用 `radius.sm`，选中 thumb 不再增加
  独立边框。
- Table 的选中态使用主题派生的 `table.selection` 不透明 accent/surface
  混色作为整行背景，并公开 `row` 与 `row_selected` 样式 parts；
  hover/striping 不覆盖选中填充。

## 自动验证

- 本轮审计原生探针：18/18 通过。
- Workspace 全目标、全 feature：564 项通过。
- 原生独立包：100 项通过，其中 Chart 28 项、控件视觉 4 项。
- Workspace 与原生独立包严格 Clippy、fmt：通过。
- Chart Gallery 永久测试验证页面滚动确实改变离屏图表的呈现位置。
- 100k release（5 warmups / 30 samples）：prepare 119.833 ms、首帧
  107.587 ms、resize p50/p95 23.765/23.958 ms、128 行 sliding update
  p50/p95 74.980/75.994 ms；stream 后 Rhai operations 为 0。
- 完整 `scripts/release-smoke.sh` 通过，包括 Chart/Component Gallery、全部
  examples、Theme Studio 与 Table 状态矩阵。

解锁后的最终实机矩阵已完成：Chart Gallery 在浅/深主题下的轴标签与
页面滚动通过；Component Gallery 的主题化 Tabs 通过；Table 的浅色多选
行与深色分组行均显示完整 `table.selection` 背景。滚轮落在 Chart 内部时
仍由图表缩放消费，落在 Gallery 留白时由页面滚动消费。
