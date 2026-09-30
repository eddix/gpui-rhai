# 0.1.8 第二轮对抗审查：变换与选择

基线：`d77b4b49a667da618fe9b9583d331d8c325cf180`；对比上一轮 `0b9b887c`。遵循修订后的 ADR 0022，不再用旧 ADR 的 awaiting-control 描述评判当前实现。

本子域确认 **1 个 P1、3 个 P2**，涉及五个新失败场景；另有一个负缩放、非均匀缩放、旋转组合的原生正向对照通过。旧六个探针由主审统一复跑，本目录没有重复运行旧案例。没有修改产品、正式 tests 或旧审计材料。

## 独立证据

- [探针源码](src/lib.rs)
- [manifest](Cargo.toml)、[锁文件](Cargo.lock)
- [最终原生执行日志](native-probes.log)

仓库根目录执行：

```sh
CARGO_TARGET_DIR=target cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8-round2/transforms-selection/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

结果：**1 passed / 5 failed**。失败测试断言的是期望行为；六个案例均通过当前 registry 组件、真实 ScriptViewHost、GPUI 原生输入与布局运行。

## R2-TS-1 / P1：Rotatable 的 pivot 补偿仍未以实际 Canvas 几何为准

定位：

- [rotatable.rs:101](../../../../crates/gpui-rhai/src/rotatable.rs#L101) 至 107：新增初始化直接使用 RotationElement 自身的 prepaint bounds。
- [rotatable.rhai:72](../../../../registry/components/rotatable.rhai#L72) 至 75：有 `handle` 时，RotationElement 位于 28×28 的独立操作柄里。
- [rotatable.rs:432](../../../../crates/gpui-rhai/src/rotatable.rs#L432) 至 439：token 相同就跳过同步；token 没有包含实际 Canvas 布局变化。

### 新场景 A：独立操作柄

给 300×220 Rotatable 设置 `angle:90`、`pivot:{x:0,y:0}`、`handle:text("R")`。Canvas 实际内容尺寸为 298×218。

预期：是否提供独立操作柄不能改变旋转中心；补偿应基于 Canvas 中心，约为 `(-258,40)`。

实际：初始补偿为 **`(-26,0)`**，Canvas visual 为 `(-25,1,298,218)`。这是操作柄内部约 26×26 的中心补偿。首次同步随后写入匹配 token，后面取得正确 viewport 的路径也不再校正。

原生探针：`round2_initial_pivot_with_separate_handle`。

### 新场景 B：窗口真实 resize

不提供 handle，保持 `angle/pivot` 不变，让 Rotatable 的宽度跟随窗口。通过 TestWindow 真正改变窗口尺寸。

实际：Canvas 宽度 **1918 → 598**，平移补偿始终为 **`tx=-1066`**，没有随绘制中心移动而变化。画布尺寸改变后，声明 pivot 对应的屏幕点会漂移。

原生探针：`round2_resize_recomputes_pivot`，额外断言 Canvas 宽度确实发生变化，避免把空操作当作 resize。

整改：同一个已呈现 Canvas 几何快照应决定 paint/hit/pivot；不能用操作柄 bounds 冒充内容 bounds。变换有效性必须包含实际布局尺寸/内容原点，而不仅是 props 字符串。把独立 handle、无 handle、边框/内边距、首次挂载、真实窗口 resize 和缩放后的 pivot 不动性一起验收。

这属于上一轮 TS-2 的整改闭环未完成，不是要求支持任意 GPUI 子树旋转。

## R2-TS-2 / P2：退化 marquee 把线段延长线上的远处目标误判为选中

定位：[selection_area.rs:507](../../../../crates/gpui-rhai/src/selection_area.rs#L507) 至 522，及 556 至 561。

触发：在无变换的 SelectionArea 内水平拖动，从局部 `(5,20)` 到 `(50,20)`；目标 a 位于 `x=180..230,y=20..60`，目标 b 位于 `x=100..150,y=20..60`。二者均与这个短线段不相交。

预期：不选中 a/b；可以把零面积 marquee 当作短线段、空区域或其他明确规则，但不能无限延长。

实际：提交 **`["a","b"]`**。水平拖动产生退化四边形。目标上边缘角点与所有非零边共线，`point_in_polygon` 所有 cross 为零后无条件返回 true，没有验证线段范围。

原生探针：`round2_horizontal_marquee_does_not_select_far_collinear_object`。

整改：几何入口显式处理零面积及近退化多边形，使用有界点/线段判断或拒绝零面积 marquee。覆盖水平、垂直、反向拖动、边界相切和缩放后近退化情形。普通非退化凸多边形算法不能直接用于退化区域。

## R2-TS-3 / P2：单选约束只修了 marquee，Space 路径仍能提交多选

定位：[selection_area.rs:178](../../../../crates/gpui-rhai/src/selection_area.rs#L178) 至 185。

触发：公开合法控制状态 `multiple:false`、`selected_keys:["a"]`、`active_key:"b"`；取得原生焦点后按 Space。active 与 selected 是文档明确分离的概念，不能假设两者永远相等。

预期：提交至多一个选中 key；如 Space 用于选择 active，则为 `["b"]`。

实际：提交 **`["a","b"]`**。Space 分支总是克隆旧 selected 集合并切换 active，不读取 `config.multiple`。

原生探针：`round2_single_mode_space_respects_selection_invariant`。探针使用固定受控 props，先以空白点击取得原生焦点，Host 保持这组源值，再按 Space；记录实际 proposal。没有向组件输入已违反单选的 selected 集合。

整改：单选约束应施加在统一 selection proposal 边界，覆盖 click、marquee、Space、范围选择和源状态校验；不应只在单个输入分支加限制。验收必须包含 active 与 selected 不同的正常受控状态。

## R2-TS-4 / P2：Escape 无法取消进行中的 wheel zoom

定位：

- [pan_zoom.rs:357](../../../../crates/gpui-rhai/src/pan_zoom.rs#L357) 至 364：键盘处理只有 transform proposal，未处理 wheel session 的 Escape。
- [app.rs:256](../../../../crates/gpui-rhai/src/app.rs#L256)：Host 的 Escape 只取消 shared pointer coordinator 或 overlay；wheel preview/generation 位于另一组原生 signal 中。
- [pan_zoom.rs:200](../../../../crates/gpui-rhai/src/pan_zoom.rs#L200) 起：wheel 的 active/ending 处理独立于上述取消路由。

触发：先让 PanZoom 取得焦点，以 control+wheel 的 `TouchPhase::Started` 开始缩放，按 Escape。

预期：按组件文档的 shared interaction contract，取消临时交互并恢复 source，不允许迟到 Ended 再提交。

实际：source scale=1，按 Escape 后原生 preview 仍为 **`1.1051709180756477`**；继续发送 Ended，会产生 **1 次** transform proposal。

原生探针：`round2_escape_cancels_active_wheel_preview`；取消后的中间 signal 值和最终 proposal 数均有日志。

整改：wheel session 也应参与明确的取消/失效入口，并统一清除 active/pending、废弃延迟 commit generation、恢复 source；避免仅修补 `TouchPhase::Cancelled` 一条路由。这里不重复另一个审查 agent 正在验证的 PanZoom suspend/source 更新重入 panic。

## 本轮验证通过及边界

`round2_negative_nonuniform_rotation_enclose_control` 使用 Canvas `scale_x=1.5`、`scale_y=-0.5`、`rotate=30°`，依据目标真实映射的屏幕包围区域框选，`enclose` 正确只选中 a。这确认四角 inverse + polygon 的修改在非退化、含负行列式的组合变换上有实际收益。

源码检查也确认 `canvas_motion_affine` 的 invalid-result fallback 被 paint、Canvas hit test 和 SelectionArea inverse 路径共用；本轮没有把“必然 paint/hit 不一致”作为 finding。PanZoom 新增上限确实限制了上一轮巨值入口。不过 finite / inverse / 最终 f32 数值范围的全域证明不属于上述一个正向探针能给出的结论，不能将这些结果等同于完整数值安全验收。

不变的核心问题：几何、控制约束和取消资格仍在各输入路径分别实现。当前修复能使上一轮具体案例通过，但不同 handle、布局变化、退化几何、键盘和 Escape 会再次绕过这些局部条件；整改应围绕同一呈现几何、同一 selection 不变量、同一 interaction 取消契约来做。
