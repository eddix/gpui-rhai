//! Public-API contribution lifetime probes, cda11ce7, no native window needed.
use gpui_rhai::*;
use std::{cell::RefCell,collections::{BTreeMap,BTreeSet},rc::Rc};
const COMMON:&str=r#"
fn state_schema(){#{fields:#{direct:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}},swap:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
fn noop(ctx,value){()}
fn read(ctx,key){
 let scalar=ctx.get_app_store_path("probe","model",[key]);
 let window_scalar=ctx.get_window_store_path("probe","model",[key]);
 let doc=ctx.get_native_text_document(key);
 let direction=ctx.text_direction();let viewport=ctx.viewport_class();let duration=ctx.motion_duration("fast");
 try {let rows=ctx.get_native_collection(key);return text("ready").on_click(Fn("noop"));}
 catch(error){return text(error.to_string()).on_click(Fn("noop"));}
}
define_component(#{metadata:#{id:"audit/reader","export":"Reader",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("formal_row")});
fn formal_row(ctx,props){let shared_value=read(ctx,"shared");let result=read(ctx,props.key);if props.key=="row-79"{throw "failed-row";}result}
fn rows(swap){let data=[];for i in 0..80{let index=if swap{(i+1)%80}else{i};data.push(#{key:`row-${index}`});}data}
fn list(ctx,key){virtual_collection(#{key:key,label:key,data:rows(ctx.get_state("swap")),height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))}
fn view(ctx){let nodes=[];if ctx.get_state("direct"){nodes.push(read(ctx,"root-source"));let shared_value=read(ctx,"shared");}nodes.push(list(ctx,"a"));nodes.push(list(ctx,"b"));column(nodes)}
"#;
fn find<'a>(node:&'a UiNode,key:&str)->Option<&'a VirtualCollectionNodeSpec>{
 match node.kind(){UiNodeKind::VirtualCollection{spec}=>if spec.id.key==key{Some(spec)}else{spec.realized.values().find_map(|n|find(n,key))},
 UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(|n|find(n,key)),_=>None}
}
struct Harness{engine:RuntimeEngine,runtime:Rc<RefCell<UiRuntimeState>>,life:ScriptLifecycle,root:ComponentInstancePath,theme:ThemeVariant,formal:bool}
impl Harness{
 fn new(formal:bool)->Self{
  let mut engine=RuntimeEngine::new();let repo=std::env::var("GPUI_RHAI_AUDIT_ROOT").unwrap();
  let theme=load_theme_source(engine.engine(),"theme",&std::fs::read_to_string(format!("{repo}/registry/themes/default_dark.rhai")).unwrap()).unwrap();
  let locale=load_locale_source(engine.engine(),"locale",&std::fs::read_to_string(format!("{repo}/registry/locales/en.rhai")).unwrap()).unwrap();
  let mut runtime=UiRuntimeState::new();runtime.locale=Some(LocaleManager::new([locale],"en","en").unwrap());runtime.theme=Some(ThemeManager::from_variants([theme.clone()],ThemeSelection::new("Default","Dark")).unwrap());
  let names=(0..80).map(|i|format!("row-{i}")).chain(["root-source".into(),"shared".into()]).collect::<Vec<String>>();
  let model=UiValue::Map(names.iter().map(|n|(n.clone(),UiValue::Integer(0))).collect());
  let schema=ComponentStateSchema::new(BTreeMap::from([("model".into(),StateField::new(ValueSchema::UiValue,model))])).unwrap();
  runtime.stores.declare(StoreId::app("probe"),schema.clone()).unwrap();runtime.stores.declare(StoreId::window("main","probe"),schema).unwrap();
  for name in names{runtime.native_documents.register(name.clone(),NativeTextDocument::new(name,0,"text").unwrap()).unwrap();}
  let row=if formal{r#"fn row(ctx,p){render_component("audit/reader",#{key:p.key})}"#}else{r#"fn row(ctx,p){let shared_value=read(ctx,"shared");let result=read(ctx,p.key);if p.key=="row-79"{throw "failed-row";}result}"#};
  let compiled=engine.compile(&format!("{COMMON}\n{row}")).unwrap();let schema=engine.root_state_schema(&compiled).unwrap();
  let runtime=Rc::new(RefCell::new(runtime));let root=ComponentInstancePath::root("App","reads");let mut life=ScriptLifecycle::new(compiled,runtime.clone(),root.clone(),Some("main".into()),BTreeMap::new(),&schema).unwrap();life.start(&mut engine).unwrap();
  Self{engine,runtime,life,root,theme,formal}
 }
 fn target(&mut self,key:&str,indices:impl IntoIterator<Item=usize>)->Result<bool,LifecycleError>{let id=find(self.life.root().unwrap(),key).unwrap().id.clone();self.runtime.borrow().virtual_requests.request(id,indices);self.life.realize_virtual_requests(&mut self.engine)}
 fn owner(&self,key:&str,row:&str)->ComponentInstancePath{if self.formal{self.root.child("VirtualCollection",key).child("Reader",row)}else{self.root.clone()}}
 fn counts(&self)->(usize,usize,usize){let snapshot=InspectorSnapshot::capture(self.life.root(),&self.runtime.borrow(),&self.theme,&self.engine.component_exports().unwrap(),vec![]);(snapshot.locale_readers,snapshot.theme_readers,snapshot.viewport_readers)}
 fn store_readers(&self,key:&str,revision:i64,window:bool)->BTreeSet<ComponentInstancePath>{let id=if window{StoreId::window("main","probe")}else{StoreId::app("probe")};let path=UiValuePath::new(vec![UiValuePathSegment::Key(key.into())]).unwrap();self.runtime.borrow_mut().stores.write_path(&id,"model",&path,UiValue::Integer(revision)).unwrap()}
 fn document_readers(&self,key:&str,revision:u64)->BTreeSet<ComponentInstancePath>{self.runtime.borrow_mut().native_documents.replace(key,NativeTextDocument::new(key,revision,format!("rev-{revision}")).unwrap()).unwrap()}
 fn register(&self,key:&str){self.runtime.borrow_mut().register_native_collection_from_host(&self.root,key,NativeCollection::new("id",std::iter::empty::<BTreeMap<String,UiValue>>()).unwrap()).unwrap();}
}

#[test]
fn raw_and_formal_66_rows_prune_all_registry_contributions_without_losing_shared_readers(){
 for formal in [false,true]{
  let mut h=Harness::new(formal);assert_eq!(h.counts(),(5,5,5));
  for i in 2..68{h.target("a",[i]).unwrap();assert_eq!(h.counts(),(4,4,4),"formal={formal} index={i}");}
  let a=find(h.life.root().unwrap(),"a").unwrap();assert_eq!(a.realized.len(),1);assert!(matches!(a.realized[&67].kind(),UiNodeKind::Text{text} if text.contains("not registered") && !text.contains("limit")));
  for window in [false,true]{assert!(h.store_readers("row-2",1,window).is_empty());assert_eq!(h.store_readers("row-0",1,window),BTreeSet::from([h.owner("b","row-0")]));}
  assert!(h.document_readers("row-2",1).is_empty());assert_eq!(h.document_readers("row-0",1),BTreeSet::from([h.owner("b","row-0")]));
  h.register("row-2");assert!(!h.life.render_dirty(&mut h.engine).unwrap());
  h.register("row-67");assert!(h.life.render_dirty(&mut h.engine).unwrap());assert!(matches!(find(h.life.root().unwrap(),"a").unwrap().realized[&67].kind(),UiNodeKind::Text{text} if text=="ready"));
  h.target("a",[]).unwrap();assert_eq!(h.counts(),(3,3,3));h.target("b",[]).unwrap();assert_eq!(h.counts(),(1,1,1));
  assert_eq!(h.store_readers("shared",1,false),BTreeSet::from([h.root.clone()]));assert_eq!(h.document_readers("shared",1),BTreeSet::from([h.root.clone()]));
  h.runtime.borrow_mut().set_component_state_from_host(&h.root,"direct",UiValue::Bool(false)).unwrap();h.life.render_dirty(&mut h.engine).unwrap();
  let reseeded=(find(h.life.root().unwrap(),"a").unwrap().realized.keys().copied().collect::<Vec<_>>(),find(h.life.root().unwrap(),"b").unwrap().realized.keys().copied().collect::<Vec<_>>());
  println!("ROOT_RERENDER formal={formal} legitimate_reseed={reseeded:?} readers={:?}",h.counts());
  h.target("a",[]).unwrap();h.target("b",[]).unwrap();assert!(find(h.life.root().unwrap(),"a").unwrap().realized.is_empty());assert!(find(h.life.root().unwrap(),"b").unwrap().realized.is_empty());assert_eq!(h.counts(),(0,0,0));
  assert!(h.store_readers("shared",2,false).is_empty());assert!(h.document_readers("shared",2).is_empty());h.register("shared");assert!(!h.life.render_dirty(&mut h.engine).unwrap());
  println!("LIFETIME formal={formal} 66_targets_bounded=true offscreen_store_document_missing_edges_removed=true other_collection_and_direct_survive=true last_contribution_removed=true");
 }
}

#[test]
fn failed_candidate_restores_reads_and_discards_its_new_contributions(){
 for formal in [false,true]{
  let mut h=Harness::new(formal);h.target("a",[65]).unwrap();let counts=h.counts();assert!(h.target("a",[79]).is_err());assert_eq!(h.counts(),counts);
  for window in [false,true]{assert_eq!(h.store_readers("row-65",1,window),BTreeSet::from([h.owner("a","row-65")]));assert!(h.store_readers("row-79",1,window).is_empty());}
  assert_eq!(h.document_readers("row-65",1),BTreeSet::from([h.owner("a","row-65")]));assert!(h.document_readers("row-79",1).is_empty());
  h.register("row-79");assert!(!h.life.render_dirty(&mut h.engine).unwrap());h.register("row-65");assert!(h.life.render_dirty(&mut h.engine).unwrap());
  println!("ROLLBACK formal={formal} previous_edges_preserved=true failed_candidate_edges_gone=true");
 }
}

#[test]
fn reordered_stable_keys_replace_contributions_using_the_new_realized_keys(){
 for formal in [false,true]{
  let mut h=Harness::new(formal);h.target("a",[65]).unwrap();
  h.runtime.borrow_mut().set_component_state_from_host(&h.root,"swap",UiValue::Bool(true)).unwrap();h.life.render_dirty(&mut h.engine).unwrap();
  let keys=find(h.life.root().unwrap(),"a").unwrap().realized.values().map(|node|node.key().unwrap().as_str().to_owned()).collect::<Vec<_>>();
  assert_eq!(keys,vec!["row-1","row-2"]);assert_eq!(h.counts(),(5,5,5));assert!(h.store_readers("row-65",1,false).is_empty());assert!(h.document_readers("row-65",1).is_empty());
  let expected=BTreeSet::from([h.owner("a","row-1"),h.owner("b","row-1")]);assert_eq!(h.store_readers("row-1",1,false),expected);assert_eq!(h.document_readers("row-1",1),expected);
  h.register("row-65");assert!(!h.life.render_dirty(&mut h.engine).unwrap());h.register("row-1");assert!(h.life.render_dirty(&mut h.engine).unwrap());
  println!("REORDER formal={formal} realized_keys={keys:?} old_edges_removed=true current_key_wakes_correct_owner=true");
 }
}
