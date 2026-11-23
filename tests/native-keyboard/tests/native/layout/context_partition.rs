use crate::support::{Mount, Step, bounds, click_center, last_error, settle};
use gpui::{TestAppContext, px, size};

#[gpui::test]
fn context_partition_resize_keeps_hit_and_native_tile_bounds_together(cx: &mut TestAppContext) {
    let source = r#"
        define_component(#{
            metadata:#{id:"main","export":"Probe",version:"0.1.0",
                runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                state:#{fields:#{selected:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
                slots:#{},events:#{},parts:[]},
            render:Fn("probe")
        });
        fn view(ctx){
            column([render_component("main",#{key:"board"})])
                .with_style(style().width(relative(1.0)).padding(px(12)))
        }
        fn select(name,ctx,event){ctx.set_state("selected",name);handled()}
        fn probe(ctx,p){
            let field=element_ref("field");
            let dims=ctx.element_size(field);
            let w=if dims==(){500.0}else{dims.width};
            let h=if dims==(){240.0}else{dims.height};
            let plan=context_partition([4,0,4],16,w,h);
            let tiles=[];
            for tile in plan.tiles{
                let id=if tile.index==(){""}else{["__free","zero","chat"][tile.index]};
                let name=if id==""{"Unused capacity"}else{id};
                let node=box([]).accessibility_label(name)
                    .with_style(style().absolute().left(relative(tile.left)).top(relative(tile.top))
                        .width(relative(tile.width)).height(relative(tile.height))
                        .background(theme_color(if id==""{"surface_hover"}else{"accent"})));
                if id==""{tiles.push(node.with_key("capacity").accessibility_role("status"));}
                else{
                    tiles.push(node.with_key("block:"+id).accessibility_role("button")
                        .on_click(Fn("select").curry(id)));
                }
            }
            let selected=ctx.get_state("selected");
            column([box(tiles).with_key("field").with_ref(field).accessibility_role("group")
                .accessibility_label("Allocation field")
                .with_style(style().relative().height(px(240)).width(relative(1.0))),
                text(if selected==(){""}else{"Selected "+selected}).accessibility_role("status")])
                .with_style(style().width(relative(1.0)))
        }
    "#;
    let (mut v, view) = Mount::new(source).mount(cx);
    for width in [600., 192., 410.] {
        v.simulate_resize(size(px(width), px(400.)));
        settle(&mut v, Step::Tick, 6);
        assert_eq!(last_error(&mut v, &view), None);
        let field = bounds(&mut v, &view, "group", "Allocation field");
        let free = bounds(&mut v, &view, "status", "Unused capacity");
        let business = bounds(&mut v, &view, "button", "__free");
        let chat = bounds(&mut v, &view, "button", "chat");
        assert!(
            (field.width - (f64::from(width) - 24.0)).abs() < 1.0,
            "{field:?}"
        );
        let nearly = |a: f64, b: f64| assert!((a - b).abs() < 1.0, "{a} vs {b}");
        if width > 300. {
            nearly(free.x, field.x + field.width * 0.5);
            nearly(free.y, field.y);
            nearly(free.height, field.height);
        } else {
            nearly(free.x, field.x);
            nearly(free.y, field.y + field.height * 0.5);
            nearly(free.width, field.width);
        }
        for tile in [business, chat] {
            let normalized = tile.width * tile.height / (field.width * field.height);
            assert!((normalized - 0.25).abs() < 0.007, "{tile:?}");
        }
        click_center(&mut v, business);
        settle(&mut v, Step::Frame, 2);
        bounds(&mut v, &view, "status", "Selected __free");
        click_center(&mut v, chat);
        settle(&mut v, Step::Frame, 2);
        bounds(&mut v, &view, "status", "Selected chat");
        click_center(&mut v, free);
        settle(&mut v, Step::Frame, 2);
        bounds(&mut v, &view, "status", "Selected chat");
    }
}
