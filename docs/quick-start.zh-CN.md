# 快速开始

GPUI Rhai 的 Rust runtime 负责 GPUI、生命周期与安全边界；组件、主题、locale
和小型资源会复制到你的项目中，由你直接修改和维护。项目完全不依赖
`gpui-component`。

完整的使用方式、架构边界与 agent 工作规范见
[User Guide](../USER_GUIDE.md)；这里仅保留最短上手路径。CLI 尚未发布到
crates.io，也没有正式 release；请在仓库 checkout 中安装：

```text
cargo install --path crates/gpui-rhai-cli
```

```text
gpui-rhai init
gpui-rhai add button input dropdown dialog
gpui-rhai check
gpui-rhai dev
```

首次正式发布前，`init` 写入的 `version = "0.1"` 尚不能从 crates.io 解析。
dogfooding 时请将目标项目的依赖改为本地 checkout，或有权限访问的固定 Git
commit；不要无意中跟随不断变化的 `main`：

```toml
gpui-rhai = { path = "/path/to/gpui-rhai/crates/gpui-rhai", features = ["dev-reload"] }
```

另一台有私有仓库权限的机器可使用：

```toml
gpui-rhai = { git = "https://github.com/eddix/gpui-rhai", rev = "<commit>", features = ["dev-reload"] }
```

`check` 会用同一份 Engine 函数元数据检查所有 Rhai AST 中已知函数与参数数量
（包括首帧未执行的分支），随后加载真实主题、locale、资源和状态，执行一次完整的
无窗口首帧生命周期。Rhai 是动态分派语言，因此运行时的类型检查仍然是最终依据。

在 `ui/main.rhai` 中通过别名导入组件，并从 `view(ctx)` 返回 `UiNode`：

```rhai
import "components/button" as button;

fn state_schema() {
    #{ fields: #{ count: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } } } }
}

fn clicked(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }

fn view(ctx) {
    button::Button(#{ key: "continue", text: "继续", on_click: Fn("clicked") })
}
```

`view` 只能描述 UI，不能执行副作用。文件、网络、持久化和平台服务必须由
Rust host 注册为带 schema 和版本的 capability。需要发布时运行
`gpui-rhai embed`，使用生成的 embedded source 构建 release；不要在生产环境
依赖任意文件路径或动态下载脚本。

可跨 render 保存的回调必须是命名且不捕获环境的函数。Rhai 的编译只检查语法，
不会证明所有动态函数重载都存在；`gpui-rhai check` 会额外检查已知调用并真实执行
首帧，但事件分支仍应以真实 payload 类型测试。

Rust 侧以脚本视图为核心：`FileScriptView` / `EmbeddedScriptView` 先生成
`PreparedScriptView`。独立应用交给 `ScriptApplication` 打开窗口；已有 GPUI
应用则通过一个共享的 `ScriptViewHost` 在同一窗口挂载多个相互隔离的视图。
脚本 view 作为 Rust flex row/column 的直接子项时应使用 `view.flex_item()`；固定、
绝对、grid 或自行控制布局时才使用较底层的 `view.element()`。

节点事件中需要当前控件的窗口坐标时，使用事件时快照，不要建立 resize → store
通道：

```rhai
fn clicked(ctx, payload) {
    let bounds = ctx.event_target_bounds();
    // #{ x, y, width, height } | ()
}
```

原始 pointer/wheel payload 同时提供 `payload.target`。Rust 原生 handler 使用
`NativeEvent::target`。这些值不建立 render 依赖；真正需要跨 render 追踪另一元素
布局时，才使用 `element_ref(...)` 与 `ctx.element_bounds(ref)`。

更新已复制的组件前先运行 `gpui-rhai diff`。`update` 使用三方合并，冲突会写入
`.gpui-rhai/conflicts/`，不会覆盖你的源文件。

进一步阅读：[文档索引](README.md)、[组件编写](component-authoring-guide.md)、
[嵌入](embedding.md)、[主题](theming.md)、[性能](performance.md)与
[安全边界](security-boundary.md)。
