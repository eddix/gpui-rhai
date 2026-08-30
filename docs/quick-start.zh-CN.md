# 快速开始

GPUI Rhai 的 Rust runtime 负责 GPUI、生命周期与安全边界；组件、主题、locale
和小型资源会复制到你的项目中，由你直接修改和维护。项目完全不依赖
`gpui-component`。

```text
gpui-rhai init
gpui-rhai add button input dropdown dialog
gpui-rhai check
gpui-rhai dev
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
    button::Button(#{ text: "继续", on_click: Fn("clicked") })
}
```

`view` 只能描述 UI，不能执行副作用。文件、网络、持久化和平台服务必须由
Rust host 注册为带 schema 和版本的 capability。需要发布时运行
`gpui-rhai embed`，使用生成的 embedded source 构建 release；不要在生产环境
依赖任意文件路径或动态下载脚本。

Rust 侧以脚本视图为核心：`FileScriptView` / `EmbeddedScriptView` 先生成
`PreparedScriptView`。独立应用交给 `ScriptApplication` 打开窗口；已有 GPUI
应用则通过一个共享的 `ScriptViewHost` 在同一窗口挂载多个相互隔离的视图。

更新已复制的组件前先运行 `gpui-rhai diff`。`update` 使用三方合并，冲突会写入
`.gpui-rhai/conflicts/`，不会覆盖你的源文件。

进一步阅读：组件编写、主题、locale/RTL、capability、自定义 primitive、热更新与
生产嵌入，以及安全边界文档。
