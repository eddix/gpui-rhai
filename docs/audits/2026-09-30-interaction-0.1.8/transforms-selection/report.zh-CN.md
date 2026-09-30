# 0.1.8 对抗式审查：Canvas 变换、PanZoom、Rotatable、SelectionArea

审查基线：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`，对照 `v0.1.7` 和 ADR 0022。日期：2026-09-30。

结论：本子域发现 **3 个 P1、3 个 P2**，全部通过独立 GPUI 原生探针确认。没有修改产品源码、正式测试或已有审计材料。这里的 Canvas 变换属于当前明确支持的能力；没有将任意 GPUI 子树缩放、3D 或其他已排除范围列为缺陷。

## 证据与运行方式

- [独立探针源码](src/lib.rs)
- [独立 Cargo manifest](Cargo.toml) 与 [锁文件](Cargo.lock)
- [完整执行日志](native-probes.log)

从仓库根目录运行：

```sh
CARGO_TARGET_DIR=target cargo test \
  --manifest-path docs/audits/2026-09-30-interaction-0.1.8/transforms-selection/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

探针使用项目固定的 `gpui-pre = 0.3.7`、当前产品 crate、真实 registry 组件和真实 ScriptViewHost。六个测试均断言应有行为，因此当前基线结果为 **0 passed / 6 failed**；其中五个失败来自探针断言，一个直接来自产品绘制路径 panic。这里不把测试失败包装成验收成功。

## TS-1 / P1：合法有限 PanZoom 参数能令 Host 绘制直接 panic

定位：

- [pan_zoom.rs:394](../../../../crates/gpui-rhai/src/pan_zoom.rs#L394)：只约束平移和比例区间关系，没有确保缩放及派生矩阵处于可呈现数值范围。
- [canvas.rs:577](../../../../crates/gpui-rhai/src/canvas.rs#L577) 至 584：矩阵组合和中心重定位均使用 `expect`。

触发：将公开 `PanZoom` 参数设为 `transform: #{x:0.0,y:0.0,scale:1.0e308}`、`max_scale:1.0e308`，内容为普通 Canvas 矩形，控件尺寸 300×220。所有输入都为有限数且满足公开 schema。

预期：不支持的值在脚本/组件边界给出可处理诊断，或者绘制安全拒绝该帧；Host 不应因参数数据崩溃。

实际：`canvas.rs:584` panic：`validated canvas bounds produce a finite affine origin: InvalidTransform`。有限矩阵成员乘以视口中心仍然可能溢出；验证有限输入不等于验证有限派生结果。

验证：`audit_finite_panzoom_scale_does_not_panic`。此失败来自产品源码，而非探针断言。

整改要求：明确变换参数、组合结果、逆变换及最终 GPUI 坐标的支持范围；去除用户数据可达路径上的 `expect` 假设，返回诊断或安全拒绝。不能只给本例常量加特判。回归还应覆盖极端有限平移/缩放、极小缩放、奇异矩阵、矩阵组合溢出及最终 f32 坐标边界。

## TS-2 / P1：Rotatable 初始非零角度没有应用显式 pivot 补偿

定位：

- [rotatable.rhai:49](../../../../registry/components/rotatable.rhai#L49) 至 53：角度 signal 取 props，平移 signal 却固定为 0，同时 source token 初始化为当前 token。
- [rotatable.rs:424](../../../../crates/gpui-rhai/src/rotatable.rs#L424) 至 431：token 已匹配即跳过 `write_preview`。

触发：首次挂载 `angle:90.0`、`pivot:#{x:0.0,y:0.0}` 的 300×220 Rotatable，Canvas 中放普通矩形。

预期：初始帧就绕声明的 pivot 旋转；恢复保存的非零角度与交互后得到的相同角度应呈现一致。

实际：Canvas 的 visual bounds 为 `x:1, y:1, width:298, height:218`，没有任何 pivot 补偿平移。Canvas 原生变换绕自身中心旋转，所以初始对象围绕错误的中心。这个错误不是只少一个通知：后续读取到相同 source token 仍然跳过同步。

验证：`audit_initial_rotation_pivot`。探针检查真实呈现几何的平移；按实际 Canvas 中心约为 `(149,109)` 计算，绕 `(0,0)` 旋转 90°需要约 `(-258,40)` 的补偿，0 显然错误。

整改要求：将首次呈现、source 更新和布局变化后的 pivot 计算纳入同一个变换解析；不能把“token 已初始化”等同于“已有匹配当前布局的有效变换”。补充初始非零角度、非中心 pivot、边框/内边距、容器尺寸变化的绘制与命中一致性测试。

## TS-3 / P1：旋转 Canvas 上的 marquee 选择与屏幕框选区域不一致

定位：

- [selection_area.rs:106](../../../../crates/gpui-rhai/src/selection_area.rs#L106) 至 110：只把起点和终点分别逆映射为局部点。
- [selection_area.rs:442](../../../../crates/gpui-rhai/src/selection_area.rs#L442) 至 450：将这两个局部点重新构造为局部轴对齐矩形，再与目标局部矩形比较。
- [app.rs:3321](../../../../crates/gpui-rhai/src/app.rs#L3321) 至 3337：逆映射本身使用 Canvas affine；问题在于后续用两个点替代完整几何区域。

触发：SelectionArea 的 Canvas 通过公开 Motion 设为恒定 45°；目标 a 的局部矩形为 `(70,75,8,8)`；在屏幕上由相对选区 `(20,20)` 拖至 `(100,100)`。旋转后的 a 完全位于这个屏幕矩形内。

预期：`intersect` 和 `enclose` 两种策略均应选中 a。

实际：`selected=[]`。45°逆变换后，上述屏幕矩形的两个对角点几乎落在同一局部 y 上；用它们构造的局部轴对齐区域退化为细条，丢失真实选区。

验证：`audit_rotated_canvas_marquee`，使用实际 Canvas 绘制路径和实际指针事件，没有依赖不支持的原生子树旋转。

整改要求：保留屏幕选区矩形并前向映射目标，或将选区的四角完整逆变换后做凸多边形/矩形相交与包含判断。仅将四角的 AABB 相交也会引入误选，不能作为精确 `enclose/intersect` 的通用替代。统一读取同一呈现帧的变换、裁剪和目标几何，并覆盖 45°、90°、非均匀缩放与选择期间几何变化。

## TS-4 / P2：SelectionArea 的 single 模式依然允许 marquee 提交多个对象

定位：[selection_area.rs:442](../../../../crates/gpui-rhai/src/selection_area.rs#L442) 至 465。

触发：`multiple:false`，有 a、b 两个目标；一次框选同时覆盖二者。

预期：维持单选约束；按明确规则选择至多一个对象，或者不提供多目标 marquee。

实际：提交 `selected_keys=["a","b"]`。`config.multiple` 仅参与 additive 判断，不限制命中集合大小。

验证：`audit_single_mode_marquee`。

整改要求：把“至多一个 selected key”作为单选模式统一不变量，应用于鼠标、marquee、键盘 Space、source 输入等路径，而不只是 click 分支。明确多目标框选时的选择规则并加入行为测试。

## TS-5 / P2：选中对象后点击空白无法清除选择

定位：[selection_area.rs:380](../../../../crates/gpui-rhai/src/selection_area.rs#L380) 至 382。

触发：先普通点击 a，再普通点击无目标的空白位置。

预期：清除 selected/active；anchor 按文档确定的规则处理。

实际：仍为 `["a"]`，没有清除提案。代码仅在 `selected.is_empty()` 时产生空选择提案，条件方向颠倒。

验证：`audit_blank_click_should_clear`，日志依次为 `selection=["a"]`、`selection=["a"]`。

整改要求：修正空白点击规则，并分别定义普通空白点击、带修饰键空白点击、空选择重复点击是否发事件；不要让无状态变化的点击持续产生冗余提案。

## TS-6 / P2：PanZoom 的 modifier wheel 取消事件可能被过滤，留下未提交预览

定位：[pan_zoom.rs:199](../../../../crates/gpui-rhai/src/pan_zoom.rs#L199) 至 215。

触发：`wheel_zoom:"modifier"` 下，以 control 开始显式 wheel gesture，随后收到已松开修饰键的 `TouchPhase::Cancelled`。

预期：正在进行的手势必须接收其取消事件，即使当前修饰键已变化；恢复 source transform 并清除 active/pending 状态。

实际：enabled 判断先于 Cancelled，且只为 Ended 保留 active 手势资格，因此 Cancelled 被直接 return。原生探针读取到 **取消后 scale 仍为 1.1051709180756477，source 为 1.0**。再投递无修饰键 Ended，会产生 1 次本不应出现的 transform proposal。

验证：`audit_cancelled_modifier_wheel_must_not_commit`。取消后预览未恢复的中间信号读数已直接确认，结论不依赖平台一定会在 Cancelled 后继续发送 Ended。

整改要求：手势归属、更新资格和终结/取消资格分开处理；当前修饰键只决定是否开始或更新，不可屏蔽已拥有手势的取消。测试修饰键中途释放、取消、Escape、失焦/暂停/卸载及后续新手势，确保旧预览和延迟提交不能复活。

## 核心模型判断

已有统一 `Affine2D` 和手势协调器是有用基础，但消费者仍然各自推导几何/控制条件：Rotatable 用 token 推断初始化完成，SelectionArea 用两个点近似选区，wheel 自行决定终结事件资格。六个问题中的前三个应优先通过统一的“已验证、已呈现变换”边界解决，而非散落新增特判。

现有原生 happy-path 测试覆盖了初始角度 0 的旋转、未旋转的选择和正常结束的 wheel，这些条件恰好绕过上述缺陷。Gallery 的 interaction workbench 把 selection、pan 和 rotate 放在三个不同的 Canvas 上，因此“同一页面展示了三个控件”不能替代“同一对象坐标空间下真实组合工作”的验收。
