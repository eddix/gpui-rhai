use std::{cell::RefCell, rc::Rc, time::{Duration,Instant}};
use gpui_rhai::{AssetData,AssetId,AssetProvider,AssetRegistry,CapabilityHandler,RuntimeEngine,ScriptGeneration,AsyncScope,ComponentInstancePath,UiValue};

struct MutableProvider(Rc<RefCell<AssetData>>);
impl AssetProvider for MutableProvider {
    fn load(&self,_:&str)->Result<AssetData,String>{Ok(self.0.borrow().clone())}
}
struct ForegroundRefresh {assets:AssetRegistry}
impl CapabilityHandler for ForegroundRefresh {
    fn call(&mut self,_:&str,_:UiValue)->Result<UiValue,String>{
        self.assets.refresh_namespace("app").map(|n|UiValue::Integer(n as i64)).map_err(|e|e.to_string())
    }
}
fn svg(color:&str)->AssetData {
    AssetData{mime_type:"image/svg+xml".to_owned(),bytes:format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><rect width=\"1\" height=\"1\" fill=\"{color}\"/></svg>").into_bytes()}
}
fn fingerprint(registry:&AssetRegistry,handle:&gpui_rhai::ImageHandle)->u64 {
    match registry.image_source(handle.opaque()).unwrap(){gpui::ImageSource::Image(image)=>image.id(),_=>panic!("expected prepared image")}
}
fn queue(registry:&AssetRegistry,id:&AssetId)->ScriptGeneration {
    let mut engine=RuntimeEngine::new();
    let compiled=engine.compile("fn loaded(ctx,value){} fn failed(ctx,value){} fn view(ctx){text(\"probe\")}").unwrap();
    let generation=compiled.generation();
    registry.start_image_decode(id,AsyncScope::Component(ComponentInstancePath::root("App","probe")),generation,
        engine.callback(&compiled,"loaded").unwrap(),
        engine.callback(&compiled,"failed").unwrap()).unwrap();
    generation
}
fn drain(registry:&AssetRegistry,generation:ScriptGeneration) {
    let start=Instant::now();
    while registry.pending_decode_count()>0 {
        let _=registry.drain_image_decodes(generation).unwrap();
        assert!(start.elapsed()<Duration::from_secs(3),"worker did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main(){
    let data=Rc::new(RefCell::new(svg("red")));let registry=AssetRegistry::new();
    registry.register("app",MutableProvider(data.clone())).unwrap();
    let id=AssetId::parse("app/cover").unwrap();let handle=registry.load_image(&id).unwrap();
    let red=fingerprint(&registry,&handle);*data.borrow_mut()=svg("blue");
    assert_eq!(registry.load_image(&id).unwrap(),handle);assert_eq!(fingerprint(&registry,&handle),red);
    println!("1 cached load_image keeps old image: confirmed");
    let generation=queue(&registry,&id);drain(&registry,generation);assert_eq!(fingerprint(&registry,&handle),red);
    println!("2 start_image_decode on cached AssetId also keeps old image: confirmed");
    let mut foreground=ForegroundRefresh{assets:registry.clone()};
    assert_eq!(foreground.call("refresh",UiValue::Null).unwrap(),UiValue::Integer(1));
    assert_eq!(registry.load_image(&id).unwrap(),handle);let blue=fingerprint(&registry,&handle);assert_ne!(red,blue);
    println!("3 foreground non-Send CapabilityHandler refreshes stable opaque handle and changes GPUI content id: confirmed");
    *data.borrow_mut()=AssetData{mime_type:"image/png".to_owned(),bytes:b"not a png".to_vec()};
    let accepted=registry.refresh_namespace("app").unwrap();assert_eq!(accepted,1);
    match registry.image_source(handle.opaque()).unwrap(){gpui::ImageSource::Image(image)=>assert_eq!(image.bytes,b"not a png"),_=>panic!()}
    println!("4 namespace refresh accepts nonempty corrupt raster bytes before GPUI decode: confirmed");
    let inflight_data=Rc::new(RefCell::new(svg("red")));let inflight=AssetRegistry::new();
    inflight.register("app",MutableProvider(inflight_data.clone())).unwrap();let generation=queue(&inflight,&id);
    *inflight_data.borrow_mut()=svg("blue");assert_eq!(inflight.refresh_namespace("app").unwrap(),0);
    drain(&inflight,generation);let installed=inflight.load_image(&id).unwrap();assert_eq!(fingerprint(&inflight,&installed),red);
    println!("5 refresh_namespace skips pending-only asset; old prepared bytes can install later: confirmed");
}
