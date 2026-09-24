# 二次图表验收整改说明

本轮按 `acceptance-16cf8e51/report.zh-CN.md` 的 R01–R06 整改，并将审计附件中的关键行为迁入产品回归测试。原审计报告和原始证据保持不变。

## 已关闭

- R01：viewport 手势拆分为 native preview、proposal 与 Host acknowledgement；`zoom_change` 携带单调 `viewport_revision`，Host 明确回写后才确认。普通鼠标滚轮、trackpad 分阶段事件、延迟确认、接受与拒绝均有原生窗口测试。
- R02：联动广播只在业务提交边界发生；接收端相等即返回。选择投影按来源保存并合并，两个或三个图表不会互相覆盖或形成空闲布局循环。
- R03：完整域、visible 域和实际 scale 使用同一坐标计划。显式轴范围、log 空间缩放、Geo viewport、Auto category 标签及 log 联动使用一致语义。
- R04：隐式轴先归一到真实轴身份；annotation 每个 region 只布局一次；内建数据 mark 使用无歧义的 role/region/series/datum/part 身份。
- R05：空数据产生合法 empty scene；空 aggregate 保留 group/output schema；domain 读取完整 transformed semantic dataset。分段 LTTB 折叠连续 null，只保留 gap 边界并严格服从总预算。
- R06：Gauge、Map、选择、键盘激活、brush、hover 和联动均以 `DatumRef` 作为业务身份；内部几何 key 不再泄露为回调 key。

## 同轮收口

- Polar、Geo 与 Cartesian 的自定义 renderer 失败统一为候选失败，不再按坐标系吞错。
- 原生 circle stroke 使用真实边框绘制；空心圆不再变成实心盘。选中状态的静态 stroke 由 scene 统一承载，native 与 export 不再各自临时修改 fill。
- chart source cache 命中不再深克隆 inline rows；不可变 snapshot 使用 `Arc` 身份判等。

## 验证

- `cargo test --workspace --all-targets --all-features --locked --offline`：555 项通过。
- 严格 Clippy：通过。
- chart 原生窗口测试：11 项通过。
- 100k debug 基线冒烟：prepare 921641 µs、首帧 427411 µs、resize 180965 µs、streaming 338322 µs，stream 后 Rhai operations 为 0。单样本仅用于冒烟，不作为分位数结论。
- release smoke：Chart Gallery 及完整示例矩阵通过。

`viewport_revision` 是本轮有意采用的受控状态协议变更。项目尚未发布 0.1.5，不保留“任意 Host redraw 即 acknowledgement”的旧语义。
