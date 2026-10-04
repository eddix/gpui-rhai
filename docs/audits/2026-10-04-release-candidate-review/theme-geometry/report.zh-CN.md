# PR #108 · D3 Theme 与 ElementRef 验收

审查基线：`736025a8b2b1ac44d41d893bc9a929c990b97532`，对照 R7 `cda11ce7`。**本子域未发现新的已确认问题；已验证的 Theme/geometry 契约可以通过。** 这是范围内结论，不代替整个 RC 的其他验收门槛。

本轮独立小组合 **16/16 通过**，包括原 R7 三个失败场景及两个正对照、D3 五项 core 与六项 native scoped tests。产品和历史材料未修改。

## 本轮亲自执行的证据

- [源码/测试组合](src/lib.rs)、[原始日志](probes.log)
- [manifest](Cargo.toml)、[锁文件](Cargo.lock)
- [基线、源码等价与结构化复核结果](verification.json)

```sh
CARGO_TARGET_DIR=tests/native-keyboard/target cargo test \
  --manifest-path docs/audits/2026-10-04-release-candidate-review/theme-geometry/Cargo.toml \
  --locked --offline --lib -- --nocapture
```

原 R7 五项探针内容保持原样，组合文件只在后面引用当前正式 `resolved_theme` core/native 测试文件，没有修改那些正式测试。保留原先明确推进32ms后台 clock 的 settle 行为；没有设置更大的测试栈或建立新 profile/cache。

| 覆盖 | 本轮结果 |
|---|---|
| ref 先未绑定，独立 Producer 后续显示目标 | Reader 从 pending 更新为120 |
| 同一 ref 从 Text 换绑 Box | NodeId 4→6，Reader 从120更新为180 |
| 目标解除绑定 | Reader 从180更新为pending |
| 同 NodeId 几何变化 | 正常更新到180 |
| 虚拟行正式组件首次 geometry read | 初始pending在prepaint后正常变为120 |
| lightweight theme identity | family/name/mode恰好三个字段；缺主题或借用不可用返回None/() |
| preference 与 resolved identity | System偏好和解析结果明确区分；headless Dark fallback有定义 |
| scope | window、nearest local、siblings和Host/Rhai setter一致 |
| 首次原生主题 | TestPlatform初始Light在init/render/Host snapshot一致，init只执行一次 |
| effects | effect body查询不隐式订阅；显式render deps才按元数据变化重启；同值不重复激活 |
| fixed/token override | 固定Dark及Host token override保留正确identity，不从颜色推断mode |
| secondary startup | init/effect看到首次native外观；失败启动/重试与native authority保持边界 |

这里的native tests使用TestAppContext。它们不是实际App空闲外观切换的替代品。

## D3 代码审查

### 元数据投影与订阅

`ThemeVariantInfo` 只含两个String和mode。`UiContext::with_resolved_theme` 经过现有 ThemeManager 的 window/component scope 解析；ThemeManager返回借用的 `ResolvedTheme`，投影过程没有克隆整份token表。

只有Render phase建立theme read dependency；Init/Event/effect body查询不会凭空创造长期响应关系。显式effect deps包含metadata时，由既有依赖值语义决定是否重启。fallback、scope选择、Host token overrides没有另起一套theme状态。

### 首次挂载与真实 native appearance ingress

共同mount路径在init/effect之前，把 `Window::appearance()` 写入所属runtime的权威map。secondary也在真实native窗口创建后先登记appearance和reservation，再instantiate，不通过重放init修补错误初值。

`install_window_lifecycle_hooks` 为每个View持有appearance Subscription。锁定GPUI0.3.7的 `observe_window_appearance` 内部使用WeakEntity；产品没有detach observer或捕获strong self。

`on_window_appearance_changed`：

- Disposed早退；release_view显式take/drop该subscription。
- 先更新当前runtime map，并对相同SystemAppearance去重。
- Active通过weak defer进入正常poll，另发native notify；因此token-only View也能获得native重绘机会，不需要伪造Rhai theme reader。
- Suspended保留最新map/dirty，不在observer里运行Rhai、启动effect或动画；resume使用已更新环境。

旧observer的延迟poll仍检查生命周期；同ID remount不借用旧Entity的observer或权限。多个普通View各自安装入口，不依赖唯一window command owner代发事件。

## 实际Mac外观切换：明确复核旧实机证据，不冒称本轮重跑

已完整阅读 [H-THEME-01失败报告](../../2026-10-02-release-convergence/final-review/theme-appearance-ingress-p1.zh-CN.md) 与 [be7最终审查](../../2026-10-02-release-convergence/final-review/final-be7a8eb8.zh-CN.md)。旧问题是**真正空闲后没有appearance ingress**，不是单纯缺少一次手动refresh；没有把startup有遗留帧的早期绿色日志当作关闭依据。

本轮验证 `53a62e3f` 到当前 HEAD 的 **crates、registry、Cargo.toml、Cargo.lock、rust-toolchain.toml 无差异**。同时复核be7实际probe源码与以下JSON断言，记录文件hash在verification.json中：

- [initial Light](../../2026-10-02-release-convergence/final-review/evidence/appearance-light-be7a8eb8.json)：7阶段，两个真实750ms idle阶段timing/dirty为0、无pending virtual request，Light→Dark→Light正确，cleanup剩余自有窗口0。
- [initial Dark](../../2026-10-02-release-convergence/final-review/evidence/appearance-dark-be7a8eb8.json)：反向序列同样通过，cleanup剩余自有窗口0。
- [lifecycle](../../2026-10-02-release-convergence/final-review/evidence/appearance-lifecycle-be7a8eb8.json)：passed=true，error=null；同Window的独立A/B Runtime与固定F、same-mode去重、suspend→appearance→resume、保留旧Disposed Handle后同ID remount、token-only T均覆盖。四段600ms idle无Rhai timing/records变化；token-only appearance后Rhai timing为0。

这些实机结果由此前独立reviewer在be7执行，本轮只是核对源码等价、probe流程与结构化原始材料。**本轮没有启动新的真实Mac App、没有改系统主题或Rift、没有产生新截图。** 既有probe采用per-App native override，不等同于用户OS全局偏好切换；token-only记录证明native重绘路径/Host snapshot及geometry，不冒称GPU颜色截图验收。

因此不重报已修复的 H-THEME-01，也不把已有证据重新标为当前HEAD亲自执行。

## R7 ElementRef 长期订阅闭环

当前 `ElementRefRegistry.geometry_readers` 保留稳定RefId到ReadDependency的关系；reconcile比较旧/新binding，分别通知 unresolved→resolved、NodeId替换、resolved→unresolved的真实owner，不再一次性消费所有pending reads。

GeometryRegistry将直接NodeId读与ref读分开；`sync_ref_readers` 按提交scope替换ref观察目标，解绑某个ref不会误删同一owner独立的direct read。Canvas drawable与外层geometry更新统一提取两类reader owner。

`flush_geometry_dependencies` 合并Ref binding dirty与几何dirty。贡献reset/prune、最终owner/provider manifest与release scope都同步清理逻辑订阅和native targets；Event phase读取几何不再意外建立Render订阅。对应代码新增的binding转换、旧node detach、贡献retarget、末item prune、provider/reader teardown和rollback单测已静态审阅；本轮亲自动态复验的是上表原五项native探针，不把未重跑的每个单测都算进16项。

R7的三个失败现象均关闭，没有新增确认的调度owner或binding生命周期问题。

## 结论边界

Theme/geometry子域本轮 **通过**。窗口ownership、Table与其他D项由并行审查负责；最终208 native、五张图像、CI及其他release gates由主审汇总。本报告不授权merge、publish或tag。
