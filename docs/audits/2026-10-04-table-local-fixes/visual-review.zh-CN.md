# 43732b89 五态 Table baseline 追加视觉复验

日期：2026-10-04。结论：**G 限定视觉范围 APPROVE，允许替换本轮五张正式 PNG 并同步 capture manifest；无视觉阻断。** 不代表整个 RC 发布通过。

本 reviewer 按 GPUI layout/style skill 的布局与交互分离原则，只读逐张查看归档的五张原始 PNG、`/tmp/gpui-rhai-018-native-capture.WZYwOr/r8-43732b89/candidates/` 五张候选及旧 `736025a8` 五张 baseline。未重新开展组件设计审计，未操作 UI/Rift/进程、修改产品或已有独立代码报告。

## 尺寸、来源与差异

- 精确源码：`43732b89c48ec6add95cc341d9e1090fe862cec4`，与本轮 R8 局部代码复验同源。
- release binary SHA-256：`4281282285f395906ecbb7e099257a66bb5cab58663f14c42ed6243ea3bb4991`；五次 capture log 一致，另对当前 `target/release/examples/data_table` 独立计算相同。
- 独立读取图片 metadata：五张 raw 均 `1960×1504`，五张 candidate 均 `980×752`。已读 [frames.json](evidence/table-native/frames.json)：五态均 attempt 1，AX/CG before/after 均 `980×752 @ (100,100)`，screen scale 2。
- 十张文件 SHA-256 独立计算与 [capture.log](evidence/table-native/capture.log) 完全一致。raw 归档于 [table-native](evidence/table-native/)，未见指针、privacy pill 或外部遮挡。
- 逐张旧/新目视对照：新原生标题栏为 Light，旧为 Dark；dark/Mocha 内容仍是各自脚本主题。标题栏及接缝差异不按组件主题回退处理，也没有要求为截图改变系统外观。
- 已读主实施者的 [像素比较结果](evidence/visual-pixel-comparison.json)：五态变化行均仅 `y=0..33`，因此 `y=34..751` 相同；本 reviewer 未另跑逐像素比较算法，不将该计算冒充自己的独立执行。目视观察与该差异边界一致。
- 已读 [restoration.json](evidence/table-native/restoration.json)：`rules_restored=true`、`other_config_unchanged=true`、原/恢复规则均 11；没有自行重新查询或控制运行进程。

## 逐张结果

| 候选 | 结论与可见依据 |
| --- | --- |
| `default-light.en.ltr.png` | PASS：六列 Name/Email/Score/Joined/Status/Action 均可见，末列及表头/数据对齐；500 items、Page 1/20、page size 25 和分页处于窗口内。底部末行按既有表格 viewport 裁剪，未把分页裁到窗外。 |
| `default-light.en.loading.png` | PASS：Loading 文本、六列表头和完整状态区域壳层可见；分页位置与正常态一致，位于窗口内。 |
| `default-light.en.empty.png` | PASS：No results、0 items、Page 1/1 及禁用的前后页按钮正确显示；状态区域高度和分页未越窗。 |
| `default-dark.en.selected.png` | PASS：显示 2 selected，User 000/001 两行选中；dark 内容、六列末列、边界及分页可辨且无外部遮挡。 |
| `catppuccin-mocha.ar.rtl.png` | PASS：Mocha 内容与逻辑 RTL 镜像一致；Name 在右、Action 在左，两端六列可见，表头/数据对齐；阿拉伯分页、箭头方向和 page-size 壳层完整。fixture 表格标签仍为 English，不宣称完整 Arabic 内容翻译。 |

批准替换的 candidate SHA-256：

```text
default-light.en.ltr.png       21c71cbc645dade639bf2a48e401faf7c599b0a03ac4fbdbc898d24a8a8c9ca2
default-light.en.loading.png   e62353a0518cf228a1435699eca9cb13573f40714a6c7c0b0a13c760d8b9d4d8
default-light.en.empty.png     b756db92e7215e79a23f2718142816d8b7c33ad46c513b6bedd5a22b3f9ed878
default-dark.en.selected.png   3e79d30e604518f706bacb6637465b5daffd4a2fb3f05a09799c2106eca7ea06
catppuccin-mocha.ar.rtl.png    5583359606568f800c7426fc969ff1a040d8909d82f0741d6b1252ba5639e897
```

## 未证边界

这是既定 G 五态静态视觉验收，不证明横滚、方向切换连续性、完整 OS 输入/辅助功能或系统全局 appearance 人工矩阵，也不替代独立代码复验。主实施者的全量矩阵、strict/package/smoke/performance 不列为本 reviewer 本轮独立覆盖。完整 OS 人工矩阵与物理 120 Hz 仍 pending；不得据此 merge、publish、tag 或宣布整个候选 ready。旧报告及旧失败捕获保持不动。
