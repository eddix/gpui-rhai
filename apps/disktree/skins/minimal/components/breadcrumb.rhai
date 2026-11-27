/* gpui-rhai
{
  "id":"components/breadcrumb", "export":"Breadcrumb", "version":"0.4.3",
  "runtime_api":{"min_inclusive":3,"max_exclusive":4},
  "dependencies":["components/menu","components/tooltip"], "capabilities":{},
  "assets":["icons/chevron_left.svg","icons/chevron_right.svg","icons/more.svg","icons/branch.svg","icons/slash.svg"],
  "tokens":["text_muted","text_primary","text.placeholder","focus_ring","surface_hover","surface_raised","typography.breadcrumb","radius.sm","space.spacing_unit","space.unit"],
  "environment":["density"]
}
*/
// A controlled ancestry trail. Width comes from retained presentation, not
// from the window, a breakpoint table, or an estimate based on string length.
// Example: Breadcrumb(#{key:"route",items:[#{value:"home",label:"Home"},#{value:"jobs",label:"Jobs"}],on_navigate:Fn("go_to")})
import "components/menu" as menu;
import "components/tooltip" as tooltip;

define_component(#{
    metadata: #{ id:"components/breadcrumb", "export":"Breadcrumb", version:"0.4.3",
        runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/menu","components/tooltip"], capabilities:#{},
        assets:["icons/chevron_left.svg","icons/chevron_right.svg","icons/more.svg","icons/branch.svg","icons/slash.svg"],
        tokens:["text_muted","text_primary","text.placeholder","focus_ring","surface_hover","surface_raised","typography.breadcrumb","radius.sm","space.spacing_unit","space.unit"],
        environment:["density"] },
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity for the trail and its overflow."},
            label:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:"Breadcrumb"},
                doc:"Accessible label of the navigation region."},
            overflow_label:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:"Earlier pages"},
                doc:"Accessible name of the ellipsis and its menu."},
            max_items:#{schema:#{type:"integer",min:2,max:32},required:false,sensitive:false,"default":#{type:"integer",value:4},
                doc:"Maximum visible crumbs, excluding the ellipsis. A narrow container can fold more."},
            separator_kind:#{schema:#{type:"string",allowed:["chevron","slash"]},required:false,sensitive:false,
                "default":#{type:"string",value:"chevron"},doc:"Built-in decorative separator; chevrons mirror in RTL."},
            separator:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,
                doc:"Decorative node used in place of the built-in separator."},
            items:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
                value:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable ancestor key."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Full page title, including when shortened on screen."},
                icon:#{schema:#{type:"optional",value:#{type:"one_of",variants:[#{type:"asset"},#{type:"handle",kind:"image"}]}},
                    required:false,sensitive:false,doc:"Decorative glyph shown inline and in overflow."},
                disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},
                    doc:"Inert ancestor, skipped by keyboard navigation."},
            }}},required:true,sensitive:false,doc:"Root-to-current ordered trail. The last entry is never an action; empty shows no controls."},
            on_navigate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
                doc:"Receives the same ancestor value for visible and folded crumbs. The caller owns navigation."},
        },
        state:#{fields:#{
            open_items:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}},
            active:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        }},
        events:#{navigate:#{payload:#{type:"string"},doc:"Requests navigation to an enabled ancestor."}},
        effects:["overflow"],
        slots:#{separator:#{required:false,multiple:false,doc:"Optional decorative replacement separator."}},
        parts:["root","trail","item","link","current","label","icon","separator","separator_icon",
                "more","more_icon","popup","popup_item","popup_icon"],
    },
    render:Fn("render_Breadcrumb")
});

fn Breadcrumb(props) { render_component("components/breadcrumb",props) }
fn navigate(ctx,value) { ctx.emit("navigate",value); }
fn set_active(ctx,value) { ctx.set_state("active",value); }
fn set_open(keys,ctx,open) {
    ctx.set_state("open_items",if open {keys}else{[]});
    ctx.set_state("active","");
}
fn reset_overflow(ctx,keys) {
    let open = ctx.get_state("open_items");
    if open.len > 0 && open != keys { set_open(keys,ctx,false); }
}
fn cleanup_overflow(ctx,keys) { () }
fn enabled(props,item) { !("disabled" in item && item.disabled == true) && "on_navigate" in props && props.on_navigate != () }
fn read_width(ctx,key) {
    let size = ctx.element_size(key);
    if size == () { () } else { size.width }
}
fn glyph(ctx, source, part, edge) {
    image_source(source).with_style(ctx.component_style(part,style().width(px(edge)).height(px(edge)).flex_shrink(false)))
        .accessibility_hidden(true)
}
fn separator(ctx,props) {
    let content = if "separator" in props && props.separator != () { props.separator }
        else if props.separator_kind == "slash" { glyph(ctx,asset("app/icons/slash"),"separator_icon",14) }
        else { glyph(ctx,asset(if ctx.text_direction()=="rtl" {"app/icons/chevron_left"}else{"app/icons/chevron_right"}),
            "separator_icon",14) };
    row([content]).with_style(ctx.component_style("separator",
        style().min_width(px(20)).items_center().justify_center().flex_shrink(false)
            .text_color(theme_color("text.placeholder")))).accessibility_hidden(true)
}

// Pure layout copies contain plain text and icon space, never callbacks, a
// focus owner, or a copy of a caller-provided separator node.
fn crumb(ctx,props,item,current,measure,shrink) {
    let children = [];
    if "icon" in item && item.icon != () {
        children.push(if measure {box([]).with_style(ctx.component_style("icon",
            style().width(px(14)).height(px(14)).flex_shrink(false)))}
            else {glyph(ctx,item.icon,"icon",14)});
    }
    let label_style = ctx.component_style("label",style().min_width(px(0)).whitespace_nowrap().text_ellipsis());
    let label = text(item.label).with_key("label")
        .with_style(if !current && !measure {label_style.max_width(px(200))}else{label_style});
    children.push(label.with_ref(element_ref(if measure {`full:${item.value}`}else{`shown:${item.value}`})));
    let interactive = !current && enabled(props,item);
    let base = style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).min_width(px(0)).radius(theme_radius("sm"))
        .flex_shrink(shrink).text_color(theme_color(if current {"text_primary"}else{"text_muted"}));
    if current { base = base.font_weight(500); }
    if interactive { base = base.cursor_pointer().hover(style().text_color(theme_color("text_primary")))
        .focus_ring(px(2),px(2),theme_color("focus_ring")); }
    let parts = if current {["item","current"]}else if interactive {["item","link"]}else{["item"]};
    let node = row(children).with_key(`crumb:${item.value}`).with_style(ctx.component_style(parts,base));
    if measure { return node.with_ref(element_ref(`measure:${item.value}`)); }
    if current {
        let timing = #{duration_ms:260,easing:cubic_bezier(0.22,1.0,0.36,1.0)};
        return node.accessibility_role("text").accessibility_label(item.label).accessibility_current("page")
            .motion(motion_transition("opacity",0.0,1.0,timing))
            .motion(motion_transition("translate_x",if ctx.text_direction()=="rtl" {4.0}else{-4.0},0.0,timing));
    }
    if interactive { return node.accessibility_role("button").accessibility_label(item.label).on_click_value(Fn("navigate"),item.value); }
    node.accessibility_role("text").accessibility_label(item.label)
}
fn more_face(ctx,open,measure) {
    let child = if measure {box([])}else{glyph(ctx,asset("app/icons/more"),"more_icon",16)};
    let hover = style().background(theme_color("surface_hover")).text_color(theme_color("text_primary"));
    let base = style().width(theme_length("space.unit").times(6.5)).height(theme_length("space.unit").times(5.5)).flex_shrink(false).radius(theme_radius("sm"))
        .items_center().justify_center().text_color(theme_color("text_muted"))
        .focus_ring(px(2),px(2),theme_color("focus_ring")).hover(hover);
    row([child]).with_style(ctx.component_style("more",if open {base.merge(hover)}else{base}))
}
fn folded_count(ctx,props,widths,minimum) {
    let n = props.items.len;
    if n <= 2 { return 0; }
    let width = read_width(ctx,"trail-width");
    let sep = read_width(ctx,"separator-width");
    let more = read_width(ctx,"measure-more");
    if width == () || sep == () || more == () { return n - 2; }
    let natural = widths[props.items[0].value];
    if natural == () { return n - 2; }
    for i in (minimum+1)..n {
        let w = widths[props.items[i].value];
        if w == () { return n - 2; }
        natural += sep + w;
    }
    for i in minimum..(n-2) {
        if natural + (if i > 0 {sep + more}else{0.0}) <= width + 0.5 { return i; }
        natural -= sep + widths[props.items[i+1].value];
    }
    n-2
}
fn overflow(ctx,props,keys) {
    let items = [];
    let active = ctx.get_state("active");
    let first = "";
    let selected = "";
    for i in 1..(keys.len+1) {
        let item = props.items[i];
        let disable = !enabled(props,item);
        let icon = if "icon" in item && item.icon != () {item.icon}else{asset("app/icons/branch")};
        items.push(#{kind:"item",value:item.value,label:item.label,disabled:disable,prefix:glyph(ctx,icon,"popup_icon",14)});
        if !disable && first == "" { first = item.value; }
        if !disable && active == item.value { selected = item.value; }
    }
    let open = ctx.get_state("open_items") == keys;
    menu::Menu(#{key:"more",label:props.overflow_label,trigger:more_face(ctx,open,false)
            .accessibility_role("button").accessibility_label(props.overflow_label).accessibility_expanded(open),
        open:open,active_value:if selected=="" {first}else{selected},items:items,
        on_open_change:Fn("set_open").curry(keys),on_active_change:Fn("set_active"),on_action:Fn("navigate"),
        part_styles:#{content:ctx.component_style("popup",style().min_width(px(180)).max_height(px(280)).overflow_y_scroll()),
            item:ctx.component_style("popup_item",style())}})
        .with_style(style().flex_shrink(false))
}
fn separator_at(ctx,props,index,id) {
    let node = separator(ctx,props).with_key(if index == 1 {"sep:anchor"}else{`sep:${id}`});
    if index == 1 { node.with_ref(element_ref("separator-width")) } else { node }
}
fn render_Breadcrumb(ctx,props) {
    let seen = #{};
    for item in props.items {
        if item.value == "" || item.value in seen { throw "Breadcrumb values must be non-empty and unique"; }
        seen[item.value] = true;
    }
    let n = props.items.len;
    if n == 0 { return row([]); }
    let minimum = max(n - props.max_items,0);
    let measures = [more_face(ctx,false,true).with_key("more-width").with_ref(element_ref("measure-more"))];
    let widths = #{};
    let full = #{};
    for i in 0..n {
        if i > 0 && i <= minimum { continue; }
        let item = props.items[i];
        measures.push(crumb(ctx,props,item,i==n-1,true,false));
        let width = read_width(ctx,`measure:${item.value}`);
        let ink = read_width(ctx,`full:${item.value}`);
        full[item.value] = ink;
        widths[item.value] = if width == () || ink == () {()}else{width - ink + min(200.0,ink)};
    }
    let hidden = folded_count(ctx,props,widths,minimum);
    let keys = [];
    for i in 1..(hidden+1) { keys.push(props.items[i].value); }
    effect("overflow",keys,Fn("reset_overflow"),Fn("cleanup_overflow"));
    let items = [];
    for i in 0..n {
        if i == 1 && hidden > 0 {
            items.push(separator_at(ctx,props,1,"overflow"));
            items.push(overflow(ctx,props,keys));
        }
        if i > 0 && i <= hidden { continue; }
        let item = props.items[i];
        if i > 0 { items.push(separator_at(ctx,props,i,item.value)); }
        let shrink = i == n-1 || (i == 0 && hidden == n-2);
        let node = crumb(ctx,props,item,i==n-1,false,shrink);
        let shown = read_width(ctx,`shown:${item.value}`);
        let wanted = full[item.value];
        let truncated = shown != () && wanted != () && wanted > shown+1.0;
        items.push(tooltip::Tooltip(#{key:`hint:${item.value}`,label:item.label,trigger:node,
            content:text(item.label),disabled:!truncated,show_delay_ms:360,hide_delay_ms:80})
            .with_style(style().min_width(px(0)).flex_shrink(shrink)));
    }
    let strip = row(measures).with_key("measure").accessibility_hidden(true)
        .with_style(style().absolute().left(px(0)).top(px(0)).width(px(0)).height(px(0))
            .overflow_hidden().invisible().flex_shrink(false));
    let trail = row(items).with_key("trail").with_ref(element_ref("trail-width"))
        .with_style(ctx.component_style("trail",style().width(relative(1.0)).min_width(px(0)).items_center()));
    box([strip,trail]).with_key(props.key).accessibility_role("navigation").accessibility_label(props.label)
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).min_width(px(0))
            .typography("breadcrumb").text_color(theme_color("text_muted"))))
}
