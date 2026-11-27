// A deterministic, in-memory Host for the Gallery. No account, network, or real credential.
import "patterns/integration_settings" as settings;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/integration_settings_demo","export":"SettingsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/integration_settings","components/button"],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        integration:#{schema:#{type:"string"},required:true,sensitive:false},
        name:#{schema:#{type:"string"},required:true,sensitive:false},
    },state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("settings_demo_view")
});

fn SettingsDemo(props){render_component("gallery/integration_settings_demo",props)}
fn demo_model(ctx,id){
    let m=ctx.get_state("model");if m!=()&&m.integration==id{return m;}
    #{integration:id,value:#{enabled:true,site:"shop.example",zone:"America/Los_Angeles"},revision:1,
        version:0,visible:false,key_pending:(),key_note:"",pending:(),serial:0,decline:false,save_note:""}
}
fn demo_update(ctx,m){ctx.set_state("model",m);}
fn demo_save(id,ctx,p){
    let m=demo_model(ctx,id);if p.integration!=id||p.revision!=m.revision||m.pending!=()||!m.value.enabled{return;}
    m.serial+=1;m.pending=#{serial:m.serial,revision:p.revision,site:p.site,zone:p.zone,decline:m.decline};
    m.decline=false;m.save_note="";demo_update(ctx,m);
}
fn demo_saved(id,ctx,serial){
    let m=demo_model(ctx,id);let p=m.pending;if p==()||p.serial!=serial{return;}
    m.pending=();
    if p.revision!=m.revision{m.save_note="The Host changed the configuration before this save.";}
    else if p.decline{m.save_note="Save declined by the demo Host.";}
    else{
        m.value.site=p.site;m.value.zone=p.zone;m.revision+=1;m.save_note="Settings saved.";
    }
    demo_update(ctx,m);
}
fn demo_toggle(id,ctx,p){
    let m=demo_model(ctx,id);if p.integration!=id||p.enabled==m.value.enabled{return;}
    m.revision+=1;m.value.enabled=p.enabled;m.visible=false;m.key_pending=();m.pending=();
    m.key_note="";m.save_note="";demo_update(ctx,m);
}
fn demo_generate(id,ctx,p){
    let m=demo_model(ctx,id);if p.integration!=id||p.version!=m.version||m.key_pending!=()||!m.value.enabled{return;}
    m.serial+=1;m.key_pending=m.serial;m.visible=false;m.key_note="";demo_update(ctx,m);
}
fn demo_generated(id,ctx,serial){
    let m=demo_model(ctx,id);if !m.value.enabled||m.key_pending!=serial{return;}
    m.version+=1;m.key_pending=();m.key_note="A sample key was generated.";demo_update(ctx,m);
}
fn demo_key(version){"demo-only-key-"+version.to_string()}
fn demo_key_action(id,action,ctx,p){
    let m=demo_model(ctx,id);
    if p.integration!=id||p.version!=m.version||m.version==0||m.key_pending!=()||!m.value.enabled{return;}
    if action=="copy"{ctx.copy_text(demo_key(m.version));m.key_note="Sample key copied.";}
    else{m.visible=p.visible;}
    demo_update(ctx,m);
}
fn demo_decline(id,ctx,e){
    let m=demo_model(ctx,id);m.decline=true;m.save_note="The demo Host will decline the next save.";demo_update(ctx,m);
}
fn demo_host_edit(id,ctx,e){
    let m=demo_model(ctx,id);m.revision+=1;
    m.value.site=if m.value.site=="host.example"{"shop.example"}else{"host.example"};
    m.save_note="The Host published a new site.";demo_update(ctx,m);
}
fn settings_demo_view(ctx,props){
    let id=props.integration;let m=demo_model(ctx,id);
    if m.pending!=(){timeout("save-reply",360,false,Fn("demo_saved").curry(id),m.pending.serial);}
    if m.key_pending!=(){timeout("generate-reply",360,false,Fn("demo_generated").curry(id),m.key_pending);}
    let preview=if m.visible{text(demo_key(m.version)).with_style(style().typography("caption").text_ellipsis())}else{()};
    let controls=row([
        button::Button(#{key:"decline",text:"Decline next save",size:"sm",variant:"secondary",on_click:Fn("demo_decline").curry(id)}),
        button::Button(#{key:"host-edit",text:"Change site on Host",size:"sm",variant:"secondary",on_click:Fn("demo_host_edit").curry(id)}),
    ]).with_style(style().gap(px(6)).flex_wrap());
    column([
        settings::IntegrationSettings(#{key:"settings",integration:id,name:props.name,meta:"Storefront data sync",count:128,
            value:m.value,revision:m.revision,zones:[#{id:"America/Los_Angeles",label:"Los Angeles"},
                #{id:"America/New_York",label:"New York"},#{id:"Europe/Paris",label:"Paris"},#{id:"Asia/Tokyo",label:"Tokyo"},
                #{id:"Asia/Shanghai",label:"Shanghai"},#{id:"Etc/UTC",label:"UTC"}],
            saving:m.pending!=(),save_message:m.save_note,credential_present:m.version>0,credential_version:m.version,
            credential_busy:m.key_pending!=(),credential_visible:m.visible,credential_preview:preview,credential_message:m.key_note,
            on_save:Fn("demo_save").curry(id),on_toggle:Fn("demo_toggle").curry(id),on_generate_key:Fn("demo_generate").curry(id),
            on_copy_key:Fn("demo_key_action").curry(id,"copy"),on_reveal_key:Fn("demo_key_action").curry(id,"reveal")}),
        controls,
        text("Accepted configuration · revision "+m.revision.to_string()+": "+m.value.site+" · "+m.value.zone)
            .test_id("integration-accepted").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(props.key).with_style(ctx.component_style("root",style().gap(px(8)).min_width(px(0)).items_start()))
}
