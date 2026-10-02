# PR #99：读取实际主题 variant 的评审

- PR：[feat(context): let scripts read the resolved theme variant](https://github.com/eddix/gpui-rhai/pull/99)
- 审查头：`3209039e2d792dde9f65daa7eb2600cd90a3e3f6`，PR base `815bb82b`。
- 工作区存在其他会话未提交的产品改动，**本报告行为证据来自固定 PR commit 的独立 git archive snapshot**，不使用 live workspace 编译结果。
- 证据：[PR diff](../evidence/pr-99.diff)、[原生 probe](probe.rs)、[执行结果](probe.log)、[复现脚本](run-probe.py)。

## 决策

**建议纳入 0.1.8，由主开发接管集成；不原样合并当前 PR。** 保留作者新增只读查询的方向，把它连同初始 WindowAppearance 的主题环境初始化点一起收口。这个范围有限，无需重写主题系统或 effect 模型。

如果仅看 getter、文档和 clone 开销，作者小改即可；但实际跑到 System 模式的首帧后，查询承诺依赖一个现存的宿主生命周期缺口。让主开发在当前 0.1.8 主线上修复该边界，比要求 dogfooding 作者同时处理与其较旧 base 的交互整改更合适。

它解决的是有实际使用场景的环境读取缺口：Canvas 等脚本自算颜色需要知道最终 light/dark。不是引入 GPUI 对象、平台 API 或新权限。可进入本轮，但不能以“有一个固定 Dark 的 Rust 单测”证明 System 解析已经完整。

## 1. System 初次进入 Light 窗口，脚本读取仍是 dark

**PR head 原生实测**：安装支持 Dark/Light 的主题家族，app preference 为 System；GPUI 测试窗口原生 appearance 为 Light。root render 只执行：

```rhai
fn view(ctx) {
    text(ctx.theme_variant().mode).accessibility_role("status")
}
```

结果：`view.theme_snapshot().variant.name == "Light"`，但脚本文本一直为 `"dark"`。已经推进 GPUI 测试后台时钟 4×16ms，不是 dirty scheduler 尚未被驱动的误报。

**来源与归因**：

- 新 getter `context.rs:2687-2691` 沿用现有 motion_tokens 的路径：没有记录 window appearance 时默认 Dark。
- 初次 mount/render 尚未将实际 GPUI WindowAppearance 写入 runtime。
- PR head `app.rs:3988-3996` 第一次插入 Light 得到 `None`，`.is_some_and(...)` 返回 false，没有使之前按 Dark 读取的组件失效。
- Chrome 用实际 Light 解析，脚本先前的 Dark 计算结果继续被复用。

这是**已有 appearance 初始化路径的问题被新 API 暴露**，不是声称这个 PR 差分新引入了 app.rs 缺陷。但是它直接影响 PR 声明要解决的 System 皮肤同步，必须与查询一起补齐，或在交付中明确不支持尚未确定窗口 appearance 的阶段；不能把错误结果当成“已经 resolved”。

**必要修订**：在已知真实 window appearance 后、第一次 window-scoped Rhai render 前建立该环境；或保证首次实际 appearance 与 fallback 不同会使所有依赖读者可靠重算。补 initial Light/initial Dark、后续系统切换、固定主题不受系统模式影响的原生验收。避免靠某个无关 state 更新碰巧触发正确结果。

探针：`initial_system_light_matches_script_resolved_theme`，当前失败。

## 2. 文档把组件 invalidation 写成了 effect body 自动重启

PR head `context.rs:2675-2677`、`:2705-2706` 和 `docs/theming.md:122-126` 宣称 effect 读取此值后会随主题变化重跑。实际 `track_theme` 记录 component reader（`environment_dependency.rs:47-58`），不是 effect activation 的独立响应式依赖。`effect.rs:170-176` 对 descriptor/deps 相同的 effect 保持原 activation。

**固定 PR head 对照**：

| 读取方式 | Dark 初始状态 | Host 正式 select_theme("Default", "Light") 后 |
| --- | --- | --- |
| effect deps 为 `()`，只在 start body 里读 `ctx.theme_variant()` | starts=1、seen=dark | Chrome Light，但仍 starts=1、seen=dark |
| render 时读查询结果，并作为 effect deps | starts=1、seen=dark | starts=2、seen=light |

两组都推进同样的 GPUI 后台时钟。第二组是通过的正向对照，排除了 Host setter、测试调度或注册方法根本没有工作的可能。

**必要修订**：保持现有显式 effect deps 模型，修正文档／Rust 注释，给出正确用法：

```rhai
fn render_Palette(ctx, props) {
    let variant = ctx.theme_variant();
    effect("palette", variant, Fn("start_palette"), Fn("stop_palette"));
    // ...
}
fn start_palette(ctx, variant) {
    // 根据 variant.mode 计算或更新 palette。
}
```

准确承诺应是“render 中读取会登记主题环境依赖；依赖变化重渲染该组件；传给 effect 的 deps 变化后 effect 才重启”。不要为了让一段新文档成立，顺手增加 effect body 自动跟踪或重写所有现有 effect 的语义。

探针：`body_read_restarts_effect_as_pr_docs_claim` 当前失败；`explicit_render_dependency_control` 通过。

## 3. 小 API 改动不应让现有 motion getter 复制整个主题

PR head `context.rs:2700` 将完整 `ThemeVariant` 克隆出来；`:2669-2671` 的 motion_tokens 再从完整副本中取 motion；`:2708-2710` 仅取两个名字也先复制全部 tokens。Rhai `theme_variant` 返回三个字符串，却同样支付完整 colors/maps/typography 等复制成本。

原 motion_tokens 只克隆 ThemeMotion。这是可以直接从 diff 确认的分配范围回归；本报告没有编造毫秒性能影响，也没有为这个小改动运行大型 benchmark。

**必要小改**：内部用一个 scope-aware borrowed resolver/projection helper，保持 track_theme 和 appearance 解析在一处；metadata getter 只复制 family/name/mode，motion getter 只复制 motion。若确有 Rust 使用方需要完整 ThemeVariant，可保留单独的显式快照 API，但不要把所有轻量读取都绕进它。

## API、作用域、Host overrides 与权限判断

- `#{family,name,mode}` 作为 Rhai 返回结构合理；`name` 对应 ThemeVariant.name，`mode` 用稳定的 `light`/`dark`，不把主题展示名当枚举。System preference 应返回实际选中的 Light/Dark variant，而不是返回字符串 `system`。
- `ThemeManager::resolve(window, component, appearance)` 已实现 nearest local scope → window → app 的层级；getter 调用正确解析器，方向对。PR 新测试只测固定 app 选择，必须补 window override/local subtree 与兄弟 scope 的隔离验证。
- Host token overrides 在安装主题时合入 variant；现有设计禁止覆盖 identity/mode。返回的 mode 是主题语义元数据，不是根据最终背景颜色推断的“明暗”。如果 Host 改了一些颜色而保持 Dark identity，返回 dark 合理。这个 PR 不等同于脚本获得了最终 RGBA token 查询接口。
- 只读当前 UI 的主题元数据不需要新的 capability、文件或系统配置读取权限。它不修改 Host overrides、偏好持久化或系统 appearance；不应增加权限弹窗或新 capability namespace。
- `theme_selection() -> Option<(String,String)>` 是额外 Rust convenience API，实际返回 resolved identity，名字容易与已有 ThemePreference/ThemeSelection 混读。可以删除未用的 convenience method，或采用现有 typed selection 并清楚命名；这是 API 收敛建议，不是本轮行为失败的根因。
- `()` 表示主题不可用应保留文档说明；当前实现还会在 borrow/resolve 失败时返回 None，不能将其说明成“永远仅代表尚未安装主题”。

## 合入前清单

1. 主开发修复/验证初次 System appearance 的 window-scoped 初始化与 invalidation。
2. 作者或主开发改正 effect 说明，增加“render deps 正向／body-only 不自动重启”的契约测试。
3. 去掉轻量 getter 的全主题 clone 回归。
4. 新增真正运行 `ctx.theme_variant()` 的 Rhai 测试，包含 map 字段、mode、缺少主题、System、window/local 层级与 Host overrides；现 PR 只有 Rust Event-phase 固定选择测试。
5. 修掉 `context.rs:4428` 测试里的多余 `mut`。总审查已确认 CI portable 因严格 Clippy 的 unused_mut 失败；此前 check 通过不代表整条 CI 验收通过。

不应扩张到：自动保存偏好、主题文件监控、第三方主题导入、自动 effect body 跟踪、新颜色 schema、统一 token 数值查询框架。这些都不是补只读 resolved variant 所必需的。

## 复现与可信度

```sh
python3 docs/audits/2026-10-01-dogfooding-pr-triage/pr-99/run-probe.py
```

runner 从固定 PR commit 归档源码，在本目录建立唯一临时 snapshot；只调整临时 workspace members 以排除不相关 CLI，产品 Rust/Rhai 源码原样编译；使用共享 `tests/native-keyboard/target`，退出清除 snapshot。`Cargo.toml` 是 runner 模板，不应直接对 live workspace 执行它。

结果：3 项原生测试，1 个显式 deps 对照通过、2 个承诺场景失败。没有测试 PR 作者尚未提交的修改，没有给失败 CI 下的整条 workspace 测试作通过背书。
