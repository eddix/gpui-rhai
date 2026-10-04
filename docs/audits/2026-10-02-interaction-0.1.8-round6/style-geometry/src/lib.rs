#[cfg(test)]
mod tests {
    use gpui_rhai::*;
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    #[test]
    fn public_style_inputs_and_shared_defaults_have_value_semantics() {
        let empty = UiNode::text("empty");
        let default_sibling = UiNode::box_node(Vec::new());
        assert!(std::ptr::eq(empty.style(), default_sibling.style()));
        let mut input = Style::new()
            .width(Length::Pixels(100.))
            .opacity(0.25)
            .unwrap()
            .font_family("Host Test Face")
            .unwrap()
            .font_fallbacks(vec!["Host Fallback".into()])
            .unwrap()
            .font_feature("tnum", 1)
            .unwrap()
            .inset_start(SignedLength::pixels(-4.).unwrap());
        let original = empty.with_style(&input);
        let clone = original.clone();
        assert!(std::ptr::eq(original.style(), clone.style()));
        input.base.width = Some(Length::Pixels(999.).into());
        input
            .base
            .font_features
            .as_mut()
            .unwrap()
            .insert("tnum".into(), 0);
        input.base.font_fallbacks.as_mut().unwrap()[0] = "changed".into();
        let changed = clone.with_style(
            &Style::new()
                .width(Length::Pixels(200.))
                .opacity(0.75)
                .unwrap()
                .inset_end(Length::Pixels(2.)),
        );
        assert_eq!(
            original.style().base.width,
            Some(Length::Pixels(100.).into())
        );
        assert_eq!(original.style().base.opacity, Some(0.25));
        assert_eq!(
            original.style().base.font_features.as_ref().unwrap()["tnum"],
            1
        );
        assert_eq!(
            original.style().base.font_fallbacks.as_ref().unwrap()[0],
            "Host Fallback"
        );
        assert!(original.style().base.inset_end.is_none());
        assert_eq!(
            changed.style().base.width,
            Some(Length::Pixels(200.).into())
        );
        assert_eq!(changed.style().base.opacity, Some(0.75));
        assert!(changed.style().base.inset_start.is_some());
        assert!(changed.style().base.inset_end.is_some());
        assert!(!std::ptr::eq(original.style(), changed.style()));
        assert_eq!(default_sibling.style(), &Style::new());
        assert_eq!(UiNode::text("future").style(), &Style::new());
        println!("input maps/vectors, sibling clones and future shared defaults isolated");
    }

    #[test]
    fn pseudo_states_and_nested_node_clones_do_not_leak_mutation() {
        let base = Style::new()
            .opacity(0.1)
            .unwrap()
            .hover(&Style::new().opacity(0.2).unwrap())
            .focus(&Style::new().opacity(0.3).unwrap())
            .disabled(&Style::new().opacity(0.4).unwrap());
        let child = UiNode::text("child").with_style(&base);
        let original = UiNode::box_node(vec![child.clone()]);
        let frozen = original.clone();
        let changed_child = child.clone().with_style(
            &Style::new()
                .focus(&Style::new().opacity(0.8).unwrap())
                .active(&Style::new().opacity(0.7).unwrap()),
        );
        let changed = UiNode::box_node(vec![changed_child]);
        let first = |n: &UiNode| match n.kind() {
            UiNodeKind::Box { children } => children[0].clone(),
            _ => panic!(),
        };
        let after = first(&changed);
        let before = first(&frozen);
        for (state, old, new) in [
            (PseudoState::Hovered, 0.2, 0.2),
            (PseudoState::Focused, 0.3, 0.8),
            (PseudoState::Active, 0.1, 0.7),
            (PseudoState::Disabled, 0.4, 0.4),
        ] {
            let s = InteractionState::default().with(state);
            assert_eq!(before.style().resolve(&s).opacity, Some(old));
            assert_eq!(after.style().resolve(&s).opacity, Some(new));
        }
        let disabled_focus = InteractionState::default()
            .with(PseudoState::Focused)
            .with(PseudoState::Disabled);
        assert_eq!(after.style().resolve(&disabled_focus).opacity, Some(0.4));
        assert_eq!(first(&original).style(), child.style());
        assert_eq!(changed.style(), &Style::new());
        println!("hover/active/focus/disabled and nested clone isolation pass");
    }

    const COMPONENT: &str = r#"/* gpui-rhai
{"id":"components/cow_probe","export":"Counter","version":"0.1.8","runtime_api":{"min_inclusive":2,"max_exclusive":3},"dependencies":[],"capabilities":{}}
*/
define_component(#{metadata:#{id:"components/cow_probe","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Counter")});
fn Counter(props){render_component("components/cow_probe",props)}
fn increment(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
fn render_Counter(ctx,props){text(`${ctx.get_state("count")}`).with_style(ctx.component_style("root",style().width(px(100+ctx.get_state("count")*10)).opacity(0.2))).on_click(Fn("increment"))}
"#;
    #[test]
    fn formal_owned_snapshot_update_keeps_old_tree_and_outer_presentations_isolated() {
        let source = EmbeddedScriptSource::new(BTreeMap::from([(
            ModuleId::parse("components/cow_probe").unwrap(),
            COMPONENT.to_owned(),
        )]));
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
        let compiled=engine.compile_self_contained_named("ui/cow.rhai",r#"import "components/cow_probe" as p;fn view(ctx){row([p::Counter(#{key:"left"}).with_style(style().opacity(0.7).inset_start(offset_px(-3))),p::Counter(#{key:"right"}).with_style(style().opacity(0.9).inset_end(px(5)))])}"#).unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "root"),
            Some("main".into()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let frozen = lifecycle.root().unwrap().clone();
        let children = |node: &UiNode| match node.kind() {
            UiNodeKind::Box { children } => children.clone(),
            _ => panic!(),
        };
        let before = children(&frozen);
        let cb = before[0].event_handlers("click")[0]
            .handler()
            .as_script()
            .unwrap()
            .clone();
        lifecycle
            .invoke_callback_transactional(&engine, &cb, UiValue::Null)
            .unwrap();
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
        let current = children(lifecycle.root().unwrap());
        assert!(matches!(current[0].kind(),UiNodeKind::Text{text}if text=="1"));
        assert!(matches!(current[1].kind(),UiNodeKind::Text{text}if text=="0"));
        assert_eq!(
            current[0].style().base.width,
            Some(Length::Pixels(110.).into())
        );
        assert_eq!(current[0].style().base.opacity, Some(0.7));
        assert!(current[0].style().base.inset_start.is_some());
        assert!(current[0].style().base.inset_end.is_none());
        assert_eq!(
            current[1].style().base.width,
            Some(Length::Pixels(100.).into())
        );
        assert_eq!(current[1].style().base.opacity, Some(0.9));
        assert!(current[1].style().base.inset_end.is_some());
        let old = children(&frozen);
        assert!(matches!(old[0].kind(),UiNodeKind::Text{text}if text=="0"));
        assert_eq!(old[0].style().base.width, Some(Length::Pixels(100.).into()));
        assert_eq!(old[0].style().base.opacity, Some(0.7));
        println!(
            "formal child rerender updates owned width; both caller styles and saved prior tree remain isolated"
        );
    }
}
