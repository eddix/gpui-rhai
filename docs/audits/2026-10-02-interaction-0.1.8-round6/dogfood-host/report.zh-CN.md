# DiskTree dogfood：Rust Host 接入、扫描生命周期与开放 issue 交接

审查对象：`disktree-rhai e33a61c4ebc8a6cd6de7ca55b36896452ee92685`；框架对照 PR #104 的 `83b19b1d`。源仓库只读，未启动 GUI、未扫描用户目录，也未执行全盘扫描。初始磁盘空间不足时仅静态检查；空间恢复后只运行了本报告的微型快照 fixture，复用现有 native target。

结论：窗口命令问题已有框架 adapter，可直接迁移；扫描结果发布和原生路径身份有应用自身缺陷，不应继续归咎于 gpui-rhai。另有热更新构建说明与实际 feature 不一致。脚本算法、主题与 PR #96/#97/#99 的细节由另一分支审查，这里只记录 Host 接口边界。

## 1. 已有 adapter 覆盖的需求

### G10：手工 Host 的 q/close/focus

Dogfood `src/main.rs:386` 仍使用 `ScriptViewHost::new_with_policy`，`:426` 使用普通 `prepared.mount`。这对应旧 integration branch 的接入方式。

在 `83b19b1d` 中，当前入口是：

```rust,ignore
let host = ScriptViewHost::new("main", cx)?;
let view = prepared.mount_window(
    ScriptViewConfig::new("main").paint_background(true),
    host, window, cx,
)?;
```

`mount_window` 显式委托一个 view 的原生窗口命令权限、登记实际 WindowHandle、安装关闭拦截；普通 `mount` 按契约禁用窗口命令。参考框架 `app.rs:2495`、`:2526` 和 `docs/multi-window.md`。

这已覆盖 G10 及 #100/#103 的接入需求，不需要再暴露第二套可覆盖 alias 的 native-window registry，也不应恢复让同 Host 其他 view 自动继承权限的旧接口。Dogfood 当前没有安装自己的 should-close 回调，因此新 adapter 的接管条件适用。Rust 仍负责 Shell、窗口布局、退出进程；脚本 q 只请求当前已委托窗口关闭。

这是应用接入迁移，不是 pre-1.0 兼容 bug。其 README 仍要求 `disktree-integration`，不能把当前 clone 直接作为 PR #104 主线的完整运行验收。`.operation_limit(...)` 也依赖尚在审查的 #96，需和另一分支的结论一起确定最终依赖组合。

### 其他已存在的入口

- `view.focus(window,cx)` 已在 mount 后使用，Host 管理 modifier chord，脚本注册 back/forward action；不需要给普通节点添加任意 modifier 解析绕行。
- Shell 通过 `last_error` 向 stderr 镜像诊断，是已有公开 API 的应用接入。它不是框架完全缺诊断入口。
- 83b19b1d 的原生关闭路径会 dispose 已挂载 view，即使应用仍持有 handle；任务/订阅/交互清理由 Runtime 管理。应用自己的后台共享状态写入仍须有业务代际，见下一节。
- PR #104 的 Table 列边界/末列/RTL 修复没有等同于 #83 自定义 grip 或 #89 context request。DiskTree 当前主要使用 Canvas 和自有 widgets，并不是这两个新 API 的实际使用证据。

## 2. 应用 P1：已取消旧扫描仍可覆盖当前 SharedTree

定位（均为 dogfood 源码）：

- `src/capability.rs:158`–`:175`：subscribe 准备阶段已启动 ScanHandle 和独立 forwarder，然后把 channel receiver 交给 SubscriptionWork。
- `:189`–`:201`：只有每隔约 100ms 的 progress send 失败才触发 `handle.cancel()`。
- `:203`–`:205`：在两次 progress send 之间可以取到完成结果。
- `:271`–`:272`：先直接覆盖全应用 `SharedTree`；`:287` 才发送 done，并忽略发送失败。
- `src/engine/mod.rs` 的 SharedTree 没有请求 epoch 或 active scan identity。

### 已运行的无扫描 fixture

快照保留原 `run_forwarder` 逻辑，仅给 ScanHandle 增加一个“预制结果 channel”测试入口，没有调用 `ScanHandle::spawn`、walk 或目录枚举：

1. 让原 forwarder 发出第一次 progress。
2. 在 SharedTree 放入代表新结果的 `/audit/new-generation`。
3. 丢弃旧订阅的 receiver，再向旧 ScanHandle 交付预制完成结果。
4. 旧 forwarder 在下次 progress send 前处理完成结果，SharedTree 实际变成 `/audit/nonexistent-old-generation`。

连接中的消费者正向对照能正常提交，证明并非 fixture 无法产生结果。取消反例稳定触发最终不变量失败，日志见 [probe-results.log](probe-results.log)。

### 影响

快速 rescan、修改 hidden/apparent 参数、切换皮肤/热更新、关闭扫描 effect 后，旧结果仍可能修改当前 Rust 树。框架即使正确抛弃旧 subscription callback，也无法撤销这个在 callback 之前发生的 Host 副作用。UI 的新 scan 摘要、crumbs、layout 与 tree capability 可能来自不同扫描代际。

### 交给应用实施 agent

- 扫描请求、候选结果和最终 SharedTree 都带同一 scan revision；只有仍有效的当前请求可以发布。
- 把 producer 的实际工作与 SubscriptionWork 生命周期绑定，避免在仅“构造工作”时就失去取消关联地启动独立线程。
- 优先在有效前台完成回调/Host 提交边界按 revision 发布候选树；大树仍留在 Rust，事件只携带 bounded request ID，不把整棵树塞给 Rhai。
- 单独把 `send(done)` 挪到赋值前并不足够：发送成功不代表此 activation 最终仍会被消费。
- 终态统计应在收到完成结果后读取/派生，避免使用本轮 poll 之前取的 progress snapshot。此统计时序另有源码风险，本轮未单独计为动态证实缺陷。

验收：A 开始→B 开始并完成→A 最后完成，只能显示 B；取消 A 后其完成不能提交；热更新/关闭 effect 同样不能提交；持续正常扫描的完成与进度仍可用。全部可用预制小树和 channel barriers 验证，不需要真实大目录。

## 3. 应用 P2：把显示名称当作原生路径身份，非 UTF-8 文件无法正确 round-trip

`src/engine/scan.rs:447`–`:450` 将非 UTF-8 名称 `to_string_lossy` 后存入 Node。扫描时还保留 DirEntry 原始路径，因此可统计该文件；但完成树只有 `Node.name: Box<str>`。`src/engine/node.rs:322`–`:334` 的 `path_of` 用这个显示名重新拼接路径，`TreeCapability::path` 又把结果返回为字符串给 reveal。

无文件系统访问的原生数据 fixture：

```text
原始路径       /audit/root/a\xFF
Node 重建路径  /audit/root/a�
相等           false
```

Linux 文件名允许这类字节；两个不同原始名称还可能显示成相同替代字符名称。显示层可以 lossy，Host 路径身份不能 lossy。

同一区域另有确定的输入边界：`path_of` 在 crumbs 越界时直接 break，`[99]` 悄悄返回 root；capability schema 也未排除负 index。这会把无效/陈旧引用解释成另一条真实路径，应返回错误或无结果，而不是打开某个祖先。

交接方案：Rust 树保留原始 OsString/PathBuf 或不透明 node/path identity，Rhai 只接收显示文本及可验证的 `scan_revision + node_id/crumbs`。reveal 在 Rust 当前树中验证身份并恢复原始路径。失效 crumbs、跨扫描 revision、负 index 和越界必须拒绝。无需要求 gpui-rhai 把任意原生路径直接暴露给脚本。

验收用内存节点和极小临时目录即可：非 UTF-8 两个不同名字、普通 Unicode、源节点删除、旧 revision、负/越界 crumbs。不要对真实用户树执行 destructive 操作。

## 4. 应用 P2：--dev 的文档命令没有启用热更新 feature

Dogfood `Cargo.toml:26`–`:29` 设置 `default=[]`，仅 `dev` feature 启用 `gpui-rhai/dev-reload`。README `:79` 却给出 `cargo run -- --dev`；CLI 的 `--dev` / `--skin-dev` 仅调用 `.development(true)`。

框架 watcher 由编译 feature 门控，运行时 bool 无法补回未编译的 watcher。该默认命令会读取文件皮肤，但没有文档承诺的热更新。这不是需要新增 file-watcher API。

实施：文档使用 `cargo run --features dev -- --dev`；不含 feature 的 binary 遇到 `--dev` 或 `--skin-dev` 应清楚拒绝或说明，不静默降级。保持 production 默认不开 watcher。验收各跑 default/dev 两种构建，观察改一个临时 skin 文件的实际 reload，并确认坏更新保留 last-good。

## 5. 权限、线程与尚未动态验证的边界

已看到合理做法：Rhai 不运行 scanner；扫描、统计和文件管理器启动位于 Rust capability；后台结果通过订阅前台回调消费；目录错误数量和消息有界；默认不跟随 symlink，文件系统边界和 hardlink 去重在 engine；reveal 使用 Command 参数而非 shell 拼接。

这里的 scan/open capability 由 Host 显式激活，能扫描用户权限允许的指定路径。皮肤选择不是“只有颜色修改”的无权限模板；权限策略属于应用 Host，本报告不把用户授权的文件探索功能误报成框架 sandbox 绕过。

需要后续测试但本轮不另列已证实缺陷：

- FilterCache 只缓存 Arc 地址整数且不持有该 Arc；跨多轮 rescan 存在 allocator ABA 的可能。应用可用显式 scan revision 统一解决，不应把指针地址当业务版本。
- tree.matches/children 的搜索、Matches clone 和子项排序是同步能力，Rhai operation budget 不限制其中的 Rust CPU 工作。应以合成大树测 UI 线程耗时，并给 `max` 设置 Host 侧上限。
- OpenCapability 的 TaskWork 不接取消 token，且只检查进程 spawn。若产品要求关闭后不再启动 reveal、或者要报告 xdg-open 非零退出，需要应用补明确语义。
- CLI 的 canonicalize 错误被归并成“没有 home/path”，以及 mount/open_window 后错误的退出状态，可另做轻量诊断测试。

本次没有启动完整 dogfood binary。旧 constructor 和 #96 API 依赖必须先迁移/确定；不能将这个纯 Host fixture 的成功/失败当作完整 GUI 或 #104 adapter 验收结果。

## 6. 给本机实施 agent 的执行顺序与开放 issue

1. **先接入当前窗口 adapter**：new + mount_window，保留 Host chords、明确焦点、系统关闭与 q；更新 UPSTREAM_GAPS G10 和冲突的 checklist 描述。配合另一分支确认 #96/#97/#99 的最终版本。不要重新引入 native registry escape hatch。
2. **修应用扫描提交与路径身份**：按上面两个 fixture 的正确不变量整改；加确定性取消/旧结果测试，再跑极小受控目录。扫描结果发布不需要新增框架异步原语。
3. **修 dev 构建说明和标志检查**，再验证受控 skin 的 reload。主题 system 的 Host 映射与脚本 paint cache 问题由脚本审查报告交接。
4. 再处理以下增强，避免把 dogfood 自身缺陷混入框架新增 API。

| Issue | 实施归属与优先级 | 本轮建议 | 可直接执行的验收要求 |
|---|---|---|---|
| #98 缓存图片刷新 | gpui-rhai 文档 + Cadenza Host，P2；通用 per-asset API 后续独立设计 | 0.1.8 补已有前台 refresh 桥与版本化模式说明，不因“后台 Rc 非 Send”改变 Runtime 线程模型 | MutableProvider 内容变化后同 handle 的 GPUI image id/实际像素变化；前台刷新并通知共享窗口；后台只传 ID/revision；坏文件 last-good、反序完成和 pending-only 的强一致契约须单独设计，不能谎称旧 namespace API 已保证 |
| #83 自定义 resize grip | 交互 Runtime + registry 组件，P2/0.1.9 | 保持独立设计，不随 #104 的 Table handle 定位修复宣称完成 | 默认外观回归；自定义装饰 overhang 在两面板之上且能命中；native drag/focus/disabled 状态、键盘 separator 语义不重复；RTL/缩放/主题 token；pointer move 不跑 Rhai |
| #89 Table context request | Table/registry，P2/0.1.9 | 单独加 row/cell 语义事件，复用已有 Menu.anchor，不新造 overlay 系统 | Array/NativeCollection 同契约；真实滚动后新行也能触发；稳定 row key、可空 column key、window logical anchor；右键不冒充左键 row_click；若纳入 Shift+F10，明确实际焦点及 selection 策略 |

DiskTree 本身未提供 #98 图片缓存或 #83/#89 组件的使用证据，这些优先级仍来自 Cadenza / oh-my-byted。先前详论可参考 [issues-assets 报告](../../2026-10-01-dogfooding-pr-triage/issues-assets/report.zh-CN.md)。本报告只形成执行交接，不代表获准实施或关闭 GitHub issue。

## 证据

- [baseline.json](baseline.json)：两个仓库版本和只读边界。
- [snapshot.json](snapshot.json)：原始源文件散列和测试接缝说明。
- [probe-results.log](probe-results.log)：纯内存/路径 fixture 的实际输出；最终正确性断言失败属于上述应用反例。
- [probe/src/main.rs](probe/src/main.rs)：入口。快照只给 ScanHandle 注入结果 channel 并追加 driver；原 run_forwarder 与 path_of 逻辑不变。

```sh
cargo run \
  --manifest-path docs/audits/2026-10-02-interaction-0.1.8-round6/dogfood-host/probe/Cargo.toml \
  --target-dir tests/native-keyboard/target --offline
```

该命令不执行 filesystem walk，只对不存在的审计路径做 forwarder 自带的空间/设备查询并处理预制树。修复后应把期望保留最新树、原始路径身份的断言转绿。
