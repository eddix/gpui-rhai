# 第二轮工具边界审查

基线：`d77b4b49a667da618fe9b9583d331d8c325cf180`。日期：2026-09-30。
未修改产品、正式测试、旧审计或 GitHub 状态。

## T2-1 · P2：#90 的最小复现已修，但依赖 init 状态的合法应用仍被 check 错拒

`Project::check` 现在先解析 manifest，对声明了 capability 的应用不运行 init，并准确提示 Host 生命周期未验证。这修复了上一轮的简单 `init 调 capability + view 返回静态文字`，本轮对照也通过。

但是 `crates/gpui-rhai-cli/src/lib.rs:447–450` 仅依据 manifest 是否有任意 capability 选择该分支；`lifecycle.rs:181–200` 的 `validate_initial_view_without_init` 将状态直接改为 Initialized，再实际执行 view。它把“尚未初始化的运行环境”当成“应用的合法初始运行环境”。

最小反例只涉及公开 state API，不需要伪造 Host capability 数据：

```rhai
fn state_schema() {
    #{fields: #{ready: #{schema: #{type: "bool"},
        "default": #{type: "bool", value: false}}}}
}
fn init(ctx) { ctx.set_state("ready", true); }
fn view(ctx) {
    if !ctx.get_state("ready") { throw "init must prepare view state"; }
    text("ready")
}
```

同一份脚本，没有 capability 声明时 check 成功；仅在 `ui/app.toml` 声明 `app.demo="*"` 后，check 退出 1，报 `init must prepare view state`。init 本身甚至不调用 capability。正常 ScriptLifecycle 明确先 initialize 再 render，因此这不是应用违反生命周期契约。

实际应用会以 init 建立 store、首屏数据、资源引用或派生状态，错误表现不一定是显式 throw。不能要求应用为 CLI 增加一条实际运行从不会出现的“未初始化 view”兼容路径。

整改建议：将静态分析与执行验证分开。在没有 Host context 的模式下，不把缺初始化环境引起的动态失败认定为应用无效；更不能通过捕获任意 view 错误来猜哪些失败安全可忽略。默认静态模式可以验证语法、导入、manifest、已知调用和可静态验证的 schema，并明确 view/lifecycle 尚未运行；完整执行验证由应用提供明确的 Host fixture。若有其他可证明安全的执行模式，应独立声明前提。

验收包含：旧 #90 最小复现；init 只初始化本地状态；init 从 Host 取得首屏数据；声明 capability 但 init 不调用；静态可发现错误；真正的 view 错误在 Host 执行验收中仍被拒绝。

证据：[cli-repro.py](cli-repro.py)、[cli-results.json](cli-results.json)。脚本重新构建当前 CLI，在 TemporaryDirectory 中建项目，源码不写入用户应用。输出为现状表征，不把出现错误的 case 当成修复通过。建议在 #90 下继续跟踪这一剩余边界，而不是声称原问题完全闭环。

## T2-2 · P2：native Automation 返回成功时，实际 callback 尚未执行且可能失败

位置：`app.rs:3919–3945` 的 native key fallback；`resizable.rs:664–695` 的 keyboard policy；`primitive.rs:906–918` 的 deferred `propose`。

新增 `perform_key` 确实让 `key:right` 能驱动 Resizable，这一能力已经存在。问题在于它调用 `events.propose`，只登记一个 window.defer，就返回 true；Automation 随即报告 invoked=1 的成功 Dispatch。实际 Rhai callback 要在之后才执行。

使用同一个会 `throw "audit-tooling-callback-failed"` 的业务 callback 作原生对照：

| 路径 | Automation 返回 | 排空事件后 |
|---|---|---|
| 普通声明式 click | `Err(Automation(Command(...)))` | 失败准确返回 |
| Resizable native `key:right` | `Ok(Dispatch { invoked:1, ... })` | `view.last_error` 才出现相同 callback 错误 |

这违反 [automation.md](../../../automation.md) 的明确契约：传播与执行结果分离，callback／rerender 失败应使 automate 返回错误。对依赖返回码的 Host 自测而言，它会记录一次成功，而实际动作没有成功完成。

整改应继续复用同一 native 尺寸政策，但区分“产生提议”和“完成受控 callback”。同步 Automation 需要拿到语义执行结果；若改成异步命令，则必须有明确的 queued/completed/failed 与关联 ID，不能保留现有成功返回的含义。不要在 native 计算路径复制 Rhai 版本几何，也不要在已经更新的 Entity 中强行同步重入。

验收：正常接受、受控拒绝、callback throw、rerender invalid、disable、过期目标和有界错误；公共 Automation 的结果与最终状态一致。原生 keyboard 的真实输入行为保持现有路线。

证据：[probes.rs](probes.rs)、[native-probes.log](native-probes.log)。2 项原生 probe：1 正向通过、1 正确性断言失败。

## 运行

```sh
python3 docs/audits/2026-09-30-interaction-0.1.8-round2/tooling/cli-repro.py
python3 docs/audits/2026-09-30-interaction-0.1.8-round2/tooling/run-native.py
```

两个 runner 使用唯一的临时目录，native test target 为 `round2_tooling`，与其他并行审查不重名。未执行真实桌面、AX 或全量发布门禁。
