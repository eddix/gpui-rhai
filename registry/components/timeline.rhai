/* gpui-rhai
{"id":"components/timeline","export":"Timeline","version":"0.2.2","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"tokens":["border","accent","success","warning","danger","text_primary","text_muted","text.accent","selection","surface_hover","focus_ring","metrics.row","metrics.control","metrics.label_column","metrics.inset","spacing.xs","spacing.sm","spacing.lg","typography.body","typography.caption","typography.label"],"environment":["size","density"]}
*/
// A source-owned, controlled timeline. Application work and data loading stay outside it.
define_component(#{metadata:#{id:"components/timeline","export":"Timeline",version:"0.2.2",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},
    tokens:["border","accent","success","warning","danger","text_primary","text_muted","text.accent","selection","surface_hover","focus_ring","metrics.row","metrics.control","metrics.label_column","metrics.inset","spacing.xs","spacing.sm","spacing.lg","typography.body","typography.caption","typography.label"],environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable timeline identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible timeline name."},
        items:#{schema:#{type:"array",max_items:10000,items:#{type:"ui_value"}},required:true,sensitive:false,doc:"Nested keyed records: title, status, start/end epoch milliseconds, current, description, meta and children; keys are globally unique."},
        details:#{schema:#{type:"map",values:#{type:"node"}},required:false,sensitive:false,"default":#{type:"map",value:#{}},doc:"Optional detail nodes keyed by item key, including nested records."},
        order:#{schema:#{type:"string",allowed:["ascending","descending"]},required:false,sensitive:false,"default":#{type:"string",value:"ascending"},doc:"Sibling source order or its reverse, without reordering nested groups."},
        time_display:#{schema:#{type:"string",allowed:["absolute","relative","duration"]},required:false,sensitive:false,"default":#{type:"string",value:"absolute"},doc:"Time column presentation; skewed durations and future relative times fall back to absolute."},
        time_zone:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:"local"},doc:"local, UTC, offset:<minutes>, or IANA timezone."},
        expanded:#{schema:#{type:"array",items:#{type:"string"},max_items:10000},required:false,sensitive:false,"default":#{type:"array",value:[]},doc:"Controlled expanded group keys; completed groups may start folded."},
        selected_keys:#{schema:#{type:"array",items:#{type:"string"},max_items:1},required:false,sensitive:false,"default":#{type:"array",value:[]},doc:"Controlled single selection."},
        selection_mode:#{schema:#{type:"string",allowed:["none","single"]},required:false,sensitive:false,"default":#{type:"string",value:"none"},doc:"Single selection makes the timeline one tab stop; none still permits group folding."},
        day_headers:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Adds localized day headings to time-ordered feeds."},
        visible:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},doc:"Pause relative presentation ticks while a retained timeline is not visible."},
        height:#{schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,"default":#{type:"integer",value:320},doc:"Virtualized viewport height in logical pixels."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["xs","sm","md","lg"]}},required:false,sensitive:false,doc:"Explicit size or inherited size."},
        on_expanded_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with proposed expanded keys on disclosure or Left/Right."},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with proposed selected keys on row activation or navigation."},
        on_item_click:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with the activated item key."},
    },state:#{fields:#{
        cursor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        tick:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        expanded_change:#{payload:#{type:"array",items:#{type:"string"},max_items:10000},doc:"Proposed expansion after group activation."},
        selection_change:#{payload:#{type:"array",items:#{type:"string"},max_items:1},doc:"Proposed single selection."},
        item_click:#{payload:#{type:"string"},doc:"Activated event key."},
    },slots:#{},parts:["root","row","rail","lamp","time","title","description","detail","meta","disclosure","day_header","indicator_bar","cursor"]},render:Fn("render_Timeline")});
fn Timeline(props){render_component("components/timeline",props)}
fn field(item,name,fallback){if name in item&&item[name]!=(){item[name]}else{fallback}}
fn timeline_records(items,expanded,order,depth,parent){
    if depth>16{throw "Timeline nesting exceeds 16 levels";}
    let ordered=items;if order=="descending"{ordered.reverse();}let result=[];
    for item in ordered{
        if type_of(item)!="map"||!("key" in item)||!("title" in item){throw "Timeline items require key and title";}
        let status=field(item,"status","pending");
        if !(status in ["pending","running","success","warning","failed","skipped","canceled"]){throw "Timeline status is invalid";}
        let children=field(item,"children",[]);let record=item;record.remove("children");
        record.depth=depth;record.parent=parent;record.has_children=children.len>0;record.expanded=item.key in expanded;record.status=status;
        result.push(record);
        // Validate keys in closed groups too; flattening is not validation's boundary.
        if children.len>0{let nested=timeline_records(children,expanded,order,depth+1,item.key);
            if record.expanded{result+=nested;}}
    }result
}
fn timeline_keys(items,seen){for item in items{
    if type_of(item)!="map"||!("key" in item)||type_of(item.key)!="string"||item.key==""{throw "Timeline requires nonempty string keys";}
    if item.key in seen{throw `Timeline duplicate key '${item.key}'`; }seen[item.key]=true;
    if seen.len>10000{throw "Timeline supports at most 10000 keyed records";}
    for name in item.keys(){if !(name in ["key","title","status","start","end","current","description","meta","children"]){throw `Timeline unknown item field '${name}'`;}}
    if !("title" in item)||type_of(item.title)!="string"{throw "Timeline title must be text";}
    for name in ["status","description","meta"]{if name in item&&item[name]!=()&&type_of(item[name])!="string"{throw `Timeline ${name} must be text`;}}
    if "current" in item&&type_of(item.current)!="bool"{throw "Timeline current must be bool";}
    if "children" in item&&type_of(item.children)!="array"{throw "Timeline children must be an array";}
    let start=field(item,"start",());let finish=field(item,"end",());
    if start!=()&&type_of(start)!="i64"{throw "Timeline start must be epoch milliseconds";}
    if finish!=()&&type_of(finish)!="i64"{throw "Timeline end must be epoch milliseconds";}
    let children=field(item,"children",[]);if children.len>0{seen=timeline_keys(children,seen);}
}seen}
fn timeline_toggle(expanded,ctx,key){let next=[];for value in expanded{if value!=key{next.push(value);}}if !(key in expanded){next.push(key);}ctx.emit("expanded_change",next);handled()}
fn timeline_activate(mode,ctx,key){ctx.set_state("cursor",key);ctx.emit("item_click",key);if mode=="single"{ctx.emit("selection_change",[key]);}handled()}
fn timeline_move(mode,ctx,key){ctx.set_state("cursor",key);if mode=="single"{ctx.emit("selection_change",[key]);}handled()}
fn timeline_tick(ctx,value){ctx.set_state("tick",value+1);}
fn timeline_keyboard(expanded,mode,ctx,action){
    if action.kind=="toggle"{timeline_toggle(expanded,ctx,action.key);}
    else if action.kind=="move"{timeline_move(mode,ctx,action.key);}handled()
}
fn timeline_color(status){if status=="running"{theme_color("accent")}else if status=="success"{theme_color("success")}
    else if status=="warning"{theme_color("warning")}else if status=="failed"{theme_color("danger")}else{theme_color("text_muted")}}
fn timeline_time(ctx,item,mode,zone,now){
    let start=field(item,"start",());let finish=field(item,"end",());if start==(){return "—";}
    if start>now||(finish!=()&&(finish<start||finish>now)){return ctx.format_time(start,"short",zone);}
    if mode=="relative"&&start<=now{return ctx.format_relative(start);}
    if mode=="duration"{
        if finish==()&&item.status=="running"&&now>=start{return ctx.format_duration(now-start);}
        if finish!=()&&finish>=start{return ctx.format_duration(finish-start);}
        if finish==(){return "—";}
    }ctx.format_time(start,"short",zone)
}
fn timeline_row(expanded,selected,cursor,mode,display,zone,now,details,total,ctx,payload){
    let item=payload.item;let current=field(item,"current",false);let chosen=item.key in selected;
    let content=[];let status=ctx.t(`timeline.${item.status}`);let start=field(item,"start",());
    let absolute=if start==(){"—"}else{ctx.format_time(start,"long",zone)};
    if "day_header" in item{content.push(text(item.day_header).with_style(ctx.component_style("day_header",style()
        .typography("label").text_color(theme_color("text_muted")))));}
    let lamp=row([]).with_key("lamp").with_style(ctx.component_style("lamp",style().width(px(6)).height(px(6))
        .border(px(1)).border_color(timeline_color(item.status)).background(if item.status=="pending"{rgba(0x00000000)}else{timeline_color(item.status)})));
    if item.status=="running"{lamp=lamp.motion(motion_transition("opacity",0.45,1.0,
        #{duration_ms:ctx.motion_duration("ambient"),easing:"ease_in_out",iterations:"infinite",autoreverse:true,intent:"essential"}));}
    // 6px lamp, 1px rail and 2px cursor are structural signatures, not sizing tokens.
    let rail_top=row([]).with_style(style().width(px(1)).height(theme_spacing("sm")).background(theme_color("border")));
    let rail_bottom=row([]).with_style(style().width(px(1)).flex_grow().background(theme_color("border")));
    let rail=column([rail_top,lamp,rail_bottom]).with_style(ctx.component_style("rail",style().width(theme_length("metrics.control"))
        .self_stretch().items_center()));
    let title=text(item.title).with_style(ctx.component_style("title",style().typography("body")
        .text_color(if current{theme_color("text.accent")}else{theme_color("text_primary")})));
    let title_nodes=[title];
    if item.has_children{title_nodes.insert(0,row([text(if item.expanded{"−"}else{"+"})]).with_key("disclosure")
        .with_style(ctx.component_style("disclosure",style().width(theme_length("metrics.control")).min_height(theme_length("metrics.control"))
            .items_center().justify_center().border(px(2)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring")))))
        .accessibility_role("button").accessibility_label(`${ctx.t(if item.expanded{"timeline.collapse"}else{"timeline.expand"})} ${item.title}`)
        .on_click_value(Fn("timeline_toggle").curry(expanded),item.key).tab_stop(mode=="none"));}
    let body=[row(title_nodes).with_style(style().items_center().gap(theme_spacing("xs")))];
    if field(item,"description","")!=""{body.push(text(item.description).with_style(ctx.component_style("description",style()
        .typography("caption").text_color(theme_color("text_muted")))));}
    if item.key in details{body.push(details[item.key].with_style(ctx.component_style("detail",style())));}
    let time=text(timeline_time(ctx,item,display,zone,now)).with_style(ctx.component_style("time",style().typography("body").font_feature("tnum",1)
        .width(theme_length("metrics.label_column")).flex_shrink(false)
        .padding_top(theme_spacing("xs")).text_color(theme_color("text_muted"))));
    let indent=row([]).with_style(style().width(theme_spacing("lg")*item.depth));
    content.push(row([indent,rail,time,column(body).with_style(style().flex_grow().min_width(px(0)).gap(theme_spacing("xs")).padding_y(theme_spacing("xs"))),
        text(field(item,"meta","")).with_style(ctx.component_style("meta",style().typography("body").font_feature("tnum",1).padding_top(theme_spacing("xs")).text_color(theme_color("text_muted"))))])
        .with_style(style().gap(theme_spacing("sm")).min_height(theme_length("metrics.row")).items_start()));
    if chosen||current{content.push(row([]).with_style(ctx.component_style("indicator_bar",style().absolute().inset_start(px(0))
        .top(px(0)).bottom(px(0)).width(px(2)).background(theme_color("accent")))));}
    if item.key==cursor{content.push(row([]).with_style(ctx.component_style("cursor",style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .border(px(2)).border_color(rgba(0x00000000)).group_focus(style().border_color(theme_color("focus_ring"))))));}
    let node=column(content).with_key(payload.key).with_style(ctx.component_style("row",style().relative().min_height(theme_length("metrics.row"))
        .padding_start(theme_length("metrics.inset")).padding_end(theme_length("metrics.inset"))
        .background(if chosen{theme_color("selection")}else{rgba(0x00000000)})
        .hover(style().background(theme_color("surface_hover")))))
        .accessibility_role("listitem").accessibility_label(`${item.title}, ${status}, ${absolute}, ${timeline_time(ctx,item,display,zone,now)}`)
        .accessibility_selected(chosen).accessibility_level(item.depth+1).accessibility_position_in_set(payload.index+1).accessibility_size_of_set(total);
    if current{node=node.accessibility_current("step");}if item.has_children{node=node.accessibility_expanded(item.expanded);}
    node.on_click_value(Fn("timeline_activate").curry(mode),item.key).tab_stop(false)
}
fn render_Timeline(ctx,props){
    timeline_keys(props.items,#{});let records=timeline_records(props.items,props.expanded,props.order,0,());
    let now=ctx.now();time_parts(now,props.time_zone);
    let cursor=ctx.get_state("cursor");let index=0;let first_current=();let previous_day="";let visible_keys=#{};
    for i in 0..records.len{let item=records[i];visible_keys[item.key]=true;if item.key==cursor{index=i;}if field(item,"current",false)&&first_current==(){first_current=item.key;}
        if props.day_headers&&field(item,"start",())!=(){let day=time_parts(item.start,props.time_zone).date;
            if day!=previous_day{let today=time_parts(now,props.time_zone).date;item.day_header=if day==today{ctx.t("time.today")}
                else if day==date_checked_add_days(today,-1){ctx.t("time.yesterday")}else{ctx.format_date(day,"medium")};previous_day=day;records[i]=item;}}
    }
    if !(cursor in visible_keys)&&records.len>0{cursor=if props.selected_keys.len>0&&props.selected_keys[0] in visible_keys{props.selected_keys[0]}else if first_current!=(){first_current}else{records[0].key};
        for i in 0..records.len{if records[i].key==cursor{index=i;}}}
    if records.len==0{cursor="";}
    let tick=ctx.get_state("tick");
    if props.time_display!="absolute"{timeout("relative-labels",60000,!props.visible,Fn("timeline_tick"),tick);}
    let config=#{key:`${props.key}-rows`,label:props.label,data:records,height:props.height,estimated_height:48.0};
    if cursor!=""{config.reveal_key=cursor;}
    let node=virtual_collection(config,Fn("timeline_row").curry(props.expanded,props.selected_keys,cursor,props.selection_mode,
        props.time_display,props.time_zone,now,props.details,records.len)).with_key(props.key)
        .with_style(ctx.component_style("root",style().focus(style()))).tab_stop(props.selection_mode=="single")
        .accessibility_role("list").accessibility_label(props.label);
    if records.len>0{let item=records[index];let left=if item.has_children&&item.expanded{#{kind:"toggle",key:item.key}}
        else if item.parent!=(){#{kind:"move",key:item.parent}}else{#{kind:"none"}};
        let right=if item.has_children&&!item.expanded{#{kind:"toggle",key:item.key}}else if index+1<records.len&&records[index+1].depth>item.depth{
            #{kind:"move",key:records[index+1].key}}else{#{kind:"none"}};
        node=node.on_key_value("up",Fn("timeline_move").curry(props.selection_mode),records[if index>0{index-1}else{0}].key)
            .on_key_value("down",Fn("timeline_move").curry(props.selection_mode),records[if index+1<records.len{index+1}else{index}].key)
            .on_key_value("home",Fn("timeline_move").curry(props.selection_mode),records[0].key)
            .on_key_value("end",Fn("timeline_move").curry(props.selection_mode),records[records.len-1].key)
            .on_key_value("left",Fn("timeline_keyboard").curry(props.expanded,props.selection_mode),left)
            .on_key_value("right",Fn("timeline_keyboard").curry(props.expanded,props.selection_mode),right)
            .on_key_value("enter",Fn("timeline_activate").curry(props.selection_mode),cursor);
    }if "size" in props&&props.size!=(){node.env(#{size:props.size})}else{node}
}
