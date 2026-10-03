# G 五张原生Table候选：独立视觉复验

日期：2026-10-03。审查HEAD `dface4b456eb1867e7273d75cd4bd2425d315d2a`；产品源码53冻结。依批准计划G/H与GPUI layout/context技能，只读逐张查看五份native raw PNG及五份归一候选，核对frame、hash与恢复材料。

**结论：PASS / APPROVE（G有限视觉范围）。五张均无视觉阻断，可以替换对应正式五PNG并完成baseline文件审计。** 这不是横滚功能、OS手工/120Hz或整个候选发布批准。没有重新跑产品套件、操作UI/Rift/进程或修改产品/截图/旧失败诊断。

## 捕获与几何证据

- 来源为最终有效的native window capture，不是先前失败的CUA JPEG或manage:true尝试。临时规则仅五个精确test bundle ID、manage:false；native AX负责位置/尺寸。
- 五张均attempt1；capture before/after的AX size与CG frame一致：`(100,100,980,752)`，DPI2；后续inspect记录同样稳定。content viewport980×720，window chrome32logical。
- 实际逐文件读取raw均 **1960×1504**，candidate均 **980×752**。capture脚本先强制检查raw尺寸，再以sips作精确1/2密度归一；没有981→980压缩、裁补、retouch或拼接。
- 使用`/usr/sbin/screencapture -x -o -l <verified-owned-window-id>`原生PNG；十张图均无CUA privacy-pill或指针光晕覆盖。包含真实系统标题栏，当前系统chrome为Dark，与Light内容主题的差异已明确保留，不是Table绘制错误。
- 同一release data_table binary SHA256：`5ca976270f2387e7c6f4b2e336a000821d1973a7d8c7621f6839bc4bb82b95c1`；五个bundle、capture.log和当前release artifact一致。
- archive：[raw/capture/helper/frames](evidence/table-native/)，公共manifest：[capture.json](../../../../tests/visual/macos/data_table/capture.json)。candidate审查来源为 `/tmp/gpui-rhai-018-native-capture.WZYwOr/candidates/`，raw为其上层对应文件。

## 逐张视觉结果

| 候选 | 独立视觉确认 |
| --- | --- |
| default-light.en.ltr.png | 六列Name/Email/Score/Joined/Status/Action齐全，header/body边界对齐，末列内容可见；条纹行、底部partial row在固定body viewport内裁剪；500items/Page1/20、分页与page-size完整，无越界/遮挡 |
| default-light.en.loading.png | Loading文字位于一致的bordered壳层；六列header及末列保留，固定空白body和pagination完整；没有此前所疑footer出窗，外框总高度与data一致 |
| default-light.en.empty.png | No results与相同header/空白壳层保持；0items/Page1/1、前后页disabled和page-size显示正确，无塌陷或footer跳出 |
| default-dark.en.selected.png | 顶部明确2selected；恰好User000/001两行高亮，其他行保留Dark条纹与清晰边界；六列/末列/分页完整，文本与控件可辨，无选中态遮挡 |
| catppuccin-mocha.ar.rtl.png | Mocha主题一致；逻辑列序在屏幕从右Name/Email/Score/Joined/Status到左Action，header/body对齐、末列可见；控件和Arabic分页方向相应反转，页码1选中，当前示例English数据label按manifest边界保留 |

三种Light态的table top/header/body/bottom及footer落点一致，符合470body+30header+2border的502logical总高度；空白态没有text-only布局塌陷。原图与候选相比，只有密度改变，没有内容消失/压宽或增加图像元素。

## 精确图像SHA256

| case | raw PNG | 980×752 candidate |
| --- | --- | --- |
| default-light.en.ltr | `6fe7199fab103de8b2f350b85a46b8d33563db7c661814f482b211897bece345` | `b1fa18d0d3fd2fb91d16236c5e60e50f378471d5ab0584a5064c2cc5c6891708` |
| default-light.en.loading | `a09106de312c9e86281a8e0778b06726e568bc75e0c0d4c0c65d3609048524f7` | `a09a55d80cc6567f1b46f827b6ba861c9e67e8b5cd1b1e88bf7bfe09bc6f2f05` |
| default-light.en.empty | `30de539f74574bb28db67e46b163e5e49f4651e586e554689fc7c8cb6caac83d` | `08b3103d7a351db3dabef97cb5b15c7c54db317fd071c4bfb300ec13f3956e12` |
| default-dark.en.selected | `4f9fd1718313ec24c0dc0ff2bdde7bba72d03446054a5d78646a1993dbc64a30` | `681a9d9e31bd6750bc91ae5528f127775e8227247f0fc5359af46be14f5dc52d` |
| catppuccin-mocha.ar.rtl | `18ba31adc0fa153584e291b825a13c1b70075c448406f83e8b039c3dbc9bd51b` | `1e71c0243a47a9dfeef420bdc44067101eab935c9682cea236cefbe539159d13` |

以上与capture.log、manifest及归档raw的实际shasum一致。候选尚由主实施者按该hash替换正式文件，本reviewer没有写入正式PNG。

## 恢复与未证边界

独立读取original/restored runtime config并作结构比较：app_rules完全相等、其余配置完全相等，前后均11规则；与 [restoration.json](evidence/table-native/restoration.json) 一致。捕获脚本只按精确canonical bundle executable/PID清理；manifest记录本轮五进程均退出、旧90509保留。没有持久Rift配置修改或第三方窗口控制。

本报告证明这五个既定场景的实际像素、壳层/列/选中态/RTL分页与稳定捕获合同。**不能从zero-offset静态图证明横滚、隐藏列hit/resize、offset保持、键盘focus或IME**；这些仍由维护的Table extent/boundary/frame/native行为套件证明。截图也不替代OS手工、VoiceOver、physical120Hz或merge/publish/tag授权。

旧失败CUA捕获/1962尺寸与只读诊断留档，不将其混入这批有效native PNG判断。G视觉候选通过后，整个RC的状态仍由其余批准计划门槛与最终归档head验证决定。
