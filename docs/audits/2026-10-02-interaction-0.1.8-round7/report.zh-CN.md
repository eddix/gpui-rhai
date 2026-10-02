# 0.1.8 第七轮独立验收

日期：2026-10-02。审计提交：`cda11ce7d80f817e5675543cc7a95f577b977320`（PR #104），对比第六轮 `83b19b1d`。

**结论：第六轮五项原反例均已关闭，但当前提交仍不建议合入或发布。扩展验收确认2个P1、1个P2。** 剩余边界是共享窗口命令队列的原始授权、宽表的实际横向内容范围，以及稳定ElementRef在绑定改变后的订阅。

本轮只审查gpui-rhai。#96/#97/#99未纳入或宣称已完成；disktree-rhai产品不在本轮范围，其既有意见已另存到它自己的目录，后文说明位置。

## 1. 已确认有效的整改

| 第六轮问题 | 本轮独立结果 |
| --- | --- |
| 最后Handle drop后旧deferred close关闭replacement | 原3个probe全部通过；新增只drop一个clone的有效owner正控通过 |
| 父子virtual target同批导致新owner stale | 父先、子先、同批三种顺序，新内层Counter自有callback均返回7 |
| 并列scope的全量旧snapshot复活state | A先、B先、同批均清除移出状态；两行重挂均恢复默认7 |
| raw row依赖贡献按滚动历史累积 | raw/formal各66次切换均有界；离屏释放、共享/root直读、late register、失败回滚、stable key重排通过 |
| Table loading/empty缺少viewport外壳 | 四态live transition的越界点击被阻止；固定总高度212、fill高度310、已接受A列宽128保持一致 |

新增正向覆盖还包括：连续重叠target、部分候选建立后另一scope失败的原子回滚及重试、父裁剪与过期子请求；同Host两个View中，目的View真实状态提交与重绘不破坏另一View的有效拖放。

实现方向值得保留：StateStore已按scope candidate delta合并，并以最终invocation graph裁剪；ReadDependency明确区分可执行owner与item contribution；Table状态共用shell；mount lease贯穿已进入deferred阶段的native操作。没有要求撤销这些有效改进或恢复旧兼容路径。

独立验收来源：

- **完整产品native workspace 161/161通过**，执行时显式移除RUST_MIN_STACK覆盖；[日志](evidence/native-default-stack.log)。
- 当前精确head的远端CI为 **SUCCESS**；[快照](evidence/ci-current.json)。
- 多scope批次：[collections-tree报告](collections-tree/report.zh-CN.md)。
- 共享读取贡献：3/3，[read-contributions报告](read-contributions/report.zh-CN.md)。
- 拖放交叉：4/4，[interaction-crosscheck报告](interaction-crosscheck/report.zh-CN.md)。

没有把作者的623项workspace、严格Clippy、package和完整release smoke全部冒称为本轮独立重跑；本轮完整重跑的是native workspace，其余以远端CI与实施材料分别注明来源。新反例不在现有161项断言覆盖之内。

## 2. R7-1 · P1：共享Runtime队列没有保存原始source，peer View会替已撤权命令授权

定位：`window.rs:58–61`、`:223–261`；`app.rs:4929–4983` 与 `:5292–5306`。

第六轮修正了“原source已把命令取出并defer，随后撤权”的路径。本轮反例发生得更早：命令还在共享Runtime队列，尚未由任何View取出。

真实public API复现：

1. 通过mount_window建立parent，并由它打开真实child窗口，共两扇GPUI窗口。
2. parent同批两条timer：第一条成功入队Close(child)或Open(late)，第二条独立抛错。
3. 第一项事务已成功，但poll_async错误分支未在尾部drain窗口命令。立即dispose parent View，Rust窗口按既有契约保留。
4. 存活child的正常pump随后drain共享Runtime队列。

| 遗留命令 | 期望 | 实测 |
| --- | --- | --- |
| Close(child) | 原source已撤权，不关闭child | 窗口2→1，child被关闭 |
| Open(late) | 不再替已撤权source创建窗口 | 窗口2→3，新增late |

`WindowCommand`只有目标和参数，没有入队时source身份；`request_*_from`检查权限后调用无来源的request入口，把来源丢掉。另一个View drain时，`process_window_commands`使用它自己的 `self.window_authority`，deferred闭包也捕获这个drainer的state/lease。新增source/target校验本身在工作，只是source已被换成了peer。

整改：命令在入队时携带原始window/mount/authority标识；任意pump可执行调度，但不能替命令提供新的权限。执行时核验原source仍有效，并保留当前target实际WindowId/lease校验。撤销尚未创建的Open还需释放Pending reservation，避免后续合法请求永久占用同一ID。

不应回滚整个成功/失败混合batch，也不应禁止secondary pump。整改限定在WindowCommand来源与执行边界，沿用已有mount lease；不需要借此扩建#91的通用调用来源系统。

验收包括成功→失败、失败→成功、source dispose/drop/native close/同ID remount、Open/Close/Focus、目标替换及source仍有效的正控。Focus同分支本轮只作源码推导，实测Open/Close算一个根因。

证据：[runtime-core报告](runtime-core/report.zh-CN.md)，6项=4通过/2失败；其中旧R6三项和clone控制通过。

## 3. R7-2 · P1：宽Table完成了裁剪，却没有实际横向滚动范围

定位：`registry/components/table.rhai:482–489` 的shell及`:628–633`的body；锁定gpui-pre0.3.7 `elements/div.rs:1984–2007`、`:2467–2477`。

合法宽表：viewport300px，固定列120/140/160，总宽420px；LTR把A接受为128后总宽428px。使用1,000行NativeCollection进行真实水平wheel与刷新：

- LTR：wheel -80后A header x仍为1，没有移动。
- RTL：C列在x=-121，正负160水平wheel后仍为-121；data/loading/original-empty/projected-empty四态均无法触及逻辑末端列。
- 正向对照：相同driver，在普通300px scroll容器中放500px直接子节点，x从0正确移动到-80。

因此不是输入方向、未刷帧或测试驱动问题。被裁掉的列无法通过组件声明的横向滚动访问，宽表的功能没有闭合。

GPUI以直接子布局bounds计算scroll content_size。当前shell的header/body仍与viewport同宽，列是它们内部的溢出：布局行宽298px，C列却延伸到x429。孙级溢出并未变成scroll根的更宽内容范围。

原data分支此前已有这一结构，本轮helper使四态一致，但没有建立实际scrollable extent。默认例子改窄会掩盖问题，因此需要保留专用宽表验收。

整改：提供覆盖实际已解析列宽的共享横向content track，header/body使用相同范围、列宽与offset；RTL的起点和另一端可达性也明确。不能改成只裁剪、硬写一个更大宽度或只修示例。保留本轮通过的越界点击防护、height/fill_height和受控列宽状态一致性。

验收以“能实际滚到隐藏列”为主，覆盖fixed/percent/flex混合、resize后变宽、LTR/RTL、四态和非零offset跨状态保持。不能以存在overflow_x_scroll样式或零offset未变化当作完成。

证据：[resize-controls报告](resize-controls/report.zh-CN.md)，4项=2通过/2失败；两项失败属于同一根因，状态切换内的裁剪/高度/列宽前置断言均通过。

## 4. R7-3 · P2：ElementRef绑定变化后读者不更新

定位：`element_ref.rs` 的 `resolve_and_track_geometry` / `reconcile`，以及GeometryRegistry按NodeId保留的订阅。

一个长期存在的Provider声明稳定ElementRef；Producer根据自身state使目标出现、改变节点kind或移除；独立Reader仅在render读取该ref的geometry。推进正常GPUI后台时钟和绘制后：

| 目标变化 | 实际目标 | Reader显示 |
| --- | --- | --- |
| 从未绑定→出现 | 宽120 | 仍pending |
| 同一ref从Text换成Box，产生新NodeId | 宽180 | 仍120 |
| 已绑定目标移除 | 无目标 | 仍180 |
| 同NodeId仅改宽度，正对照 | 120→180 | 正确180 |

正式virtual row自身首次读取geometry的调度对照也通过。因此本项不是整个新的owner/contribution模型都失败，而是稳定ref与当前NodeId之间的绑定生命周期缺少持续订阅。

`reconcile`一次性移除pending readers；尚未绑定时也会丢弃等待。成功解析后，后续几何更新主要依靠NodeId的reader，ref被解绑或重新绑定时，没有把这些观察者迁移/唤醒到新的绑定状态。

整改：保留logical ref → ReadDependency的订阅关系，区分未绑定、已绑定及绑定改变；出现、移除、重新绑定都应使正确owner失效，并把geometry读取迁移到当前NodeId。清理仍遵守component/item contribution，不应让一次解析永久保留无效读者。

本次Provider本身持续存活，改变的是目标绑定；没有用跨已卸载组件的旧ref为新incarnation提出未经验证的额外语义要求。

证据：[geometry-reads报告](geometry-reads/report.zh-CN.md)，最终5项=2通过/3失败，三个失败计为一个P2。初版夹具未推进poll时间导致正控失败，已修正并重跑；无效的ElementRef curry夹具已排除，均不计产品问题。

## 5. 下一轮建议

1. 窗口命令的授权身份在**入队时**确定，不能在drain时猜发起者。新的mount lease可以继续复用。
2. Table的viewport与内容extent分别建模；裁剪和滚动可达性必须同时成立，state shell继续保留。
3. Ref的逻辑订阅与当前节点几何订阅分别建模；贡献存活规则继续共用，不再退回不同registry各自清理。

批次提交和raw依赖清理这次已通过对应模型的顺序/合批、rollback、key重排等对照，应保留这些回归约束。本轮不建议推翻已经关闭的整改或扩大到其他新功能。

## 6. 范围、截图与资料归属

- 本轮没有启动floating测试窗口、没有更改Rift设置，也没有重拍或替换正式Table PNG。临时floating许可仍待明确答复；不能把旧候选或PNG文件审计当成修复后视觉通过。
- #96/#97/#99保持下一组集成工作，不将其尚未交付的能力混入本轮验收。
- disktree-rhai的既有意见副本已放在其独立目录：`/Users/eddix/Codes/github.com/eddix/disktree-rhai/docs/reviews/2026-10-02-gpui-rhai-integration/README.zh-CN.md`。仅复制历史审查资料并说明旧运行器仅作reference；应用源码、依赖和配置未修改。本轮不再分析或实施该应用。
- 历史gpui-rhai审计原件保留，不删除或重写既有证据；本轮仅新增round7材料。
- PR #104未合入，未发包、未创建tag，也未修改GitHub状态。

最终发布判据应是新反例转绿并在同一最终commit通过完整release gates。当前161项原生与CI绿灯均属实，但不能覆盖上述尚未关闭的接口边界。
