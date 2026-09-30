# 0.1.8 未关闭 Issue 分级与纳入建议

日期：2026-09-30。代码基线：`0b9b887c961990d4d0365655aeca8b6e6e8bb87a`。
GitHub 只读快照时间：2026-09-30 06:48 UTC。当时共有 **10 个 open issues**；完整正文、评论、labels、时间和 URL 保存在 [evidence/issues](evidence/issues/)，清单见 [open-issues.json](evidence/open-issues.json)。本轮没有回复、关闭 issue 或修改 PR。

“当前实现存在问题”“是否由 0.1.8 新引入”“是否应纳入本轮”分别判断。报告者的复现和修法是输入证据，不直接当作既定事实或设计要求。主报告中的新缺陷也必须解决，不能只清理这张已有 issue 表。

## 推荐决策

| Issue | 当前核验 | 0.1.8 决策 | 依据 |
|---|---|---|---|
| [#80 retained callback 栈深度](https://github.com/eddix/gpui-rhai/issues/80) | 修复成立；实际异步 task 链 200 跳通过，同步递归与操作预算仍受限 | **保留修复，最终合并后关闭** | 已有独立正向与负向证据，不需要重写或提高限额 |
| [#82 unified Interaction Runtime](https://github.com/eddix/gpui-rhai/issues/82) | 公共路由／batch preview 有实现；统一 ownership、取消、controlled revision、呈现命中仍有实证缺口 | **本轮必须完成，当前不能按完成关闭** | 它就是本轮核心交付；详见主报告的发布阻塞 |
| [#84 SplitPane 覆盖 caller 节点](https://github.com/eddix/gpui-rhai/issues/84) | SplitPane.start 和新增 Draggable.handle 都能破坏 formal component 自有 ref；普通 wrapper 对照正常 | **本轮必修** | 虽原 issue 建议 0.1.9，但组件组合是本轮基础，不宜要求所有调用方猜测性包 box |
| [#86 超限异步结果](https://github.com/eddix/gpui-rhai/issues/86) | 成功／错误消息、嵌套累计、数组与 subscription 均存在问题 | **本轮必修** | 会令加载状态无法完成，且错误处理本身可能失败；只修顶层字符串不够 |
| [#90 CLI check 与 capability](https://github.com/eddix/gpui-rhai/issues/90) | 干净项目基线通过；合法 manifest 的 init capability 调用仍被报成未声明 | **本轮必修开发工具契约** | 严重度低于 Host panic，但不应继续让推荐 check 错拒真实应用 |
| [#85 Resizable 自动化键盘](https://github.com/eddix/gpui-rhai/issues/85) | 定位成功但 `Dispatch(key:right)` 返回 invoked=0、尺寸不变 | **建议本轮补齐** | 与当前键盘／native action 统一和验收直接相关；不应再为它复制一套脚本几何算法 |
| [#87 挂载后 NativeCollection 注册](https://github.com/eddix/gpui-rhai/issues/87) | 当前确实只能预注册，再 replace 已有名字 | **建议本轮仅补 Host live-register** | 与本轮虚拟 Sortable／Tree 和实际应用数据接入相关；这是受控增量，优先级低于全部发布阻塞 |
| [#89 Table 行／单元格 context request](https://github.com/eddix/gpui-rhai/issues/89) | 当前公共事件确实没有该能力；普通 ContextMenu 无法直接进入内部生成的单元格 | **建议下一轮** | 有实际价值，但属于新增 Table 交互 API，未违背本轮已有承诺；先稳定所有权和事件基础 |
| [#83 自定义 resize grip](https://github.com/eddix/gpui-rhai/issues/83) | slot、overhang hit area、层叠与原生状态绑定尚未提供 | **保留 0.1.9 enhancement** | 不是只加 `handle: node`；应在 #84 与命中模型修复后设计 |
| [#91 capability invocation origin](https://github.com/eddix/gpui-rhai/issues/91) | Handler 目前只收到 method/input，没有可靠来源上下文 | **后续独立设计** | 来源信息与用户激活权限不是同一契约，不能用最近点击时间临时冒充权限保证 |

上述 #85/#87 是本轮建议增量，不与 panic、状态损坏和错误提交等硬阻塞同级。如果本轮需要收缩，只能明确移出其承诺、保留 issue，不可把它们写成已完成；尤其不要为了抢进 #87 而推迟已确认 P1 的修复。

## 重点修法与验收边界

### #80：可以认定根因修复，关闭时机仍是最终合并

[Rhai 子报告](rhai-async/report.zh-CN.md) 运行真实 `start_task → completion → start_task`，不是仅调用 adapter：200 跳，每跳独立操作预算；同一 retained callback 内真实递归仍 Stack overflow、无限循环仍被终止。没有证据要求重开其技术方案。保留这些测试，随最终可发布分支合并后再关闭。

### #84：组件应该拥有 wrapper，不能接管用户组件的根身份

SplitPane 的 part/ref/preview 应放在自己的 wrapper 上，调用方 node 保留 key/ref/style/信号与 formal component 路径。Draggable 的 handle/content 同样检查。要验证 wrapper 新增后 flex、最小尺寸、百分比布局和折叠恢复正常；“文档注明会覆盖 ref”不构成合适修复。

证据：[resize-controls](resize-controls/report.zh-CN.md) 的 RC-4，有正式组件、正常容器对照及两个失败路径。

### #86：先校验可交付性，不能在 callback 失败后补跑 error

Schema 合法不等于 Rhai 可以消费。应按当前 Engine 的递归累计字符串／Array／Map 限额，在执行回调前检查 task/subscription 的成功与失败数据，生成自身也有界的错误对象。

本轮纠正了 issue 中一项推断：超大参数并不保证回调从未执行。探针证明，回调忽略参数可以成功；先调用 Host capability 再读取大参数，会在外部副作用已经发生后失败。因此不能 catch success callback 的任意错误后重试或再调用 error。仅对执行前的交付失败做路由，脚本运行中的事务错误继续遵循原语义。

证据：[rhai-async](rhai-async/report.zh-CN.md)。

### #90：先明确 check 的验证等级，再修实现

实际代码在 `crates/gpui-rhai-cli/src/lib.rs:438` 先 `validate_entry`，随后才解析 `ui/app.toml`；`validate_entry:1011–1087` 自建 RuntimeState 并启动 init，没有配置真实 Host handlers 或激活 manifest。

通过当前 HEAD 重新构建 CLI，在一次性空项目中独立复现：生成项目 check 成功；manifest 声明 `app.demo`，init 调用它后退出 1，错误仍说 “was not declared”。负向对照中故意错误的初始 view 被正常拒绝。见 [复现脚本](cli-check/repro.py)、[结果](cli-check/results.json)。

修复至少要做到：

1. 先解析、校验 manifest、组件依赖与权限声明。
2. 区分静态检查和有 Host context 的运行检查。CLI 不知道某个 Host capability 的结果，不等于应用没有声明它。
3. 缺少运行环境时准确报告哪些动态检查未执行／需要 Host fixture，而不是输出虚假的全通过或错误的未声明诊断。
4. 可选动态验收需要应用显式提供可控的 Host descriptor/fixture；不自动调用真实应用服务，也不凭 output schema 生成一个被当作成功的业务占位值。
5. 保留对可检查的语法、import、schema、initial view 错误的发现能力；render 中同步调用 capability 仍按现有阶段限制拒绝，不能因 issue 对“首帧”的泛称放开该规则。

### #85：自动化进入同一个语义动作

当前缺口已实测，而非仅源码猜测。优先让 native keyboard、AX 和 Automation 能到达同一个受控尺寸提议入口；或为聚焦对象提供明确且可验证的真实输入注入。不要把按键字符串匹配加入每个组件，形成另一套 resize math。验收既检查结果也检查 invoked 数量、focus/disabled、边界与拒绝回退。

### #87：只做需要的最小 Host 增量

建议显式 live-register，保留 replace “更新已存在名字”的含义；不要未经设计把 replace 改成无限 upsert。明确重名、合法名、disposed/suspended view、数据所有权、清理、首次读依赖与唤醒。

暂不并入脚本 `set_native_collection`：它还涉及事务回滚、热重载、内存／行数／集合数量预算。Array 转原生集合也不能绕过输入已经要通过的 Rhai 限额。错误提示与 Table 文档应说明何时使用 NativeCollection，但 issue 中的单机数字不能写成产品性能保证。

### #89、#83、#91：下一轮需要保留的设计输入

- #89：上下文请求携带 stable row/column key、正确坐标及 pointer/keyboard 来源；排序／过滤／虚拟化后仍命中同一对象，键盘触发可用行 bounds 作锚点。不要为菜单而把所有行 materialize 到 Rhai。
- #83：视觉 grip、可命中范围与单一 focus owner 分离；先保证裁剪、层叠和状态投射，再提供自定义节点。
- #91：由 Runtime 维护来源、view 归属与嵌套还原，覆盖 UserInput/Automation/AX/Timer/Task/Subscription/Effect 等路径。异步 continuation 不继承历史用户激活。调用来源枚举不直接等于可消费权限 token；明确这个区分后再设计 API，不增加基于时间窗口的临时门禁。

## 关联 PR #88：建议 0.1.8 纳入

[PR #88](https://github.com/eddix/gpui-rhai/pull/88)，head `5ec2758e197b5ec6e7fdb868650c6847860a8071`，修复 `UiNode::replace_virtual_collection_items` 在寻找目标集合时给每个经过的节点深拷贝 realized map。

`Option::take` 补丁对准了根因，保持首个匹配目标语义；确定性分配身份测试优于仅检查单次速度。此代码位于本轮虚拟 Sortable 的关键路径，建议纳入。审查本身没有 checkout、应用或合并 PR 补丁。

已额外核验该精确 head 的远端 CI job：workspace、default、performance structure、native keyboard、package、release build、Linux X11/Wayland smoke 与 artifact audit 步骤均为 success。见 [PR CI 明细](evidence/pr-88-ci-job.json)。这比仅引用 PR 作者“本机没跑”的列表更完整，但最终合并其他整改后仍需重新验证，不能将这份旧 head 的 CI 当作最终分支结果。

不把该补丁宣传为修复所有 fling 空白行或消除整树 transaction 成本；PR 明确撤销了过度 overdraw／更高频 pump 的试验。完整静态审查及性能边界见 [collections-tree](collections-tree/report.zh-CN.md)。
