// Re-execute the now-formal PR101 regression cases, plus independent boundaries.
include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../../tests/native-keyboard/tests/table_boundaries.rs"));
fn mount_no_locale(cx:&mut TestAppContext,script:String,name:&str)->(WindowHandle<Host>,ScriptViewHandle){
 cx.update(gpui_rhai::install);let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");let entry=ModuleId::parse("main").unwrap();
 let read=|path:&str|std::fs::read_to_string(root.join(path)).unwrap();
 let assets=["disclosure_down","chevron_right","sort_ascending","sort_descending"].map(|name|(format!("icons/{name}"),AssetData{mime_type:"image/svg+xml".into(),bytes:std::fs::read(root.join(format!("registry/assets/icons/{name}.svg"))).unwrap()}));
 let prepared=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([(entry,script),(ModuleId::parse("components/table").unwrap(),read("registry/components/table.rhai")),(ModuleId::parse("components/badge").unwrap(),read("registry/components/badge.rhai"))])),read("registry/themes/default_dark.rhai")).asset_sources(assets).prepare().unwrap();
 let captured=Rc::new(RefCell::new(None));let capture=captured.clone();let name=name.to_owned();let window=cx.add_window(move|window,cx|{let host=ScriptViewHost::new(&name,cx).unwrap();let view=prepared.mount(ScriptViewConfig::new(&name),host.clone(),window,cx).unwrap();*capture.borrow_mut()=Some(view.clone());Host{host,view}});cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();let view=captured.borrow().as_ref().unwrap().clone();(window,view)
}
#[gpui::test]
fn table_without_locale_catalog_keeps_ltr_resize_available(cx:&mut TestAppContext){
 let source=script(false,false,false).replace("fn init(ctx){ctx.set_locale(\"en\");}","fn init(ctx){}");
 let(window,view)=mount_no_locale(cx,source,"round6-no-locale");let mut v=VisualTestContext::from_window(*window,cx);let h=bounds(&mut v,&view,"columnheader","A");let s=bounds(&mut v,&view,"separator","Resize A column");assert!((s.x+s.width/2.0-h.x-h.width).abs()<0.5);
 drag(&mut v,point(px((h.x+h.width+2.0)as f32),px((h.y+h.height/2.0)as f32)),40.0);println!("no-locale resize={}",status(&mut v,&view));assert_eq!(status(&mut v,&view),"a:160.0|none");
}
fn last_ref(node:&UiNode)->Option<ElementRef>{
 if let UiNodeKind::Custom{primitive}=node.kind() && primitive.primitive.as_str()=="gpui_rhai.column_resize" && primitive.props.data("column_key")==Some(&UiValue::String("c".into())){return node.element_ref().cloned()}
 match node.kind(){UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(last_ref),_=>None}
}
#[gpui::test]
fn rtl_last_column_ref_keyboard_and_autofit_share_owner(cx:&mut TestAppContext){
 let(window,view)=mount(cx,script(true,true,false),"round6-rtl-last-key-auto");let mut v=VisualTestContext::from_window(*window,cx);let r=v.update(|_,cx|last_ref(&view.root(cx).unwrap().unwrap())).unwrap();v.update(|window,cx|view.focus_element(&r,window,cx)).unwrap();v.simulate_keystrokes("left");v.run_until_parked();assert_eq!(status(&mut v,&view),"c:168.0|none");v.simulate_keystrokes("right");v.run_until_parked();assert_eq!(status(&mut v,&view),"c:160.0|none");
 let h=bounds(&mut v,&view,"separator","Resize C column");let p=point(px((h.x+h.width/2.0)as f32),px((h.y+h.height/2.0)as f32));v.simulate_mouse_move(p,None,Modifiers::none());v.simulate_event(gpui::MouseDownEvent{button:MouseButton::Left,position:p,modifiers:Modifiers::none(),click_count:2,first_mouse:false});v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();let actual=status(&mut v,&view);println!("RTL last autofit={actual}");assert!(actual.starts_with("c:")&&actual.ends_with("|none"));let header=bounds(&mut v,&view,"columnheader","C");let cell=bounds(&mut v,&view,"gridcell","Wide status value with many characters");assert!((header.width-cell.width).abs()<0.5);
}
#[gpui::test]
fn rtl_single_column_can_resize_from_outer_logical_end(cx:&mut TestAppContext){
 let(window,view)=mount(cx,script(true,false,true),"round6-rtl-single");let mut v=VisualTestContext::from_window(*window,cx);let h=bounds(&mut v,&view,"columnheader","C");let s=bounds(&mut v,&view,"separator","Resize C column");assert!((h.x-s.x).abs()<0.5);drag(&mut v,point(px((s.x+2.0)as f32),px((s.y+s.height/2.0)as f32)),-40.0);assert_eq!(status(&mut v,&view),"c:200.0|none");
}
fn inset_scene(rtl:bool)->String{format!(r#"
fn init(ctx){{ctx.set_locale("{}");}}
fn view(ctx){{box([
 box([]).accessibility_role("group").accessibility_label("start").with_style(style().absolute().width(px(8)).height(px(20)).top(px(0)).inset_start(offset_px(-4))),
 box([]).accessibility_role("group").accessibility_label("end").with_style(style().absolute().width(px(8)).height(px(20)).top(px(30)).inset_end(theme_spacing("xs")))
]).accessibility_role("group").accessibility_label("parent").with_style(style().relative().width(px(300)).height(px(100)))}}
"#,if rtl{"ar"}else{"en"})}
#[gpui::test]
fn logical_insets_resolve_signed_and_theme_lengths_without_locale(cx:&mut TestAppContext){
 let source=inset_scene(false).replace("fn init(ctx){ctx.set_locale(\"en\");}","fn init(ctx){}");let(window,view)=mount_no_locale(cx,source,"round6-inset-no-locale");let mut v=VisualTestContext::from_window(*window,cx);let p=bounds(&mut v,&view,"group","parent");let s=bounds(&mut v,&view,"group","start");let e=bounds(&mut v,&view,"group","end");println!("insets LTR parent={p:?},start={s:?},end={e:?}");assert!((s.x-p.x+4.0).abs()<0.5);assert!(e.x>p.x+250.0&&e.x+e.width<p.x+p.width);
}
#[gpui::test]
fn logical_insets_reverse_under_rtl(cx:&mut TestAppContext){
 let(window,view)=mount(cx,inset_scene(true),"round6-inset-rtl");let mut v=VisualTestContext::from_window(*window,cx);let p=bounds(&mut v,&view,"group","parent");let s=bounds(&mut v,&view,"group","start");let e=bounds(&mut v,&view,"group","end");println!("insets RTL parent={p:?},start={s:?},end={e:?}");assert!((s.x+s.width-p.x-p.width-4.0).abs()<0.5);assert!(e.x>p.x&&e.x<p.x+30.0);
}
