# R8：五张 Table 基线复核

审查基线为 `736025a8b2b1ac44d41d893bc9a929c990b97532`。本轮读取并逐张查看仓库中的五张 PNG，没有重新截图、启动 GUI、修改系统主题或窗口管理配置。

**结论：五张基线在约定场景内通过，没有发现新的视觉阻断；G 工作包已完成。** 先前报告把它们列为 pending 的状态已被后续交付取代。

## 证据完整性

- 五张图均为 980×752，SHA256 与 [capture.json](../../../tests/visual/macos/data_table/capture.json) 一致；该尺寸包含原生窗口 chrome，内容区域为 980×720。
- 当前本地 release `data_table` binary 的 SHA256 与 capture manifest 一致。
- 拍摄对应的产品源 `53a62e3f` 与当前候选的 core/CLI/registry 产品树一致，详见 [baseline.json](evidence/baseline.json)。
- 本轮的尺寸、文件 hash 和 binary 比对记录见 [visual-integrity.json](evidence/visual-integrity.json)。实际拍摄、密度归一化与临时窗口规则恢复属于先前 capture 材料，不冒称本轮执行。

## 人工查看结果

| 场景 | 本轮观察 |
| --- | --- |
| default-light / en / LTR | 六列可读，表头与正文对齐，分页与外壳没有明显裁切 |
| default-light / loading | 保持约定的固定高度外壳，加载内容位于正文区域 |
| default-light / empty | 空态与 loading/data 使用相同外壳，没有明显高度跳变 |
| default-dark / selected | 前两行选中反馈清晰，文字与选中背景仍可辨认 |
| catppuccin-mocha / ar / RTL | 列与分页按 RTL 呈现，表头/正文对齐；fixture 数据仍为英文属于记录中的约定 |

浅色正文上方保留深色 OS 窗口 chrome，是独立 OS 外观与组件主题的组合，不据此判为主题错误。图中未发现 screen-control indicator。没有新增来自用户网络参考图的仓库资产。

## 结论边界

静态 PNG 不证明滚动可达性、列拖动、语言方向切换、焦点、IME、VoiceOver 或物理 120Hz。滚动行为由本轮 native probes 单独判断；通过视觉验收不覆盖该组发现的方向切换 offset 问题。

当前图像可作为这一精确产品源的证据。若随后修改 Table 的绘制结果，需重新核对受影响基线；纯文字审计记录的新增不要求无意义地重新拍摄同一 binary。
