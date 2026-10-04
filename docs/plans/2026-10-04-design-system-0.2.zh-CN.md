# gpui-rhai 0.2.0 设计系统重做实施计划

日期：2026-10-04。分支：`feat/design-system-0.2`（自 `v0.1.8` / `d87ebb9`）。状态：五步实施完成，待维护者评审；未推送、未发版。

本计划来自 2026-10-04 与维护者的设计 grilling 会话。共识的正式内容写在
[`docs/design/`](../design/principles.md)，本文件只跟踪执行顺序、完成证据和遗留事项。
细节决策记入 [`docs/design/decisions.md`](../design/decisions.md)。

## 授权边界

- 一口气完成下列五步，步骤之间不停下评审。
- 按主题攒批提交到本分支；不推送、不合并到 `main`、不发版、不打 tag。
- 只在三种情况停下问维护者：某条共识被证明做不到、必须改方向；破坏性或对外操作；需要维护者本人操作（权限等）。
- pre-1.0 有意破坏性升级：版本 0.2.0，`runtime_api` 2→3，不保留历史兼容别名或双轨实现。
- 不修改 `oh-my-byted`、`disktree-rhai` 等外部仓库；disktree-rhai 只用 `cargo --config` 临时指向本分支做冒烟。

## 五步

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | `docs/design/` 四份规范 + 决策日志 + Gallery 线框；延迟构造 ADR | 完成（c232500） |
| 2 | 运行时地基：开放 token 与字阶、token 基础层、组件声明 token、声明式环境值（原生解析）与原生继承 disabled、颜色派生与读取、Rust 派生迁到 L0、action 快捷键查询、组合审计、CLI profile | 完成（3224767、64108b0） |
| 3 | L0 `tokens.rhai` + 内置主题只写颜色 + 新默认主题；全部 62 个组件的组合契约与新视觉 | 完成（见进度记录） |
| 4 | L2：`layouts/`（Stack/Inline/Toolbar/Region）与 `patterns/`（Section/DescriptionList/Stat/FormLayout/InlineState/DataView/ListDetail/AppShell） | 完成（见进度记录） |
| 5 | 用 AppShell + L2 重做 Gallery（规格页 + 场景页）；自带设计参照示例进 CI；审计零告警；纯键盘场景测试；重拍基线；文档、CHANGELOG、迁移说明 | 完成（见进度记录） |

## 每批验收命令

与 CI 一致：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --manifest-path tests/performance/Cargo.toml --locked
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked
bash scripts/audit-visual-baselines.sh
```

视觉改动另外截图自查；Linux 只能在推送后由 CI 验证（本机为 macOS）。

## 进度记录

（按批次追加：日期、提交、内容、证据。）

### 2026-10-04 第 3 步：62 个组件按 0.2 契约重做

- 内容：全部 62 个组件升到 0.2.0，声明 `tokens` / `environment`；尺寸与密度全部走
  `metrics.*`；标记系统（Button / Tag facet / Badge 方灯 / Kbd 键帽）；字段统一 2px 框；
  列表行语法（`metrics.row`、`metrics.inset`、指示条）；Table 无外框、标签语气表头、数字列；
  Tabs 轨道 + 浮起拇指；ToggleGroup 漫游焦点；覆盖层统一为浮起色块 + 发丝线。
- 为此补的运行时能力：`group_focus` / `focus_within` 样式、带焦点样式节点的 tab stop 修复、
  覆盖层横轴 `align`、fill-height 虚拟列表延后首次 reveal、`ScriptApplication` 致命错误写 stderr、
  滑块拇指按锚点居中并解析焦点态。决策见 `docs/design/decisions.md` D8–D19。
- 截图自查（本机 macOS，默认深浅两套主题）：markers、fields、lists 三张规格图，紧凑档高度
  量过（md 28、sm 24、xs 20）。
- 证据：`cargo fmt --all --check` 通过；`cargo clippy --workspace --all-targets --all-features
  -- -D warnings` 无输出；workspace 667 passed / 0 failed；native-keyboard 213 passed / 0 failed；
  performance 2 passed / 0 failed；`scripts/audit-visual-baselines.sh` 通过（69 张，基线待第 5 步重拍）。

### 2026-10-04 第 4 步：L2 layouts 与 patterns

- 内容：`layouts/` 4 个（Stack、Inline、Toolbar、Region）与 `patterns/` 8 个（Section、
  DescriptionList、Stat + `stats()`、FormLayout、InlineState、DataView、ListDetail、AppShell），
  登记进 `BUNDLED_LAYOUT_SOURCES_BY_ID` / `BUNDLED_PATTERN_SOURCES_BY_ID`，registry lint 覆盖到 L2。
- AppShell 区域键盘模型：F6 / Shift+F6 在侧栏、主区、检查器之间移动焦点；区域自身持焦点时
  显示 2px 墨色框，Tab 进入区域内第一个控件。
- 运行时：键处理名支持修饰键（`shift+f6`、`cmd+k`，无修饰名仍对任意修饰生效）；
  `audit_allow([...])` 显式豁免；审计规则按真实组合校正：同轴才比较嵌套间距、
  `justify_between` 的 gap 视为下限、标题引领的内容向外层看齐、控件与标记按自身边缘对齐、
  字体检查认可声明的回退链、原生输入框参与同行字号检查。
- 组合修正（审计发现的真问题）：区域内的 section 之间用 `section` 间距；DataView 让表格
  `bleed` 到区域两侧，行文字与标题同一条边。
- 证据：`tests/native-keyboard/tests/l2_patterns.rs` 两项：完整 AppShell 组合在
  productivity 规则下零发现（测试平台无系统字体，字体规则交给 Gallery 在真实平台上验）；
  F6 依次框住三个区域、Shift+F6 返回上一个。fmt / clippy（all-features）无输出；
  workspace 667 / 0，native-keyboard 215 / 0，performance 2 / 0。

### 2026-10-04 第 5 步：Gallery 验收应用、BYOD 示例、基线与文档

- 提交：53caeca（组件与运行时修正）、416b8c9（Gallery）、7339995（BYOD 示例）、
  34c0db1（离屏基线）及本文档提交。
- Gallery：`registry/gallery/` 下的 Rhai 应用，基于 AppShell + L2；83 页 / 11 组
  （Foundations 4、Markers 8、Fields 14、Lists 9、Overlays 6、Containers 11、Display 7、
  Interaction 7、Layouts and patterns 12、Scenes 4、Effects 1，合计 83）；四个场景
  （运维、数据浏览、表单、设置）、命令面板、密度/明暗/语言切换、源码检查器、状态栏实时审计数。
  Host（`gpui-rhai-cli` 的 `acceptance` 模块）只提供 Cmd+K 绑定、页面源码文档、审计计数。
  旧 Rust 外壳 `gallery_app` 删除；`--story` 改为无外壳的独立 story 窗口。
- 审计与视觉复查发现并修复的问题（决策 D20–D33）：RTL 下 `justify_start/end` 不镜像（运行时）；
  TitleBar 两行标题在紧凑档被裁；0.2 去掉 Alert 图标后 `info`/`warning` 无人声明、画不出来；
  Slider 数值漂到容器最右；Table 状态列文字与徽标重复；ghost/outline 按钮禁用时多出底色；
  stacked DescriptionList 无分组；弹层触发按钮与 ToggleGroup 被拉满整行。
- 排查记录：命令面板“打不进字”的根因是 Gallery 入口脚本的 `fn index_of(values, key)`
  劫持了 Command 组件里的 `s.index_of(c, pos)`（Rhai 脚本函数可按方法调用且优先于内置函数），
  渲染失败后事务回滚、`last_error` 又被下一次成功渲染清空，表现为静默。已改名并写入
  composition.md、迁移说明和 AgentNotes。
- BYOD：`examples/byod_treemap` 自带调色板、无 token 基础层、无官方组件；进 verification
  manifest，CI 构建、release smoke 启动。disktree-rhai 在临时副本中经 `cargo --config` 冒烟：
  需要三处改动（runtime_api 2→3 是唯一由 0.2 引起的），45 个测试通过、自有皮肤渲染正常；
  未修改其仓库。oh-my-byted 只读扫描，迁移要点写入 `docs/releases/0.2.0.md`，未修改其仓库。
- 基线：本机运行着平铺窗口管理器 rift，会把新窗口改成 852×1067，屏幕截图不可控；改用
  gpui-pre 的 `VisualTestAppContext` 离屏窗口 + `Window::render_to_image` 读回 GPU 纹理。
  19 张 2560×1720（4 页 × 两档密度 × 明暗 + 1 张阿拉伯语 RTL + 2 张简体中文），替换旧外壳的
  31 张；总数 57 = 示例 38 + Gallery 19。**示例的 38 张仍是 0.1.x 外观，待单独重拍。**
- 文档：`docs/gallery.md` 重写；`docs/design/` 补 atoms/composition/decisions/wireframes；
  `registry-design-system.md` 改为指向 `docs/design/`；theming、authoring、locale-and-rtl、
  visual-testing 更新；CHANGELOG 0.2.0（Unreleased）；`docs/releases/0.2.0.md` 含迁移说明与
  disktree / omb 实地报告。crate 版本号仍为 0.1.8，按发版流程再改。
- 证据：`cargo fmt --all --check` 通过（native-keyboard、performance 同）；
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` 退出码 0，
  native-keyboard 与 performance 的 clippy 退出码 0；workspace 672 passed / 0 failed
  （第 4 步 667 + BYOD 2 + acceptance 单测 2 + 资产声明测试 1）；native-keyboard 219 passed /
  0 failed（215 + gallery_acceptance 5 − 删除的旧外壳测试 1）；performance 2 / 0；
  `scripts/audit-visual-baselines.sh` 通过（57 张）；`verify-target-manifest.py` 21 个示例；
  release 版 `gallery --list` 含所需 page/story，`--page scene.operations --density compact`
  与 `--story apps/operations --case large` 各运行 3 秒退出码 0、无报错、启动目录无写入。
