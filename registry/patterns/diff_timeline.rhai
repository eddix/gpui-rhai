/* gpui-rhai
{"id":"patterns/diff_timeline","export":"DiffTimeline","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["accent","border","diff_timeline.add","diff_timeline.add_ink","diff_timeline.binary","diff_timeline.card","diff_timeline.code","diff_timeline.del","diff_timeline.del_ink","diff_timeline.faint","diff_timeline.muted","diff_timeline.grid","diff_timeline.image","diff_timeline.net","diff_timeline.sheet","diff_timeline.tray","diff_timeline.wash","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.code","typography.metadata"],"environment":[]}
*/
// Plot cumulative additions, deletions and signed net changes from a Host ledger.
// Example: DiffTimeline(#{key:"patch",run:"patch-1",agent:#{id:"ada",name:"Ada"},edits:[#{id:"import",at:3,file:"src/lib.rs",added:8,removed:1}]})
define_component(#{
    metadata:#{id:"patterns/diff_timeline","export":"DiffTimeline",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],
        tokens:["accent","border","diff_timeline.add","diff_timeline.add_ink","diff_timeline.binary","diff_timeline.card","diff_timeline.code","diff_timeline.del","diff_timeline.del_ink","diff_timeline.faint","diff_timeline.muted","diff_timeline.grid","diff_timeline.image","diff_timeline.net","diff_timeline.sheet","diff_timeline.tray","diff_timeline.wash","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.code","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card instance."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host run identity. Changing it clears the inspected edit."},
        agent:#{schema:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the agent's mirrored dot mark."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent display name."}
        }},required:true,sensitive:false,doc:"The agent whose changes are shown."},
        context:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional context, such as a review reference."},
        edits:#{schema:#{type:"array",max_items:192,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable edit ID; timestamps can coincide."},
            at:#{schema:#{type:"number",min:0.0,max:604800.0},required:true,sensitive:false,doc:"Elapsed seconds at this edit."},
            file:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display path, not a file-system lookup."},
            added:#{schema:#{type:"integer",min:0,max:10000000},required:true,sensitive:false,doc:"Lines added by this edit."},
            removed:#{schema:#{type:"integer",min:0,max:10000000},required:true,sensitive:false,doc:"Lines removed by this edit. Zero and zero denote a binary edit."},
            diff:#{schema:#{type:"optional",value:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                kind:#{schema:#{type:"string",allowed:["add","del","ctx"]},required:true,sensitive:false,doc:"Change kind."},
                text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Literal preview line. The inspector shows at most four."}
            }}}},required:false,sensitive:true,doc:"Optional caller-supplied diff excerpt."}
        }}},required:true,sensitive:true,doc:"Ordered ledger; ties retain the input order. Future edits set the scale, but are not exposed by inspection."},
        elapsed:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:604800.0}},required:false,sensitive:false,doc:"The Host playback cursor. Omit it to show the completed run."},
        duration:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:604800.0}},required:false,sensitive:false,doc:"Optional axis length in seconds, never shorter than the last edit."},
        live:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Show the currently edited file, live marker and auto-follow."},
        max_rows:#{schema:#{type:"integer",min:1,max:24},required:false,sensitive:false,"default":#{type:"integer",value:7},doc:"Maximum number of file rows before scrolling."}
    },state:#{fields:#{
        hover:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        keyboard:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        file_hover:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}
    }},slots:#{},events:#{},parts:["root","panel","header","mark","dots","title","context","status","totals","add","del","across","body",
        "axis","axis_text","plot","time_axis","area_add","area_del","tooltip","tip_header","tip_path","tip_time","tip_rows","tip_line",
        "tip_code_panel","tip_code","tip_code_sign","tip_code_add","tip_code_del","file_panel",
        "files","file_row","file_name","file_dir","file_icon","file_track","file_counts","file_count","count_measure",
        "file_bar","file_edit","file_piece","file_binary","wash","empty","footer","legend","net","net_value"],
    effects:["follow_live_file"]},render:Fn("render_timeline")
});

fn DiffTimeline(p){render_component("patterns/diff_timeline",p)}
fn get(p,k,d){if p!=()&&k in p&&p[k]!=(){p[k]}else{d}}
fn sp(x){theme_length("space.spacing_unit").times(x)}
fn u(x){theme_length("space.unit").times(x)}
fn tx(ctx,k,t){ctx.t_or("diff_timeline."+k,t)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn line(){style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis().text_color(theme_color("text_primary"))}
fn quiet(){line().typography("caption").text_color(theme_color("diff_timeline.muted"))}
fn tick_ink(){quiet().text_color(theme_color("diff_timeline.faint")).font_feature("tnum",1)}
fn label(ctx,part,s,base){text(s).with_style(ctx.component_style(part,base))}
fn color_for(kind){
    switch kind{
        "add"=>theme_color("diff_timeline.add"),"del"=>theme_color("diff_timeline.del"),
        "bin"=>theme_color("diff_timeline.binary"),_=>theme_color("diff_timeline.net")
    }
}
fn metric_ink(kind){
    switch kind{
        "add"=>theme_color("diff_timeline.add_ink"),"del"=>theme_color("diff_timeline.del_ink"),_=>theme_color("text_muted")
    }
}
fn clock(seconds){
    let n=seconds.to_float().round().to_int();
    let s=(n%60).to_string();
    (n/60).to_string()+":"+(if s.len==1{"0"}else{""})+s
}
fn signed(ctx,value,sign){sign+ctx.format_number(value)}
fn brief(ctx,value,sign){
    if value<1000{signed(ctx,value,sign)}else{sign+ctx.format_number(value.to_float()/1000.0,#{max_fraction_digits:if value<10000{1}else{0}})+"k"}
}
fn axis_label(ctx,value){if abs(value)<10000{ctx.format_number(value.round().to_int())}
    else{ctx.format_number(value/1000.0,#{max_fraction_digits:1})+"k"}}
fn basename(path){
    let parts=path.split("/");
    let name=parts[parts.len-1];let dir="";
    for i in 0..parts.len-1{dir+=parts[i]+"/";}
    #{dir:dir,name:name}
}
fn validate(p){
    if p.run==""||utf8_byte_count(p.run)>96||p.agent.id==""||utf8_byte_count(p.agent.id)>96||p.agent.name==""||
        utf8_byte_count(p.agent.name)>512{throw "DiffTimeline requires bounded run and agent identities";}
    let ids=#{};
    for e in p.edits{
        if e.id==""||utf8_byte_count(e.id)>96||e.id in ids{throw "DiffTimeline edit IDs must be unique and bounded";}
        ids[e.id]=true;
        for line in get(e,"diff",[]){if utf8_byte_count(line.text)>4096{throw "DiffTimeline diff lines must fit 4096 bytes";}}
    }
}
fn stamp(ctx,field,r){
    let value=ctx.get_state(field);
    if value!=()&&value.run==r{value.id}else{""}
}
fn set_target(run,field,id,ctx,inside){
    let current=#{run:run,id:id};
    let before=ctx.get_state(field);
    if inside{if before!=current{ctx.set_state(field,current);}}
    else if before==current{ctx.set_state(field,());}
}
fn pick(run,ids,operation,ctx,event){
    let next="";
    if operation!="close"&&ids.len>0{
        let highlighted=stamp(ctx,"hover",run);
        let selected=if highlighted!=""{highlighted}else{stamp(ctx,"keyboard",run)};
        let i=ids.index_of(selected);
        next=switch operation{
            "first"=>ids[0], "last"=>ids[ids.len-1], "next"=>ids[if i<0{ids.len-1}else{min(ids.len-1,i+1)}],
            _=>ids[if i<0{ids.len-1}else{max(0,i-1)}]
        };
    }
    ctx.set_state("hover",());ctx.set_state("keyboard",if next==""{()}else{#{run:run,id:next}});handled()
}
fn focus_plot(run,ids,ctx,inside){
    if inside{if stamp(ctx,"keyboard",run)==""&&ids.len>0{ctx.set_state("keyboard",#{run:run,id:ids[ids.len-1]});}}
    else{ctx.set_state("keyboard",());}
}
fn active_index(ctx,p,m){
    for name in ["hover","keyboard"]{
        let id=stamp(ctx,name,p.run);
        if id!=""{
            for line in m.shown{if p.edits[line.index].id==id{return line.index;}}
        }
    }
    -1
}
fn agent_mark(ctx,p){
    let seed=0;
    for c in p.agent.id.chars(){seed=(seed*31+c.to_int())%2147483647;}
    let rows=[];let count=0;
    for i in 0..3{
        let row=[];
        for j in 0..3{
            let bit=(i*2+if j==1{1}else{0});
            let lit=((seed >> bit)&1)==1||i==1||i==0&&j!=1;
            let px=box([]).with_key("dot:"+j.to_string()).with_style(ctx.component_style("dots",style().width(u(0.9))
                .height(u(0.9)).radius(px(1)).background(alpha(theme_color("accent"),if lit{1.0}else{0.16}))));
            if p.live&&lit{
                px=px.motion(motion_transition("opacity",0.44,1.0,#{duration_ms:520,delay_ms:count*45,
                    iterations:"infinite",autoreverse:true,intent:"decorative"}));
            }
            row.push(px);count+=1;
        }
        rows.push(row(row).with_key("dots:"+i.to_string()).with_style(style().gap(u(0.35))));
    }
    column(rows).with_key("agent-mark").accessibility_hidden(true)
        .with_style(ctx.component_style("mark",style().flex_shrink(false).width(u(8)).height(u(8)).radius(theme_radius("sm"))
            .background(theme_color("surface_hover")).items_center().justify_center().gap(u(0.35))))
}
fn total_chip(ctx,part,value,kind){
    row([text(value).with_style(style().whitespace_nowrap().font_feature("tnum",1))])
        .with_key("total:"+part).with_style(ctx.component_style(part,style().items_center().typography("metadata")
            .padding_x(sp(2)).height(u(5.5)).radius(theme_radius("sm")).text_color(metric_ink(kind))
            .background(alpha(color_for(kind),if kind=="add"{0.14}else{0.1}))))
}
fn path_line(ctx,parts,dir,name){
    let children=[];
    if dir!=""{
        children.push(label(ctx,parts[0],dir,quiet().flex_shrink(true)
            .text_color(theme_color("diff_timeline.faint"))).with_key("directory"));
    }
    children.push(label(ctx,parts[1],name,line().flex_shrink(false).max_width(relative(1.0)))
        .with_key("basename"));
    children
}
fn head(ctx,p,m,narrow){
    let context=get(p,"context","");
    let title=[label(ctx,"title",p.agent.name,line().font_weight(600).flex_shrink(false).max_width(relative(1.0)))];
    if context!=""{title.push(label(ctx,"context"," · "+context,quiet()));}
    let latest=if m.shown.len==0{()}else{p.edits[m.shown[m.shown.len-1].index]};
    let s=if m.shown.len==0{
        if p.live{tx(ctx,"waiting","Waiting for the first edit…")}else{tx(ctx,"no_edits","No edits")}
    }else if p.live{tx(ctx,"editing","Editing")+" "+latest.file}else{
        ctx.t_or("diff_timeline.complete","Done · {count} edits in {time}",#{count:ctx.format_number(m.shown.len),time:clock(m.head)})
    };
    let status=if p.live && latest!=(){
        let name=basename(latest.file);
        let editing=tx(ctx,"editing","Editing");
        let children=[label(ctx,"status",editing,quiet().flex_shrink(false).margin_end(u(1))).with_key("editing")];
        for node in path_line(ctx,["status","status"],name.dir,name.name){children.push(node);}
        row(children).with_style(style().min_width(px(0)).overflow_hidden())
    }else{label(ctx,"status",s,quiet())};
    let words=column([
        row(title).accessibility_role("heading").accessibility_level(3)
            .accessibility_label(p.agent.name+(if context!=""{" · "+context}else{""}))
            .with_style(style().min_width(px(0)).overflow_hidden().items_baseline()),
        status.accessibility_role("status").accessibility_label(s)
    ]).with_style(style().min_width(px(0)).flex_grow());
    let count=0;for f in m.files{if f.edits.len>0{count+=1;}}
    let a=signed(ctx,m.add,"+");let d=signed(ctx,m.del,"−");
    let across=ctx.t_or("diff_timeline.files_count","across {count} files",#{count:ctx.format_number(count)});
    let stats=[total_chip(ctx,"add",a,"add"),total_chip(ctx,"del",d,"del")];
    if !narrow{stats.push(label(ctx,"across",across,quiet().typography("metadata").margin_start(sp(0.5))));}
    let totals=row(stats).with_key("totals").accessibility_role("group")
        .accessibility_label(tx(ctx,"totals","Change totals")).accessibility_description(a+" "+d+" "+across)
        .with_style(ctx.component_style("totals",style().min_width(px(0)).flex_shrink(false).gap(sp(1.5)).items_center()));
    let layout=if narrow{
        column([row([agent_mark(ctx,p),words]).with_style(style().width(relative(1.0)).gap(sp(2.5)).items_center().min_width(px(0))),
            totals.with_style(style().width(relative(1.0)).padding_start(sp(6.5)))]).with_style(style().width(relative(1.0)))
    }else{row([agent_mark(ctx,p),words,totals])};
    layout.with_key("head").with_style(ctx.component_style("header",style().min_width(px(0)).gap(sp(2.5))
        .items_center())).with_style(if narrow{style().items_start()}else{style()})
}
fn y(value,m,h){(m.ceiling-value)*h/(m.ceiling-m.floor)}
fn point(s,m,dims){
    #{x:s.t*dims.width/m.end,a:y(s.a,m,dims.height),d:y(s.d,m,dims.height),net:y(s.a-s.d,m,dims.height)}
}
fn dashed(points){
    let out=[];let on=true;let remain=5.0;let continuing=false;
    for i in 1..points.len{
        let a=points[i-1];let b=points[i];let dx=b.x-a.x;let dy=b.net-a.net;
        let length=(dx*dx+dy*dy).sqrt();
        let steps=0.0;
        while steps<length-0.00001{
            let take=min(remain,length-steps);
            if on{
                let begin=steps/length;let end=(steps+take)/length;
                if !continuing{out.push(path_move(a.x+dx*begin,a.net+dy*begin));}
                out.push(path_line(a.x+dx*end,a.net+dy*end));continuing=true;
            }
            steps+=take;remain-=take;
            if remain<=0.00001{on=!on;continuing=false;remain=if on{5.0}else{4.0};}
        }
    }
    out
}
fn canvas_commands(ctx,m,dims,guide,live){
    let marks=[];
    let numbers=m.floor;
    while numbers<=m.ceiling+0.01{
        let g=max(0.5,min(dims.height-0.5,y(numbers,m,dims.height)));
        marks.push(canvas_line("grid:"+numbers.to_string(),0,g,dims.width,g,1,
            theme_color(if numbers==0.0{"border"}else{"diff_timeline.grid"})));
        numbers+=m.step;
    }
    if m.samples.len>1&&m.shown.len>0{
        let a=[];let b=[];let pts=[];
        for s in m.samples{
            let v=point(s,m,dims);pts.push(v);
            a.push(if a.len==0{path_move(v.x,v.a)}else{path_line(v.x,v.a)});
            b.push(if b.len==0{path_move(v.x,v.d)}else{path_line(v.x,v.d)});
        }
        let x=m.head/m.end*dims.width;let base=y(0.0,m,dims.height);
        for what in ["add","del"]{
            let s=if what=="add"{a}else{b};
            s.push(path_line(x,base));s.push(path_line(0,base));s.push(path_close());
            let hue=color_for(what);
            let paint=ctx.component_style(if what=="add"{"area_add"}else{"area_del"},
                style().background(alpha(hue,0.16)));
            marks.push(canvas_fill_path("wash:"+what,s,paint));
        }
        let net=dashed(pts);
        if net.len>=2{marks.push(canvas_stroke_path("net",net,1,theme_color("diff_timeline.net")));}
        marks.push(canvas_stroke_path("del",b,1.8,color_for("del")).stroke_cap("round").stroke_join("round"));
        marks.push(canvas_stroke_path("add",a,1.8,color_for("add")).stroke_cap("round").stroke_join("round"));
        if live{marks.push(canvas_line("playhead",x,0,x,dims.height,1,alpha(theme_color("accent"),0.55)));}
    }
    if guide!=(){
        let pt=point(guide,m,dims);
        marks.push(canvas_line("inspection",pt.x,0,pt.x,dims.height,1,theme_color("diff_timeline.net")));
        marks.push(canvas_circle("inspect-add",pt.x,pt.a,3.3,color_for("add")));
        marks.push(canvas_circle("inspect-del",pt.x,pt.d,3.3,color_for("del")));
    }
    marks
}
fn elapsed_step(end){
    for s in [15.0,30.0,60.0,120.0,300.0,600.0,900.0,1800.0,3600.0,7200.0,14400.0,28800.0,43200.0,86400.0,172800.0]{
        if end/s<=6.0{return s;}
    }
    172800.0
}
fn axis(ctx,m,dims){
    let labels=[];let n=m.floor;
    while n<=m.ceiling+0.01{
        let top=max(0.0,min(dims.height-16.0,y(n,m,dims.height)-8.0));
        labels.push(label(ctx,"axis_text",axis_label(ctx,n),tick_ink().absolute().top(px(top))
            .width(relative(1.0)).text_end()));
        n+=m.step;
    }
    box(labels).with_key("axis").accessibility_hidden(true).with_style(ctx.component_style("axis",style().relative().width(u(8))
        .height(px(dims.height)).flex_shrink(false)))
}
fn time_axis(ctx,m,dims){
    let labels=[];let at=0.0;let step=elapsed_step(m.end);
    let width=min(49.0,dims.width);
    while at<=m.end+0.00001{
        let offset=max(0.0,min(dims.width-width,dims.width*at/m.end-width*0.5));
        labels.push(label(ctx,"axis_text",clock(at),tick_ink().absolute().top(px(6)).left(px(offset))
            .width(px(width)).text_center()));
        at+=step;
    }
    box(labels).accessibility_hidden(true).with_key("ticks").with_style(ctx.component_style("time_axis",style().relative().height(u(5)).width(relative(1.0))))
}
fn preview_code(ctx,diff){
    let lines=[];
    for i in 0..min(4,diff.len){
        let d=diff[i];let kind=d.kind;let changed=kind!="ctx";
        let parts=if changed{["tip_code","tip_code_"+kind]}else{["tip_code"]};
        let sign=if kind=="add"{"+"}else if kind=="del"{"−"}else{" "};
        let st=style().typography("code").min_width(px(0)).padding_end(u(2)).clip().items_center()
            .text_color(theme_color(if changed{"text_primary"}else{"text_muted"}));
        if changed{st=st.background(alpha(color_for(kind),if kind=="add"{0.14}else{0.10}));}
        let prefix=label(ctx,"tip_code_sign",sign,style().width(u(4.5)).flex_shrink(false).text_center()
            .text_color(if changed{metric_ink(kind)}else{theme_color("diff_timeline.faint")})).with_key("sign");
        let code=text(d.text).with_key("body").with_style(style().min_width(px(0)).flex_grow().whitespace_nowrap().text_ellipsis());
        lines.push(row([prefix,code]).with_key("diff:"+i.to_string()).with_style(ctx.component_style(parts,st)));
    }
    column(lines).with_key("code-preview").with_style(ctx.component_style("tip_code_panel",style().min_width(px(0)).margin_top(u(2))
        .gap(px(0)).clip().radius(theme_radius("sm")).background(theme_color("surface_hover"))))
}
fn tip(ctx,p,m,index,guide,dims){
    let e=p.edits[index];
    let when=clock(e.at);
    let desc=when+" · "+e.file+" · "+signed(ctx,e.added,"+")+" / "+signed(ctx,e.removed,"−");
    let title=row([label(ctx,"tip_path",e.file,line().font_weight(500).flex_grow()),label(ctx,"tip_time",when,quiet().flex_shrink(false))])
        .with_style(ctx.component_style("tip_header",style().min_width(px(0)).gap(sp(1.5)).justify_between()));
    let out=[title];
    for kind in ["add","del"]{
        let added=kind=="add";
        let words=if added{tx(ctx,"added","Added")}else{tx(ctx,"removed","Removed")};
        let c=if added{guide.a}else{guide.d};
        let delta=if added{signed(ctx,e.added,"+")}else{signed(ctx,e.removed,"−")};
        out.push(row([label(ctx,"tip_line",words,quiet().flex_grow()),label(ctx,"tip_line",ctx.format_number(c.round().to_int()),line().font_weight(500)),
            label(ctx,"tip_line",delta,quiet().text_color(metric_ink(kind)))])
            .with_style(ctx.component_style("tip_rows",style().min_width(px(0)).gap(sp(1.5)).items_center())));
    }
    if e.added+e.removed==0{out.push(label(ctx,"tip_line",tx(ctx,"binary","Binary file changed"),quiet()));}
    let diff=get(e,"diff",[]);
    if diff.len>0{out.push(preview_code(ctx,diff));}
    let width=max(1.0,min(236.0,dims.width-6.0));
    let x=guide.t/m.end*dims.width;
    let left=max(0.0,min(dims.width-width,if x>dims.width*0.5{x-width-10.0}else{x+10.0}));
    column(out).with_key("tip:"+e.id).accessibility_role("status").accessibility_label(tx(ctx,"selected_edit","Selected edit"))
        .accessibility_description(desc).with_style(ctx.component_style("tooltip",style().absolute().left(px(left)).top(px(3))
        .width(px(width)).min_width(px(0)).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))
        .radius(theme_radius("sm")).padding(sp(2)).gap(sp(0.5))))
}
fn plot(ctx,p,m,selected){
    let ref=element_ref("plot");
    let dims=ctx.element_size(ref);
    let dims=if dims==(){#{width:460.0,height:156.0}}else{#{width:max(1.0,min(4096.0,dims.width)),height:max(1.0,dims.height)}};
    let guide=();let ids=[];
    for s in m.shown{ids.push(p.edits[s.index].id);if s.index==selected{guide=s.guide;}}
    let absolute=style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0));
    let out=[canvas(canvas_scene(canvas_commands(ctx,m,dims,guide,p.live)).view_box(0,0,dims.width,dims.height))
        .with_key("chart-canvas").accessibility_hidden(true).with_style(absolute)];
    if p.live&&guide==()&&m.shown.len>0{
        let edge=point(m.samples[m.samples.len-1],m,dims);
        for series in ["add","del"]{
            let y=if series=="add"{edge.a}else{edge.d};
            out.push(box([]).with_key("glow:"+series).accessibility_hidden(true).with_style(
                style().absolute().left(px(max(0.0,edge.x-4.0))).top(px(max(0.0,y-4.0))).width(px(8)).height(px(8))
                .radius(px(4)).background(color_for(series)))
                .motion(motion_transition("opacity",0.35,1.0,#{duration_ms:650,iterations:"infinite",autoreverse:true,intent:"decorative"})));
        }
    }
    if guide!=(){out.push(tip(ctx,p,m,selected,guide,dims));}
    let n=m.shown.len;
    let i=0;
    while i<n{
        let a=p.edits[m.shown[i].index].at;let j=i+1;
        while j<n&&p.edits[m.shown[j].index].at==a{j+=1;}
        let left=if i==0{0.0}else{(a+p.edits[m.shown[i-1].index].at)*0.5/m.end};
        let right=if j==n{1.0}else{(a+p.edits[m.shown[j].index].at)*0.5/m.end};
        let id=ids[i];
        out.push(box([]).with_key("hit:"+id).accessibility_hidden(true)
            .on_hover_change(Fn("set_target").curry(p.run,"hover",id))
            .with_style(style().absolute().left(relative(left)).top(px(0)).width(relative(right-left)).height(relative(1.0))));
        i=j;
    }
    let chart=box(out).with_key("plot").with_ref(ref).accessibility_role("group")
        .accessibility_label(tx(ctx,"plot","Edit timeline")).accessibility_description(tx(ctx,"keys","Use left and right arrows to inspect edits"))
        .with_style(ctx.component_style("plot",style().relative().min_width(px(0)).width(relative(1.0)).height(u(39))
            .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))));
    if n>0{
        chart=chart.tab_stop(true).on_focus_within_change(Fn("focus_plot").curry(p.run,ids))
            .on_key_value("plain+left",Fn("pick").curry(p.run,ids,"prior"),())
            .on_key_value("plain+right",Fn("pick").curry(p.run,ids,"next"),())
            .on_key_value("plain+home",Fn("pick").curry(p.run,ids,"first"),())
            .on_key_value("plain+end",Fn("pick").curry(p.run,ids,"last"),())
            .on_key_value("plain+escape",Fn("pick").curry(p.run,ids,"close"),());
    }
    row([axis(ctx,m,dims),column([chart,time_axis(ctx,m,dims)]).with_style(style().flex_grow().min_width(px(0)))])
        .with_key("body").with_style(ctx.component_style("body",style().min_width(px(0)).gap(sp(2)).padding(sp(2))))
}
fn glyph(ctx,path){
    let name=basename(path).name;let dots=name.split(".");
    let ext=if dots.len>1{dots[dots.len-1].to_lower()}else{""};
    let kind=if ext in ["ts","js","jsx","tsx","rs","py","go","toml","json","css","rhai"]{"code"}
        else if ext in ["csv","xls","xlsx","tsv"]{"sheet"}
        else if ext in ["png","jpg","svg","webp","gif"]{"image"}else if ext==""{"folder"}else{"document"};
    let ink=switch kind{"code"=>theme_color("diff_timeline.code"),"sheet"=>theme_color("diff_timeline.sheet"),
        "image"=>theme_color("diff_timeline.image"),_=>theme_color("text_muted")};
    let rails=if kind=="folder"{
        "<path d='M1.4 3c0-.7.4-1.2 1.1-1.2h2.1l1.4 1.3h3.7c.5 0 .9.4.9.9v1.6H1.4z' fill='currentColor'/><path d='M1.4 4.6h9.2v4.9c0 .6-.5 1-1.1 1h-7c-.6 0-1.1-.4-1.1-1z' fill='currentColor' opacity='.3'/>"
    }else{
        let signature=switch kind{
            "code"=>"m4.9 6.4-1.4 1.2 1.4 1.3m2.2-2.5 1.4 1.2-1.4 1.3",
            "sheet"=>"M4 6.2h4v3H4zm1.9 0v3m-1.9-1.5h4",
            "image"=>"m3.7 9 1.9-2.3 1.3 1.1L8 6.4l.5 2.6z",
            _=>"M4 6.6h4m-4 2h2.6"
        };
        "<path d='M3.3 1h3.9L10 3.8v6.4c0 .7-.6 1.3-1.3 1.3H3.3c-.7 0-1.3-.6-1.3-1.3v-8C2 1.5 2.6 1 3.3 1Z' fill='currentColor' opacity='.25'/><path d='M7.2 1v1.8c0 .55.45 1 1 1H10Z' fill='currentColor'/><path d='"+
            signature+"' fill='none' stroke='currentColor' stroke-width='0.9' stroke-linecap='round' stroke-linejoin='round'/>"
    };
    svg("<svg viewBox='0 0 12 12'>"+rails+"</svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("file_icon",
            style().width(u(3.5)).height(u(3.5)).flex_shrink(false).text_color(ink)))
}
fn bar(ctx,p,m,f,selected){
    let amounts=f.add+f.del;let marks=[];
    let active=selected>=0&&p.edits[selected].file==f.path;
    for i in f.edits{
        let e=p.edits[i];
        let opacity=if active&&selected!=i{0.3}else{1.0};
        if e.added+e.removed==0{
            marks.push(box([]).with_key("binary:"+e.id).with_style(ctx.component_style("file_binary",style()
                .width(px(6)).height(relative(1.0)).flex_shrink(false).radius(px(2))
                .background(theme_color("diff_timeline.faint")).opacity(opacity))));
            continue;
        }
        let pieces=[];
        for kind in ["add","del"]{
            let n=if kind=="add"{e.added}else{e.removed};
            if n==0{continue;}
            pieces.push(box([]).with_key(kind).with_style(ctx.component_style("file_piece",style().flex_grow(n)
                .flex_basis(px(0)).min_width(px(1)).height(relative(1.0)).background(alpha(color_for(kind),0.75)))));
        }
        marks.push(row(pieces).with_key(e.id).with_style(ctx.component_style("file_edit",style()
            .flex_grow(e.added+e.removed).flex_basis(px(0)).min_width(px(3)).height(relative(1.0))
            .overflow_hidden().radius(px(3)).opacity(opacity))));
    }
    let frac=min(1.0,max(1,amounts).to_float()/m.max_file.to_float());
    let draw=row(marks).with_key("pieces").with_style(ctx.component_style("file_bar",style()
        .width(relative(frac)).min_width(u(1.5)).height(u(2)).gap(sp(0.5)).items_center()))
        .motion(motion_transition("width_fraction",0.0,frac,#{duration_ms:340,easing:ease(),intent:"decorative"}));
    row([draw]).with_key("file-track").accessibility_hidden(true)
        .with_style(ctx.component_style("file_track",style().min_width(px(0)).overflow_hidden().items_center()))
}
fn count_text(ctx,value,sign,narrow){
    if narrow{brief(ctx,value,sign)}else{signed(ctx,value,sign)}
}
fn count_column(ctx,files,narrow){
    let units=6;
    for f in files{
        let a=count_text(ctx,f.final_add,"+",narrow);
        let d=count_text(ctx,f.final_del,"−",narrow);
        if f.final_add+f.final_del>0{units=max(units,a.len+d.len+1);}
    }
    let zero=ctx.element_size("file-count-measure");
    if zero==()||zero.width<=0.0{u(units.to_float()*2.0)}else{px(zero.width*units.to_float())}
}
fn file_counts(ctx,f,narrow,width){
    let values=[];
    if f.add+f.del==0{
        values.push(label(ctx,"file_count",tx(ctx,"binary_short","binary"),quiet().typography("metadata")).with_style(style().font_weight(400)));
    }else{
        let faint=theme_color("diff_timeline.faint");
        for kind in ["add","del"]{
            let n=if kind=="add"{f.add}else{f.del};
            let sign=if kind=="add"{"+"}else{"−"};
            values.push(label(ctx,"file_count",count_text(ctx,n,sign,narrow),line().typography("metadata").font_feature("tnum",1))
                .with_key("count:"+kind).with_style(style().flex_shrink(false)
                    .text_color(if n==0{faint}else{metric_ink(kind)})));
        }
    }
    row(values).with_key("counts").with_style(ctx.component_style("file_counts",style()
        .width(width).flex_shrink(false).justify_end().items_baseline().gap(sp(1.5)).overflow_hidden()))
}
fn file_ref(i){"row:"+i.to_string()}
fn follow(ctx,data){
    let row=ctx.element_bounds(data.reference);
    let viewport=ctx.element_bounds("file-viewport");
    if row==()||viewport==(){return;}
    let r=row.layout;let v=viewport.layout;
    if r.y>=v.y-0.5&&r.y+r.height<=v.y+v.height+0.5{return;}
    let position=max(0.0,data.index.to_float()*r.height-(v.height-r.height)*0.5);
    ctx.scroll_to("file-viewport",0.0,position);
}
fn idle(ctx,data){}
fn file_panel(ctx,p,m,selected,narrow){
    let opened=if selected<0{stamp(ctx,"file_hover",p.run)}else{p.edits[selected].file};
    let last=if m.shown.len==0{""}else{p.edits[m.shown[m.shown.len-1].index].file};
    let first=ctx.element_size("row:0");
    let h=if first==(){32.0}else{max(1.0,first.height)};
    let count_width=count_column(ctx,m.files,narrow);
    let rows=[];let wash=-1;
    for i in 0..m.files.len{
        let f=m.files[i];
        let base=style().min_width(px(0)).height(u(8)).flex_shrink(false).padding_x(sp(1.5)).items_center().gap(sp(1.5));
        let row_style=ctx.component_style("file_row",base);
        if narrow{row_style=row_style.padding_x(sp(1.5)).gap(sp(2));}
        let key=file_ref(i);
        if f.edits.len==0{
            rows.push(row([]).with_key("file:"+f.path).with_ref(element_ref(key)).accessibility_hidden(true).with_style(row_style));
            continue;
        }
        let name=basename(f.path);
        let has=f.add+f.del>0;
        let count=if has{(if narrow{brief(ctx,f.add,"+")+" "+brief(ctx,f.del,"−")}else{signed(ctx,f.add,"+")+" "+signed(ctx,f.del,"−")})}
            else{tx(ctx,"binary_short","binary")};
        let words=path_line(ctx,["file_dir","file_name"],if narrow{""}else{name.dir},name.name);
        let path=row(words).with_key("file-path").with_style(style().min_width(px(0)).flex_basis(px(0))
            .flex_grow(if narrow{1.0}else{0.9}).items_baseline().overflow_hidden());
        let track=bar(ctx,p,m,f,selected).with_style(style().flex_basis(px(0)).flex_grow(if narrow{0.7}else{1.0}));
        let icon=row([glyph(ctx,f.path)]).with_style(style().width(u(4)).flex_shrink(false).items_center().justify_center());
        let row_style=if opened!=""&&opened!=f.path{row_style.opacity(0.4).isolated_opacity(true)}else{row_style};
        rows.push(row([icon,path,track,file_counts(ctx,f,narrow,count_width)])
            .with_key("file:"+f.path).with_ref(element_ref(key))
            .accessibility_role("listitem").accessibility_label(f.path).accessibility_description(count)
            .on_hover_change(Fn("set_target").curry(p.run,"file_hover",f.path)).with_style(row_style).paint_transition(150,"ease_out"));
        if p.live&&last==f.path{wash=i;}
    }
    if rows.len==0{rows.push(label(ctx,"empty",tx(ctx,"no_files","No files changed"),quiet().padding(sp(2))));}
    if wash>=0{
        let y=wash.to_float()*h;
        rows.insert(0,box([]).with_key("live-wash").accessibility_hidden(true).with_style(ctx.component_style("wash",style().absolute()
            .top(px(0)).left(px(0)).width(relative(1.0)).height(px(h)).background(theme_color("diff_timeline.wash"))))
            .motion(motion_transition("translate_y",y,y,#{duration_ms:350,easing:ease(),intent:"decorative"})));
        if opened==""&&m.files.len>p.max_rows&&first!=(){
            effect("follow_live_file",#{run:p.run,reference:file_ref(wash),index:wash},Fn("follow"),Fn("idle"));
        }
    }
    let more=m.files.len>p.max_rows;
    let stage=column(rows).with_key("rows").with_style(style().relative().min_width(px(0)).padding_bottom(sp(if more{3}else{0})));
    let viewport=h*min(p.max_rows,max(1,m.files.len)).to_float();
    let scroller=style().min_width(px(0)).overflow_y_scroll().max_height(px(viewport)).radius(theme_radius("lg"));
    if more{scroller=scroller.bottom_fade(max(0.0,1.0-20.0/viewport),1.0);}
    let scroller=column([stage]).with_key("files").with_ref(element_ref("file-viewport"))
        .accessibility_role("list").accessibility_label(tx(ctx,"files","Changed files"))
        .with_style(ctx.component_style("files",scroller)).scrollbars("hidden","auto");
    column([scroller]).with_key("file-panel").with_style(ctx.component_style("file_panel",style().min_width(px(0))
        .background(theme_color("diff_timeline.card")).radius(theme_radius("lg"))))
}
fn footer(ctx,m){
    let net=m.add-m.del;
    let keys=[];
    let wording=[tx(ctx,"added","Added"),tx(ctx,"removed","Removed"),tx(ctx,"net","Net")];
    for i in 0..3{
        let kind=["add","del","net"][i];
        let swatch=if kind=="net"{
            let bits=[];
            for j in 0..3{bits.push(box([]).with_style(style().width(u(0.55)).height(px(1)).background(color_for(kind))));}
            row(bits).with_style(style().width(u(2.5)).items_center().gap(u(0.35)))
        }else{box([]).with_style(style().width(u(2.5)).height(px(2)).background(color_for(kind)))};
        let swatch=swatch.accessibility_hidden(true);
        let words=[label(ctx,if i==2{"net"}else{"legend"},wording[i],quiet())];
        let net_label=();
        if i==2{
            let amount=signed(ctx,abs(net),if net<0{"−"}else{"+"});
            net_label=wording[i]+" "+amount;
            words[0]=words[0].accessibility_hidden(true);
            words.push(label(ctx,"net_value",amount,line().typography("caption").font_weight(600)
                .font_feature("tnum",1)).with_key("net-value").accessibility_hidden(true));
        }
        let phrase=row(words).with_style(style().items_baseline().gap(sp(0.75)));
        if net_label!=(){phrase=phrase.accessibility_role("text").accessibility_label(net_label);}
        keys.push(row([swatch,phrase]).with_style(style().items_center().gap(sp(1.5))));
    }
    row(keys).with_key("legend").with_style(ctx.component_style("footer",style().items_center().flex_wrap().gap(sp(3)).padding(sp(2))))
}
fn render_timeline(ctx,p){
    validate(p);
    let r=element_ref("timeline");
    let narrow=ctx.element_width_class(r,[440])==0;
    let entries=[];
    for e in p.edits{entries.push(#{file:e.file,at:e.at,added:e.added,removed:e.removed});}
    let m=diff_timeline_projection(entries,get(p,"elapsed",()),get(p,"duration",()));
    let active=active_index(ctx,p,m);
    let panel=column([head(ctx,p,m,narrow),plot(ctx,p,m,active)]).with_key("panel").with_style(ctx.component_style("panel",
        style().min_width(px(0)).radius(theme_radius("lg")).background(theme_color("diff_timeline.card"))));
    let zero=text("0").with_key("count-measure").with_ref(element_ref("file-count-measure")).accessibility_hidden(true)
        .with_style(ctx.component_style("count_measure",style().typography("body").font_feature("tnum",1)
            .absolute().left(px(0)).top(px(0)).whitespace_nowrap().opacity(0.0)));
    column([panel,file_panel(ctx,p,m,active,narrow),footer(ctx,m),zero]).with_key("timeline").with_ref(r)
        .accessibility_role("group").accessibility_label(tx(ctx,"timeline","Diff timeline"))
        .with_style(ctx.component_style("root",style().relative().min_width(px(0)).width(relative(1.0))
            .padding(sp(1)).gap(sp(1)).radius(theme_radius("lg")).background(theme_color("diff_timeline.tray"))))
}
