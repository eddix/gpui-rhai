import "components/selectable_list" as selectable_list;
import "components/button" as button;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"a2"}]}},
    opened:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    playing:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
}}}
fn set_selected(ctx,value){ctx.set_state("selected",value);}
fn set_opened(ctx,value){ctx.set_state("opened",value);}
fn play(key,ctx,payload){ctx.set_state("playing",key);}
fn album_row(playing,ctx,row){
    let album=row.item;
    let cover=box([]).with_style(style().width(px(40)).height(px(40)).flex_shrink(false)
        .radius(theme_radius("md")).background(theme_color(album.tone)));
    let lines=column([
        text(album.title).with_style(style().typography("body").whitespace_nowrap().text_ellipsis()),
        text(album.artist).with_style(style().typography("caption")
            .text_color(if row.selected {theme_color("text_primary")} else {theme_color("text_muted")})
            .whitespace_nowrap().text_ellipsis()),
    ]).with_style(style().flex_col().flex_grow().min_width(px(0)));
    row([cover,lines,button::Button(#{key:`play-${row.key}`,size:"sm",variant:"ghost",
        text:if playing==row.key {"Playing"} else {"Play"},on_click:Fn("play").curry(row.key)})])
        .with_style(style().items_center().gap(theme_spacing("sm")).flex_grow().min_width(px(0)))
}
fn view(ctx){let items=[
    #{key:"a1",label:"Kind of Blue",title:"Kind of Blue",artist:"Miles Davis",tone:"accent"},
    #{key:"a2",label:"A Love Supreme",title:"A Love Supreme",artist:"John Coltrane",tone:"success"},
    #{key:"a3",label:"Mingus Ah Um",title:"Mingus Ah Um",artist:"Charles Mingus",tone:"warning"},
    #{key:"a4",label:"Time Out",title:"Time Out",artist:"Dave Brubeck Quartet",tone:"danger",disabled:true},
];column([text("ROWS THE APPLICATION DRAWS").with_style(style().typography("heading")),
    selectable_list::SelectableList(#{key:"albums",label:"Albums",items:items,selection_mode:"single",
        selected_keys:ctx.get_state("selected"),height:240.0,
        row:ctx.node_factory(Fn("album_row").curry(ctx.get_state("playing"))),
        on_selection_change:Fn("set_selected"),on_row_click:Fn("set_opened")}),
    text(`Opened: ${ctx.get_state("opened")} · Playing: ${ctx.get_state("playing")}`)
]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("sm")).width(px(480)))}
