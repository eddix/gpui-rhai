# H 增量复核：CLI/checklist b1aa8c7d

日期：2026-10-03。精确提交：`b1aa8c7da98572d4b39315c049e07e864c93d23a`。

限定结论：**CLI生成与checklist窄修已确认；H整体仍REQUEST_CHANGES，不能发布。** 本提交只改CLI lib及release checklist，不能关闭另存的 [H-THEME-01](theme-appearance-ingress-p1.zh-CN.md)。

## CLI生成

独立read-only审查diff：`generated_embed_module` 的初始 `let mut manifest` 改为immutable绑定；每个capability采用 `let manifest = manifest.with_capability(...).expect(...)` shadowing。零cap分支没有unused_mut，非空分支仍串接前一个完整manifest，保持所有requirements和返回值。没有warning allowance、API/权限/schema变更或兼容别名。

新增unit同时检查空cap与两个capability，不只检查字符串中没有mut，还检查两个完整Host requirements和shadow数量。

本 reviewer在该精确HEAD独立执行完整CLI lib：**33/33通过**，包括embed生成、owner文件保护、update冲突/merge、static capability边界和Gallery source准备。原始日志：[cli-lib-b1aa8c7d.log](evidence/cli-lib-b1aa8c7d.log)。

另read-only核验了同任务提供的empty/nonempty strict-owned-consumer日志：均完成实际compile并输出`fixture-owner-main`与`embedded-candidate-prepare-ok`。这属于实施者提供的编译/prepare证据，不冒称本reviewer独立运行该consumer，也不当作crates.io无local-patch干净安装。

观察到重建release CLI SHA256为 `bdafdf3fe3b0e98106a81d2b9d693cacbe3a11b493c0823fb6f6e0bb40e30a31`，与旧fixture初始install binary的 `1636c026...` 不同。最终binary fixture/package记录应使用新binary，不能只改日志中的source SHA沿用旧CLI行为。

## Checklist P2关闭

`docs/release-checklist.md` 新增final convergence gate：链接维护的runtime contract index/final ADR，列R5–R7与POLICY/KEY/THEME/ASSET，保留旧probe按断言解释、default stack/safety、ownedCLI consumer、五图/OS/120 pending与最终head CI/Linux规则，并明确green不授权merge/closure/publish/tag。

这完整覆盖先前 `E-DOC-01` 的scope建议，故**该文档P2在b1aa8c7d关闭**。旧初审/行为报告保留其当时缺口，不覆写历史记录。

## 当前验收边界

core/registry相对于478/33未在本提交改变，但工作区正发生appearance ingress的范围内修复。已有144 native等独立结果只保持当时源码/行为覆盖；须在新精确产品SHA验证appearance的idle、token-only、pause/resume、多View和teardown，不以旧green豁免。

五张最终Table视觉基线、OS手工/物理120Hz和最终RC门槛仍分别pending。实际native App-only override不等于OS global/manual验收。未改产品/测试/旧证据，无commit或GitHub操作。
