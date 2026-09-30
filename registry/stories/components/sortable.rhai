import "components/sortable" as sortable;
import "components/scroll_area" as scroll_area;

fn state_schema() { #{ fields: #{
    order: #{ schema:#{type:"array",max_items:64,items:#{type:"string"}},
        "default":#{type:"array",value:[
            #{type:"string",value:"alpha"}, #{type:"string",value:"bravo"},
            #{type:"string",value:"charlie"}, #{type:"string",value:"delta"},
            #{type:"string",value:"echo"}, #{type:"string",value:"foxtrot"},
            #{type:"string",value:"golf"}, #{type:"string",value:"hotel"}
        ]} },
    status: #{ schema:#{type:"string"}, "default":#{type:"string",value:"Ready"} },
} } }

fn index_of(values, key) {
    for index in 0..values.len { if values[index] == key { return index; } }
    -1
}

fn reordered(ctx, value) {
    let order = ctx.get_state("order");
    let source_index = index_of(order, value.source_key);
    if source_index < 0 { return; }
    let moved = order.remove(source_index);
    let anchor_index = index_of(order, value.anchor_key);
    if anchor_index < 0 { return; }
    let insertion = if value.placement == "after" { anchor_index + 1 } else { anchor_index };
    order.insert(insertion, moved);
    ctx.set_state("order", order);
    ctx.set_state("status", `${value.source_key} → ${value.placement} ${value.anchor_key}`);
}

fn item_card(key, index) {
    column([
        text(key.to_upper()).with_style(style().typography("body").font_weight(700)),
        text(`Deployment stage ${index + 1}`).with_style(style().typography("caption")
            .text_color(theme_color("text_muted")))
    ]).with_style(style().padding(theme_spacing("sm")).gap(theme_spacing("xxs")))
}

fn view(ctx) {
    let order = ctx.get_state("order");
    let items = [];
    let virtual_items = [];
    for index in 0..order.len {
        let key = order[index];
        items.push(#{ key:key, label:`${key} deployment`, content:item_card(key,index) });
        virtual_items.push(#{key:key,label:`${key} virtual row`});
    }
    let list = sortable::Sortable(#{ key:"deployment-order", label:"Deployment order",
        items:items, direction:"vertical", on_reorder:Fn("reordered") });
    column([
        text("CONTROLLED SORTABLE").with_style(style().typography("heading")),
        text("Drag a handle, or focus it and use Option + Arrow/Home/End.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        text(ctx.get_state("status")).accessibility_role("status"),
        scroll_area::ScrollArea(#{ key:"sortable-scroll", label:"Sortable deployment stages",
            height:px(220), axis:"vertical", content:list }),
        sortable::Sortable(#{key:"virtual-order",label:"Virtual deployment order",
            virtual_data:virtual_items,direction:"vertical",height:180.0,on_reorder:Fn("reordered")})
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("sm")))
}
