//! Tabs allocate bounded panels on both axes without overriding standalone alignment.
use crate::support::{Mount, Step, bounds, by_role, dispatch, last_error, settle};
use gpui::{TestAppContext, px, size};

#[gpui::test]
fn standalone_tab_lists_follow_parent_cross_axis_alignment(cx: &mut TestAppContext) {
    for orientation in ["horizontal", "vertical"] {
        let source = format!(
            r#"
            import "components/tabs" as tabs;
            fn view(ctx) {{ row([text("Title"), tabs::Tabs(#{{label:"Mode",value:"a",panel:false,
                orientation:"{orientation}",tabs:[#{{value:"a",label:"A"}},#{{value:"b",label:"B"}}]}})])
                .accessibility_role("group").accessibility_label("Frame")
                .with_style(style().width(px(720)).height(px(120)).items_center()) }}
        "#
        );
        let (mut visual, view) = Mount::components(source).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let frame = bounds(&mut visual, &view, "group", "Frame");
        let tabs = bounds(&mut visual, &view, "tablist", "Mode");
        assert!(((frame.y + frame.height / 2.0) - (tabs.y + tabs.height / 2.0)).abs() < 1.0);
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn bounded_retained_panels_fill_the_remainder_and_nested_splits_stay_stable(
    cx: &mut TestAppContext,
) {
    for orientation in ["horizontal", "vertical"] {
        let source = format!(
            r#"
            import "components/tabs" as tabs;
            import "components/split_pane" as split;
            fn state_schema(){{#{{fields:#{{page:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"a"}}}},
                ratio:#{{schema:#{{type:"number"}},"default":#{{type:"float",value:0.38}}}}}}}}}}
            fn changed(ctx,value){{ctx.set_state("page",value);}}
            fn resized(ctx,value){{ctx.set_state("ratio",value);}}
            fn view(ctx){{tabs::Tabs(#{{key:"pages",label:"Pages",orientation:"{orientation}",
                value:ctx.get_state("page"),keep_mounted:true,on_change:Fn("changed"),tabs:[
                    #{{value:"a",label:"A",content:split::SplitPane(#{{key:"nested",label:"Resize",
                        size:ctx.get_state("ratio"),min_start:224,min_end:160,start:text("Start"),end:text("End"),on_resize:Fn("resized")}})}},
                    #{{value:"b",label:"B",content:text("Other")}}]}})
                .with_style(style().width(relative(1.0)).height(relative(1.0)))}}
        "#
        );
        let (mut visual, view) = Mount::components(source).mount(cx);
        for width in [720.0, 1000.0, 600.0, 720.0] {
            visual.simulate_resize(size(px(width), px(360.0)));
            settle(&mut visual, Step::Frame, 4);
            let panel = bounds(&mut visual, &view, "tabpanel", "A");
            let divider = bounds(&mut visual, &view, "separator", "Resize");
            assert!(
                panel.width > 400.0 && panel.height > 250.0,
                "{orientation}: {panel:?}"
            );
            assert!(
                (divider.height - panel.height).abs() < 1.0,
                "{orientation}: {divider:?} vs {panel:?}"
            );
            for _ in 0..4 {
                settle(&mut visual, Step::Frame, 1);
                assert_eq!(bounds(&mut visual, &view, "separator", "Resize"), divider);
            }
            dispatch(&mut visual, &view, by_role("tab", "B"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            let other = bounds(&mut visual, &view, "tabpanel", "B");
            assert!(
                (other.width - panel.width).abs() < 1.0
                    && (other.height - panel.height).abs() < 1.0
            );
            dispatch(&mut visual, &view, by_role("tab", "A"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            assert_eq!(bounds(&mut visual, &view, "separator", "Resize"), divider);
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}
