//! Actual preparation counts across row eviction, not an Entity-retention proxy. The
//! transcript renders no batch ahead and keeps no rows, so rows leave and their viewers go.

use crate::support::{Mount, Step, bounds, by_test_id, click_center, dispatch, last_error, settle};
use gpui::TestAppContext;
use gpui_rhai::{
    DocumentRuntimeConfig, MarkdownPreparation, MarkdownPreparationCache, MarkdownPreparationKey,
    NativeTextDocument, PreparedMarkdown, RuntimeEngine, ScriptViewExtension, UiRuntimeState,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

#[derive(Default)]
struct Cache {
    values: Mutex<HashMap<MarkdownPreparationKey, PreparedMarkdown>>,
    decline: bool,
    wrong: Option<PreparedMarkdown>,
    max_entries: Option<usize>,
}
impl MarkdownPreparationCache for Cache {
    fn get(&self, key: &MarkdownPreparationKey) -> Option<PreparedMarkdown> {
        self.wrong
            .clone()
            .or_else(|| self.values.lock().unwrap().get(key).cloned())
    }
    fn insert(&self, value: PreparedMarkdown) {
        if !self.decline {
            let mut values = self.values.lock().unwrap();
            if self.max_entries.is_some_and(|limit| values.len() >= limit) {
                values.clear();
            }
            values.insert(value.key().clone(), value);
        }
    }
}
struct Extension {
    cache: Option<Arc<Cache>>,
    config: Rc<RefCell<Option<DocumentRuntimeConfig>>>,
    prefetch: bool,
}
fn document(i: usize, revision: u64) -> NativeTextDocument {
    NativeTextDocument::new(format!("reply/{i}"), revision, Arc::<str>::from(format!("# Reply {i}\n\nAn unchanged **reply** with stable revision {revision}.\n\n```rhai\nlet answer = {i};\n```\n\nA final paragraph.\n"))).unwrap()
}

#[gpui::test]
fn cached_model_is_ready_for_native_copy_on_its_first_presentation(cx: &mut TestAppContext) {
    let config = Rc::new(RefCell::new(None));
    let extension = Extension {
        cache: Some(Arc::new(Cache::default())),
        config: config.clone(),
        prefetch: true,
    };
    let source = r#"import "components/markdown_viewer" as markdown;
fn view(ctx) { markdown::MarkdownViewer(#{key:"reply",label:"Ready reply",source:ctx.get_native_text_document("agent-document/0")})
.with_style(style().width(px(520)).height(px(320))) }"#;
    let (mut visual, view) = Mount::components(source)
        .with(move |v| v.extension(extension))
        .id("first-cached-presentation")
        .mount(cx);
    settle(&mut visual, Step::Frame, 1);
    let rect = bounds(&mut visual, &view, "document", "Ready reply");
    click_center(&mut visual, rect);
    visual.simulate_keystrokes("alt-cmd-c");
    let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some(document(0, 7).text()));
    assert_eq!(
        config
            .borrow()
            .as_ref()
            .unwrap()
            .markdown_preparation_stats()
            .jobs_started,
        0
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
impl ScriptViewExtension for Extension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        let config = engine.document_runtime_config();
        config
            .set_markdown_cache(
                self.cache
                    .clone()
                    .map(|cache| cache as Arc<dyn MarkdownPreparationCache>),
            )
            .unwrap();
        if self.prefetch {
            for i in 0..24 {
                let request = MarkdownPreparation::new(
                    document(i, 7).into(),
                    &engine.syntax_registry(),
                    config.limits(),
                )
                .unwrap();
                self.cache
                    .as_ref()
                    .unwrap()
                    .insert(request.prepare(&AtomicBool::new(false)).unwrap());
            }
        }
        *self.config.borrow_mut() = Some(config);
        Ok(())
    }
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        for i in 0..24 {
            runtime
                .native_documents
                .register(format!("agent-document/{i}"), document(i, 7))
                .unwrap();
        }
        Ok(())
    }
}
const SOURCE: &str = r#"
import "components/markdown_viewer" as markdown;
fn top(ctx,value){ctx.scroll_collection_to_start("agent-transcript");}
fn bottom(ctx,value){ctx.scroll_collection_to_end("agent-transcript");}
fn item(ctx,payload){
 markdown::MarkdownViewer(#{key:`message-${payload.key}`,label:`Reply ${payload.index}`,
 source:ctx.get_native_text_document(payload.item.document),height:"content"}).with_key(payload.key)
}
fn view(ctx){
 let data=[];for i in 0..24 {data.push(#{key:`message-${i}`,document:`agent-document/${i}`});}
 column([row([text("History").test_id("history").on_click(Fn("top")),
 text("Latest").test_id("latest").on_click(Fn("bottom"))]),
 virtual_collection(#{key:"agent-transcript",data:data,estimated_height:120,overdraw_pixels:240,
 batch_items:1,retain_items:0,fill_height:true,alignment:"bottom",follow_tail:true},Fn("item"))
 .with_style(style().flex_grow().flex_basis(px(0)).min_height(px(0)))])
 .with_style(style().width(px(520)).height(px(320)))
}
"#;

#[gpui::test]
fn host_capacity_eviction_causes_safe_repreparation_without_limiting_live_ui(
    cx: &mut TestAppContext,
) {
    let config = Rc::new(RefCell::new(None));
    let cache = Arc::new(Cache {
        max_entries: Some(1),
        ..Cache::default()
    });
    let extension = Extension {
        cache: Some(cache.clone()),
        config: config.clone(),
        prefetch: false,
    };
    let (mut visual, view) = Mount::components(SOURCE)
        .with(move |v| v.extension(extension))
        .id("bounded-transcript")
        .mount(cx);
    settle(&mut visual, Step::Tick, 12);
    let config = config.borrow().clone().unwrap();
    let first = config.markdown_preparation_stats().jobs_started;
    for id in ["history", "latest", "history", "latest"] {
        dispatch(&mut visual, &view, by_test_id(id), "click").unwrap();
        settle(&mut visual, Step::Tick, 12);
        assert!(cache.values.lock().unwrap().len() <= 1);
        assert_eq!(last_error(&mut visual, &view), None);
    }
    assert!(config.markdown_preparation_stats().jobs_started > first);
}

#[gpui::test]
fn revisiting_evicted_rows_reuses_preparation_without_retaining_viewers(cx: &mut TestAppContext) {
    let config = Rc::new(RefCell::new(None));
    let extension = Extension {
        cache: Some(Arc::new(Cache::default())),
        config: config.clone(),
        prefetch: false,
    };
    let (mut visual, view) = Mount::components(SOURCE)
        .with(move |v| v.extension(extension))
        .id("cached-transcript")
        .mount(cx);
    settle(&mut visual, Step::Tick, 12);
    let config = config.borrow().clone().unwrap();
    let mut warm_jobs = 0;
    for pass in 0..3 {
        for id in ["history", "latest"] {
            dispatch(&mut visual, &view, by_test_id(id), "click").unwrap();
            settle(&mut visual, Step::Tick, 12);
            assert_eq!(last_error(&mut visual, &view), None);
        }
        let stats = config.markdown_preparation_stats();
        if pass == 0 {
            warm_jobs = stats.jobs_started;
        } else {
            assert_eq!(
                stats.jobs_started, warm_jobs,
                "unchanged revisits must not launch parser jobs"
            );
        }
    }
    assert!(warm_jobs > 0);
    assert!(config.markdown_preparation_stats().cache_hits > 0);
    // A new revision at the live tail must not flush the old immutable models.
    visual.update(|_, cx| {
        view.replace_native_text_document("agent-document/23", document(23, 8), cx)
            .unwrap()
    });
    settle(&mut visual, Step::Tick, 12);
    let streamed_jobs = config.markdown_preparation_stats().jobs_started;
    assert!(streamed_jobs > warm_jobs);
    for id in ["history", "latest"] {
        dispatch(&mut visual, &view, by_test_id(id), "click").unwrap();
        settle(&mut visual, Step::Tick, 12);
    }
    assert_eq!(
        config.markdown_preparation_stats().jobs_started,
        streamed_jobs
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn host_prefetched_models_present_without_any_background_parse(cx: &mut TestAppContext) {
    let config = Rc::new(RefCell::new(None));
    let cache = Arc::new(Cache::default());
    let extension = Extension {
        cache: Some(cache.clone()),
        config: config.clone(),
        prefetch: true,
    };
    let (mut visual, view) = Mount::components(SOURCE)
        .with(move |v| v.extension(extension))
        .id("prefetched-transcript")
        .mount(cx);
    settle(&mut visual, Step::Tick, 12);
    let config = config.borrow().clone().unwrap();
    assert_eq!(config.markdown_preparation_stats().jobs_started, 0);
    assert!(config.markdown_preparation_stats().cache_hits > 0);
    assert_eq!(last_error(&mut visual, &view), None);
    // UI disposal does not clear an application-owned pool.
    visual.update(|window, cx| {
        view.dispose(cx).unwrap();
        window.remove_window();
    });
    assert_eq!(cache.values.lock().unwrap().len(), 24);
    config.set_markdown_cache(None).unwrap();
    assert!(config.markdown_cache().is_none());
}

#[gpui::test]
fn a_host_may_decline_every_entry_and_revisits_prepare_again(cx: &mut TestAppContext) {
    let config = Rc::new(RefCell::new(None));
    let extension = Extension {
        cache: Some(Arc::new(Cache {
            decline: true,
            ..Cache::default()
        })),
        config: config.clone(),
        prefetch: false,
    };
    let (mut visual, view) = Mount::components(SOURCE)
        .with(move |v| v.extension(extension))
        .id("no-admission")
        .mount(cx);
    settle(&mut visual, Step::Tick, 12);
    let config = config.borrow().clone().unwrap();
    let initial = config.markdown_preparation_stats().jobs_started;
    for id in ["history", "latest", "history", "latest"] {
        dispatch(&mut visual, &view, by_test_id(id), "click").unwrap();
        settle(&mut visual, Step::Tick, 12);
    }
    assert!(config.markdown_preparation_stats().jobs_started > initial);
    assert_eq!(config.markdown_preparation_stats().cache_hits, 0);
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn a_wrong_model_from_host_cache_is_a_miss_not_stale_presentation(cx: &mut TestAppContext) {
    let config = Rc::new(RefCell::new(None));
    let wrong = MarkdownPreparation::new(
        "wrong secret document".to_owned().into(),
        &gpui_rhai::SyntaxRegistry::new(),
        gpui_rhai::DocumentLimits::default(),
    )
    .unwrap()
    .prepare(&AtomicBool::new(false))
    .unwrap();
    let extension = Extension {
        cache: Some(Arc::new(Cache {
            wrong: Some(wrong),
            ..Cache::default()
        })),
        config: config.clone(),
        prefetch: false,
    };
    let (mut visual, view) = Mount::components(SOURCE)
        .with(move |v| v.extension(extension))
        .id("wrong-cache")
        .mount(cx);
    settle(&mut visual, Step::Tick, 12);
    let stats = config
        .borrow()
        .as_ref()
        .unwrap()
        .markdown_preparation_stats();
    assert!(stats.jobs_started > 0);
    assert_eq!(stats.cache_hits, 0);
    assert_eq!(last_error(&mut visual, &view), None);
}
