//! A view the Host sizes (`host_sets_width(true)`) keeps the heights of its
//! islands right when the Host resizes it without resizing the window (#215):
//! a pane of one window narrows and widens while a second window, which
//! renders every frame in full, runs the same script at the same widths.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    WindowHandle, div, px, size,
};
use gpui_rhai::{ScriptViewHandle, ScriptViewHost};

use crate::support::{Mount, Step, by_test_id, dispatch, painted_quads, settle, text_of};

/// A view state count above 32 paragraphs that wrap at the pane's width: a
/// click renders the view whole, and the paragraphs are one cold island.
const PARAGRAPHS: &str = r#"fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn bump(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
fn paragraphs(){
    let result=[];
    for i in 0..32{
        result.push(text(`${i}. A long detail that wraps when the host narrows its pane while the window keeps its size. This paragraph must keep the same height with cache islands on and off.`)
            .with_key(`p${i}`).with_style(style().width(relative(1.0)).whitespace_normal()));
    }
    result
}
fn view(ctx){
    column([
        text(`Count ${ctx.get_state("count")}`).test_id("count"),
        text("Bump").test_id("bump").on_click(Fn("bump")),
        column(paragraphs()).with_key("cold").with_style(style().width(relative(1.0))),
    ]).with_style(style().width(relative(1.0)))
}"#;

/// A window root that lays the view out in a pane of `width`.
struct Pane {
    host: ScriptViewHost,
    view: ScriptViewHandle,
    width: Rc<Cell<f32>>,
}

impl Render for Pane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            div()
                .w(px(self.width.get()))
                .h_full()
                .child(self.view.element().unwrap()),
        )
    }
}

struct Side {
    window: WindowHandle<Pane>,
    visual: VisualTestContext,
    view: ScriptViewHandle,
    width: Rc<Cell<f32>>,
}

fn mount(cx: &mut TestAppContext, islands: bool) -> Side {
    let width = Rc::new(Cell::new(900.0));
    let pane_width = Rc::clone(&width);
    let (window, view) = Mount::new(PARAGRAPHS)
        .ids(if islands { "pane-b" } else { "pane-a" }, "view")
        .config(move |config| config.host_sets_width(true).render_islands(islands))
        .mount_root(cx, move |host, view| Pane {
            host,
            view,
            width: pane_width,
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(900.0), px(1000.0)));
    settle(&mut visual, Step::Tick, 2);
    Side {
        window,
        visual,
        view,
        width,
    }
}

impl Side {
    fn resize_pane(&mut self, width: f32) {
        self.width.set(width);
        self.window
            .update(&mut self.visual, |_, _, cx| cx.notify())
            .unwrap();
        settle(&mut self.visual, Step::Tick, 2);
    }

    fn bump(&mut self) {
        dispatch(&mut self.visual, &self.view, by_test_id("bump"), "click").unwrap();
        settle(&mut self.visual, Step::Tick, 2);
    }

    fn statistics(&mut self) -> gpui_rhai::ScriptViewRenderStatistics {
        self.visual
            .update(|_, cx| self.view.take_render_statistics(cx))
            .unwrap()
    }

    fn quads(&mut self) -> Vec<String> {
        painted_quads(&mut self.visual)
            .iter()
            .map(|quad| format!("{quad:?}"))
            .collect()
    }

    fn geometry(&mut self) -> String {
        let digest = self
            .visual
            .update(|window, cx| self.view.debug_frame_digest(window, cx).unwrap());
        format!("{:?}", digest.geometry)
    }
}

#[track_caller]
fn assert_same(a: &mut Side, b: &mut Side, label: &str) {
    assert_eq!(a.geometry(), b.geometry(), "{label}: geometry differs");
    assert_eq!(a.quads(), b.quads(), "{label}: painted quads differ");
}

#[gpui::test]
fn a_pane_the_host_resizes_keeps_its_island_heights(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut a = mount(cx, false);
    let mut b = mount(cx, true);
    for side in [&mut a, &mut b] {
        side.bump();
        side.bump();
    }
    let _ = a.statistics();
    b.bump();
    a.bump();
    assert!(b.statistics().island_hits > 0, "the paragraphs replay");
    assert_same(&mut a, &mut b, "at 900");
    for width in [420.0, 700.0, 340.0] {
        let _ = (a.statistics(), b.statistics());
        a.resize_pane(width);
        b.resize_pane(width);
        let resized = b.statistics();
        assert_eq!(resized.island_drift, 0, "at {width}: {resized:?}");
        assert_same(&mut a, &mut b, &format!("resized to {width}"));
        // The islands measure at the new width and replay again.
        for side in [&mut a, &mut b] {
            side.bump();
            side.bump();
        }
        let replayed = b.statistics();
        assert!(replayed.island_hits > 0, "at {width}: {replayed:?}");
        assert_eq!(replayed.island_drift, 0, "at {width}: {replayed:?}");
        assert_same(&mut a, &mut b, &format!("replayed at {width}"));
        assert_eq!(
            text_of(&mut a.visual, &a.view, "count"),
            text_of(&mut b.visual, &b.view, "count")
        );
    }
}
