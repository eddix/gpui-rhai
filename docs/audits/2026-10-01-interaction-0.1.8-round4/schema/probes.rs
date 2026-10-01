//! Bounded public API differential checks; no product instrumentation.
use std::collections::BTreeMap;
use gpui_rhai::{AsyncScope, ObjectField, OpaqueHandle, RuntimeEngine,
    SubscriptionRegistration, SubscriptionRegistry, UiValue, ValueSchema};

#[test]
fn online_subscription_and_offline_schema_acceptance_agree() {
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile("fn view(){text(\"audit\")} fn success(ctx,v){v} fn failure(ctx,v){v}").unwrap();
    engine.render(&compiled).unwrap();
    let success = engine.callback(&compiled, "success").unwrap();
    let failure = engine.callback(&compiled, "failure").unwrap();
    let mut values = vec![UiValue::Null, UiValue::Bool(false), UiValue::Bool(true),
        UiValue::Integer(i64::MIN), UiValue::Integer(-1), UiValue::Integer(0),
        UiValue::Integer(1), UiValue::Integer(i64::MAX), UiValue::Float(-0.0),
        UiValue::Float(-1.0), UiValue::Float(0.5), UiValue::Float(1.0),
        UiValue::Float(f64::NAN), UiValue::Float(f64::INFINITY),
        UiValue::String(String::new()), UiValue::String("a".into()),
        UiValue::String("中文".into()), UiValue::Handle(OpaqueHandle::new("test", 1)),
        UiValue::Handle(OpaqueHandle::new("other", 2)), UiValue::Array(vec![]),
        UiValue::Map(BTreeMap::new())];
    for value in values.clone() {
        values.push(UiValue::Array(vec![value.clone()]));
        values.push(UiValue::Map(BTreeMap::from([("x".into(), value)])));
    }
    values.push(UiValue::Array(vec![UiValue::Integer(1),UiValue::Integer(2)]));
    values.push(UiValue::Map(BTreeMap::from([("y".into(),UiValue::Integer(1))])));
    let mut schemas = vec![ValueSchema::Null, ValueSchema::Bool, ValueSchema::integer(),
        ValueSchema::bounded_integer(Some(0), Some(1)), ValueSchema::float(),
        ValueSchema::bounded_float(Some(-1.0),Some(1.0)),ValueSchema::number(),
        ValueSchema::positive_number(),ValueSchema::string(),ValueSchema::enumeration(["a","中文"]),
        ValueSchema::UiValue,ValueSchema::Handle{kind:"test".into()},
        ValueSchema::Node,ValueSchema::Callback,ValueSchema::Style,ValueSchema::Length,
        ValueSchema::Asset,ValueSchema::Signal,ValueSchema::Collection,
        ValueSchema::Document,ValueSchema::ChartData,ValueSchema::Ref];
    for schema in schemas.clone() {
        schemas.push(ValueSchema::optional(schema.clone()));
        schemas.push(ValueSchema::Array{items:Box::new(schema.clone()), max_items:Some(1)});
        schemas.push(ValueSchema::Map{values:Box::new(schema.clone())});
        schemas.push(ValueSchema::one_of([ValueSchema::Bool,schema.clone()]));
        for required in [false,true] { for allow_unknown in [false,true] {
            schemas.push(ValueSchema::Object{fields:BTreeMap::from([("x".into(),
                ObjectField{schema:schema.clone(),required,sensitive:false,default:None})]),allow_unknown});
        }}
    }
    let mut cases = 0;
    let mut mismatches = vec![];
    for (si,schema) in schemas.iter().enumerate() {
        schema.validate_definition().unwrap();
        for (vi,value) in values.iter().enumerate() {
            let expected = schema.validate_ui_value(value).is_ok();
            let mut registry = SubscriptionRegistry::new();
            let (_,emitter) = registry.subscribe(SubscriptionRegistration::new("schema-parity",
                AsyncScope::App, compiled.generation(),success.clone(),failure.clone(),schema.clone()));
            emitter.emit(value.clone()).unwrap();
            let deliveries = registry.drain(compiled.generation());
            assert_eq!(deliveries.len(),1);
            let accepted = deliveries[0].callback.name() == "success";
            if accepted != expected {mismatches.push(format!("schema={si} value={vi} expected={expected} online={accepted} {schema:?} {value:?}"));}
            cases += 1;
        }
    }
    println!("SCHEMA_PARITY schemas={} values={} cases={cases} mismatches={}",schemas.len(),values.len(),mismatches.len());
    assert!(mismatches.is_empty(),"{}",mismatches.join("\n"));
}

#[test]
fn issue_93_recursive_default_contract_and_error_characterization() {
    let plain = r#"fn state_schema(){#{fields:#{current_theme:#{schema:#{type:"map",values:#{type:"ui_value"}},"default":#{type:"map",value:#{family:"Default",name:"Dark"}}}}}} fn view(ctx){text("x")}"#;
    let typed = plain.replace("family:\"Default\",name:\"Dark\"", "family:#{type:\"string\",value:\"Default\"},name:#{type:\"string\",value:\"Dark\"}");
    for (label,source,expect_ok) in [("plain",plain,false),("recursive",typed.as_str(),true)] {
        let mut engine=RuntimeEngine::new();
        let result=engine.compile(source).and_then(|compiled|engine.root_state_schema(&compiled));
        println!("ISSUE93 {label} ok={} error={:?}",result.is_ok(),result.as_ref().err().map(ToString::to_string));
        assert_eq!(result.is_ok(),expect_ok);
    }
}
