# G 本轮 Loading 截图只读诊断

日期：2026-10-03。工作区HEAD `dface4b456eb1867e7273d75cd4bd2425d315d2a`；产品源保持53冻结，main正在经用户许可的测试bundle临时floating捕获。此reviewer没有操作UI/Rift/进程、改变capture或修改产品。

读取目标：`/tmp/gpui-rhai-018-table-capture.8muIUl/default-light.en.loading.raw.jpg` 和 `default-light.en.ltr.raw.jpg`。按GPUI layout/context skill直接查看原始像素与源链路，不由缩略图或一张图宣布P1。

本次原图SHA256：Loading `0a312bba1965951161d477d834266023e19fda89296dfa25db9ea43ba2ceb2b5`；Data `e802d186b9e0e11dc9443f36cfe03374d3b7597f8e12d1543ed184205f0a683e`。后续重捕若替换路径，应以这些hash区分本诊断时点。

## 当前观察

- 本次实际查看的Loading图中，footer全部位于capture内，table下缘与pagination之间留有正常gap；没有观察到footer被窗口底边裁切。
- Data/Loading的table外框高度目测均约1004physical，即约502logical（DPI2）。footer约在1220–1296physical区域而capture高1504；这只是像素目测范围，不是runtime geometry测量。
- `sips`明确读取：Data原图 **1960×1504**；Loading原图 **1962×1504**。Loading当前不符合980×752的纯2×归一契约，必须拒绝/重捕，不能把981logical宽压成980或无证据裁边补图。
- 两图整组table/footer有约28physical水平、12physical垂直偏移，Data左边/顶部出现外部白margin。列宽/表格外框彼此一致；整体偏移更支持capture/window定位因素，但最终须与main实际native bounds/viewport/当前capture metadata核对。
- 原图顶部screen-control privacy pill与指针光晕不是Table源码组件；它们属于外部chrome/capture环境，应在G差异记录中区分，不能归因于Table布局。

## 两条消费者的源码路径

`examples/data_table.rs` 固定Table body height=470，footer是Table后面的Pagination sibling；root width920、theme-lg padding、theme-sm gap。standalone content size980×720，加外部window chrome后的capture合同为980×752。

- **Data**：Table把 `props.height` 传给virtual_collection；native virtual body在470px viewport内裁剪/实现有界行。
- **Loading/Empty**：`table_state_body` 把height470应用在content UiNode的style，再加theme padding/typography。默认content是普通Text，但renderer先把height样式应用到外层Div，再 `render_text_node` 添加text child；不是丢弃height或把它只传给一个没有布局能力的字串。
- 两条路径共享 `table_shell` 的30px header与1px四边border。预期固定总高度 **470+30+2=502px**。
- native Table layout只统一viewport-based列宽、direct horizontal extent/offset；`resolve_table_layout`对header/body合入width/flex-shrink，未改height。Resolved native track采用flex-col；只有fill_height分支才flex_1/min_h0，而此例不是fill_height。
- GPUI0.3.7把resolved size/min/max/padding/border映射到Taffy。当前两图的高度与这条链路一致，未找到loading/plain-text特有的越界高度推导。

正式native四态测试的原height180（总212）/fill等控制已通过，支持共享模型；但不把这些测试或当前一对图视为所有capture环境都通过。main应先核同instance的actual viewport、window frame/capture box，推进其已请求的正常帧并重捕合规原图。

**诊断结论：当前没有确认新的Table产品缺陷；Loading原图尺寸不合规，capture定位/外部环境仍需核实。** 这不是G五图批准，也不修改既有H审核结论或历史audit。
