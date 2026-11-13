use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;

use gpui_rhai::{
    AppManifest, AsyncCapabilityHandler, CapabilityCallKind, CapabilityContract,
    CapabilityContractError, CapabilityDescriptor, CapabilityHandler, CapabilityId,
    CapabilityMethod, CapabilityRegistry, HostFunction, ModuleId, ObjectField, RuntimeEngine,
    SubscriptionCapabilityHandler, SubscriptionWork, TaskWork, UiValue, ValueSchema,
};
use semver::{Version, VersionReq};

struct NeverRun;
impl CapabilityHandler for NeverRun {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        panic!("preflight must not call a service")
    }
}
impl AsyncCapabilityHandler for NeverRun {
    fn start(&mut self, _: &str, _: UiValue) -> Result<TaskWork, String> {
        panic!("preflight must not start work")
    }
}
impl SubscriptionCapabilityHandler for NeverRun {
    fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
        panic!("preflight must not subscribe")
    }
}

fn descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::parse("app.files").unwrap(),
        version: Version::new(1, 2, 0),
        methods: BTreeMap::from([(
            "read".into(),
            CapabilityMethod {
                input: ValueSchema::Object {
                    fields: BTreeMap::from([(
                        "path".into(),
                        ObjectField::required(ValueSchema::string()).with_doc("file name"),
                    )]),
                    allow_unknown: false,
                },
                output: ValueSchema::string(),
            },
        )]),
    }
}

fn registry() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    registry.register(descriptor(), NeverRun).unwrap();
    registry.register_async(descriptor(), NeverRun).unwrap();
    registry
        .register_subscription(descriptor(), NeverRun)
        .unwrap();
    registry
}

fn manifest() -> AppManifest {
    AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.files", "^1")
        .unwrap()
}

fn preflight(source: &str, contract: Option<&CapabilityContract>) -> gpui_rhai::ScriptPreflight {
    RuntimeEngine::new()
        .preflight_calls(
            "main.rhai",
            source,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &manifest(),
            contract,
        )
        .unwrap()
}

#[test]
fn export_and_json_roundtrip_do_not_execute_or_activate_handlers() {
    let mut registry = registry();
    let contract = registry.export_contract().unwrap();
    let json = serde_json::to_string_pretty(&contract).unwrap();
    assert!(!json.contains("NeverRun"));
    assert_eq!(contract, CapabilityContract::from_json(&json).unwrap());
    assert_eq!(
        contract.capabilities.values().next().unwrap().modes,
        BTreeSet::from([
            CapabilityCallKind::Sync,
            CapabilityCallKind::Task,
            CapabilityCallKind::Subscription
        ])
    );
    // Export is no grant: an undeclared call fails before reaching NeverRun.
    assert!(
        registry
            .call(&descriptor().id, "read", UiValue::Null)
            .is_err()
    );
}

#[test]
fn descriptors_alone_export_without_constructing_services() {
    let contract = CapabilityContract::from_descriptors([(
        descriptor(),
        BTreeSet::from([CapabilityCallKind::Sync]),
    )])
    .unwrap();
    contract.validate_manifest(&manifest()).unwrap();
    assert!(
        CapabilityContract::from_descriptors([
            (descriptor(), BTreeSet::from([CapabilityCallKind::Sync])),
            (descriptor(), BTreeSet::from([CapabilityCallKind::Sync])),
        ])
        .is_err()
    );
    assert!(CapabilityContract::from_descriptors([(descriptor(), BTreeSet::new())]).is_err());
}

#[test]
fn identity_ignores_docs_and_sdk_version_but_not_actual_schema_fields() {
    let contract = registry().export_contract().unwrap();
    let mut changed = contract.clone();
    changed.sdk_version = "99.0.0".into();
    let schema = &mut changed
        .capabilities
        .get_mut(&descriptor().id)
        .unwrap()
        .methods
        .get_mut("read")
        .unwrap()
        .input;
    let ValueSchema::Object { fields, .. } = schema else {
        unreachable!()
    };
    fields.get_mut("path").unwrap().doc = Some("new documentation".into());
    assert_eq!(
        contract.fingerprint().unwrap(),
        changed.fingerprint().unwrap()
    );
    registry().validate_contract(&changed).unwrap();
    let ValueSchema::Object { fields, .. } = &mut changed
        .capabilities
        .get_mut(&descriptor().id)
        .unwrap()
        .methods
        .get_mut("read")
        .unwrap()
        .input
    else {
        unreachable!()
    };
    // A business field named doc is not documentation metadata.
    fields.insert("doc".into(), ObjectField::optional(ValueSchema::Bool));
    assert_ne!(
        contract.fingerprint().unwrap(),
        changed.fingerprint().unwrap()
    );
    assert!(matches!(
        registry().validate_contract(&changed),
        Err(CapabilityContractError::Stale { .. })
    ));
}

#[test]
fn malformed_contracts_and_incompatible_requirements_fail_before_analysis() {
    let contract = registry().export_contract().unwrap();
    let mut changed = contract.clone();
    changed.format_version += 1;
    assert!(CapabilityContract::from_json(&serde_json::to_string(&changed).unwrap()).is_err());
    changed = contract.clone();
    changed.runtime_api += 1;
    assert!(changed.validate().is_err());
    changed = contract.clone();
    changed
        .capabilities
        .get_mut(&descriptor().id)
        .unwrap()
        .methods
        .get_mut("read")
        .unwrap()
        .input = ValueSchema::bounded_integer(Some(10), Some(1));
    assert!(changed.validate().is_err());
    let app = manifest().with_capability("app.files", "^2").unwrap();
    assert!(
        contract
            .validate_manifest(&app)
            .unwrap_err()
            .to_string()
            .contains("requires ^2")
    );
    assert!(CapabilityContract::from_json(&" ".repeat(4 * 1024 * 1024 + 1)).is_err());
    let mut reader = std::io::repeat(b' ').take(8 * 1024 * 1024);
    assert!(CapabilityContract::from_reader(&mut reader).is_err());
    assert_eq!(
        reader.limit(),
        4 * 1024 * 1024 - 1,
        "read limit applies before allocating the entire input"
    );
}

#[test]
fn preflight_checks_all_entry_points_without_executing_unreachable_script() {
    let contract = registry().export_contract().unwrap();
    let report = preflight(
        r#"
        throw "must not execute top-level code";
        fn done(ctx, value) {}
        fn event(ctx) {
            if false {
                ctx.call_capability("app.files", "read", #{path: "a"});
                ctx.start_task("app.files", "read", #{path: "b"}, Fn("done"), Fn("done"));
                ctx.start_subscription("app.files", "read", #{path: "c"}, Fn("done"), Fn("done"), #{});
                call_capability(ctx, "app.files", "read", #{path: "d"});
            }
        }
    "#,
        Some(&contract),
    );
    assert!(report.known_calls.is_empty(), "{:?}", report.known_calls);
    assert!(
        report.capabilities.diagnostics.is_empty(),
        "{:?}",
        report.capabilities.diagnostics
    );
    assert_eq!(report.capabilities.checked_calls, 4);
    assert_eq!(report.capabilities.checked_inputs, 4);
    assert!(
        report.capabilities.unchecked.is_empty(),
        "{:?}",
        report.capabilities.unchecked
    );
}

#[test]
fn typos_in_modules_and_literal_payload_mismatches_are_reported_without_payloads() {
    let contract = registry().export_contract().unwrap();
    let source = r#"fn event(ctx) {
        if false { ctx.call_capability("app.files", "raed", ()); }
        ctx.call_capability("app.files", "read", #{path: 123, secret: "NEVER_PRINT_THIS"});
    }"#;
    let report = RuntimeEngine::new()
        .preflight_calls(
            "entry",
            "fn view(ctx) { text(\"ok\") }",
            &BTreeMap::from([(ModuleId::parse("helper").unwrap(), source.into())]),
            &BTreeSet::new(),
            &manifest(),
            Some(&contract),
        )
        .unwrap();
    assert_eq!(report.capabilities.diagnostics.len(), 2);
    let diagnostic = report
        .capabilities
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(diagnostic.contains("app.files.raed"));
    assert!(diagnostic.contains("helper:2:"));
    assert!(!diagnostic.contains("NEVER_PRINT_THIS"));
}

#[test]
fn wrong_mode_undeclared_id_and_unknown_contract_id_are_rejected() {
    let contract = CapabilityContract::from_descriptors([(
        descriptor(),
        BTreeSet::from([CapabilityCallKind::Sync]),
    )])
    .unwrap();
    let report = preflight(
        r#"fn event(ctx) {
        ctx.start_task("app.files", "read", (), Fn("done"), Fn("done"));
        ctx.call_capability("app.other", "read", ());
    }"#,
        Some(&contract),
    );
    assert_eq!(report.capabilities.diagnostics.len(), 2);
    assert!(
        report.capabilities.diagnostics[0]
            .message
            .contains("Task entry point")
    );
    assert!(
        report.capabilities.diagnostics[1]
            .message
            .contains("not declared")
    );
    let empty = CapabilityRegistry::new().export_contract().unwrap();
    assert!(
        RuntimeEngine::new()
            .preflight_calls(
                "entry",
                "",
                &BTreeMap::new(),
                &BTreeSet::new(),
                &manifest(),
                Some(&empty)
            )
            .is_err()
    );
}

#[test]
fn missing_contract_and_dynamic_names_remain_explicitly_unverified() {
    let report = preflight(
        r#"fn event(ctx, name, input) {
        ctx.call_capability("app.files", "read", input);
        ctx.call_capability(name, "read", input);
    }"#,
        None,
    );
    assert_eq!(report.capabilities.checked_calls, 0);
    assert_eq!(report.capabilities.unchecked.len(), 2);
    assert!(
        report.capabilities.diagnostics.is_empty(),
        "{:?}",
        report.capabilities.diagnostics
    );
    let contract = registry().export_contract().unwrap();
    let report = preflight(
        r#"fn event(ctx, input) { ctx.call_capability("app.files", "read", input); }"#,
        Some(&contract),
    );
    assert_eq!(report.capabilities.checked_calls, 1);
    assert_eq!(report.capabilities.checked_inputs, 0);
}

#[test]
fn script_and_host_overloads_are_not_misdiagnosed_as_capability_calls() {
    let contract = registry().export_contract().unwrap();
    let source = r#"fn call_capability(id, method, input) { 42 }
        fn event(ctx) { ctx.call_capability("other.api", "fine", ()); }"#;
    let report = preflight(source, Some(&contract));
    assert!(
        report.capabilities.diagnostics.is_empty(),
        "{:?}",
        report.capabilities.diagnostics
    );
    assert_eq!(report.capabilities.unchecked.len(), 1);
    let report = RuntimeEngine::new()
        .preflight_calls(
            "entry",
            r#"fn event(ctx) { ctx.call_capability("other.api", "fine", ()); }"#,
            &BTreeMap::new(),
            &BTreeSet::from([HostFunction::parse("call_capability/4").unwrap()]),
            &manifest(),
            Some(&contract),
        )
        .unwrap();
    assert!(
        report.capabilities.diagnostics.is_empty(),
        "{:?}",
        report.capabilities.diagnostics
    );
    assert_eq!(report.capabilities.unchecked.len(), 1);
}

#[test]
fn contract_expectation_is_rechecked_atomically_and_never_grants_revoked_access() {
    let mut registry = registry();
    let contract = registry.export_contract().unwrap();
    registry.require_contract(contract.clone()).unwrap();
    registry.activate(&manifest().capabilities).unwrap();
    // Even after a passing preflight, revocation still happens at runtime.
    preflight(
        r#"fn event(ctx) { ctx.call_capability("app.files", "read", #{path: "a"}); }"#,
        Some(&contract),
    );
    registry.activate(&BTreeMap::new()).unwrap();
    assert!(
        registry
            .call(&descriptor().id, "read", UiValue::Null)
            .is_err()
    );
    let mut extra = descriptor();
    extra.id = CapabilityId::parse("app.more").unwrap();
    registry.register(extra, NeverRun).unwrap();
    assert!(
        registry
            .activate(&manifest().capabilities)
            .unwrap_err()
            .to_string()
            .contains("stale Host")
    );
    // Failed activation did not grant partial access.
    assert!(
        registry
            .call(&descriptor().id, "read", UiValue::Null)
            .is_err()
    );
    let changed = registry.export_contract().unwrap();
    registry.require_contract(changed).unwrap();
    registry
        .activate(&BTreeMap::from([(descriptor().id, VersionReq::STAR)]))
        .unwrap();
    // No service call: invalid input is still rejected by the real registry.
    assert!(
        registry
            .call(&descriptor().id, "read", UiValue::Null)
            .is_err()
    );
}

struct CheckedHost(CapabilityContract);
impl gpui_rhai::ScriptViewExtension for CheckedHost {
    fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
        runtime
            .capabilities
            .register(descriptor(), NeverRun)
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .require_contract(self.0.clone())
            .map_err(|error| error.to_string())
    }
}

#[test]
fn real_host_preparation_rejects_a_stale_contract_without_initializing_script() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("main.rhai"), "fn init(ctx) { throw \"do not run init in preparation\"; } fn view(ctx) { text(\"ready\") }").unwrap();
    std::fs::write(
        directory.path().join("theme.rhai"),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("app.toml"),
        "entry = \"main\"\nruntime_api = 3\n[capabilities]\n\"app.files\" = \"^1\"\n",
    )
    .unwrap();
    let matching = CapabilityContract::from_descriptors([(
        descriptor(),
        BTreeSet::from([CapabilityCallKind::Sync]),
    )])
    .unwrap();
    assert!(
        gpui_rhai::ScriptView::files(directory.path().join("main.rhai"))
            .extension(CheckedHost(matching))
            .prepare()
            .is_ok()
    );
    let stale = registry().export_contract().unwrap();
    let error = gpui_rhai::ScriptView::files(directory.path().join("main.rhai"))
        .extension(CheckedHost(stale))
        .prepare()
        .err()
        .expect("stale modes must fail preparation");
    assert!(
        error.to_string().contains("stale Host capability contract"),
        "{error}"
    );
}

#[test]
fn a_stale_tooling_contract_cannot_block_partial_or_complete_revocation() {
    let mut registry = registry();
    let mut second = descriptor();
    second.id = CapabilityId::parse("app.second").unwrap();
    registry.register(second.clone(), NeverRun).unwrap();
    registry
        .require_contract(registry.export_contract().unwrap())
        .unwrap();
    registry
        .activate(&BTreeMap::from([
            (descriptor().id, VersionReq::STAR),
            (second.id.clone(), VersionReq::STAR),
        ]))
        .unwrap();
    let mut added = descriptor();
    added.id = CapabilityId::parse("app.extra").unwrap();
    registry.register(added, NeverRun).unwrap();
    registry.activate(&manifest().capabilities).unwrap();
    assert!(registry.call(&second.id, "read", UiValue::Null).is_err());
    registry.activate(&BTreeMap::new()).unwrap();
    assert!(
        registry
            .call(&descriptor().id, "read", UiValue::Null)
            .is_err()
    );
    assert!(
        registry.activate(&manifest().capabilities).is_err(),
        "a new grant still checks the stale contract"
    );
}

#[test]
fn failed_contract_replacement_retains_the_last_good_expectation() {
    let mut registry = registry();
    let mut contract = registry.export_contract().unwrap();
    registry.require_contract(contract.clone()).unwrap();
    contract
        .capabilities
        .get_mut(&descriptor().id)
        .unwrap()
        .version
        .major += 1;
    assert!(registry.require_contract(contract).is_err());
    registry.activate(&manifest().capabilities).unwrap();
}

#[test]
fn dynamic_method_names_do_not_hide_a_definitely_undeclared_literal_id() {
    let report = preflight(
        r#"fn event(ctx, method) {
        ctx.call_capability("app.undeclared", method, ());
        ctx.call_capability("app.files", method, ());
    }"#,
        None,
    );
    assert_eq!(report.capabilities.diagnostics.len(), 1);
    assert!(
        report.capabilities.diagnostics[0]
            .message
            .contains("not declared")
    );
    assert_eq!(report.capabilities.unchecked.len(), 1);
}

#[test]
fn a_valid_contract_still_calls_the_real_handler_only_after_activation() {
    use std::{cell::Cell, rc::Rc};
    struct Echo(Rc<Cell<usize>>);
    impl CapabilityHandler for Echo {
        fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
            self.0.set(self.0.get() + 1);
            Ok(input)
        }
    }
    let mut descriptor = descriptor();
    descriptor.methods.get_mut("read").unwrap().input = ValueSchema::string();
    let calls = Rc::new(Cell::new(0));
    let mut registry = CapabilityRegistry::new();
    registry
        .register(descriptor.clone(), Echo(Rc::clone(&calls)))
        .unwrap();
    let contract = registry.export_contract().unwrap();
    let report = preflight(
        r#"fn event(ctx) { ctx.call_capability("app.files", "read", "ok"); }"#,
        Some(&contract),
    );
    assert_eq!(report.capabilities.checked_inputs, 1);
    registry.require_contract(contract).unwrap();
    assert!(
        registry
            .call(&descriptor.id, "read", UiValue::String("ok".into()))
            .is_err()
    );
    assert_eq!(calls.get(), 0);
    registry.activate(&manifest().capabilities).unwrap();
    assert_eq!(
        registry
            .call(&descriptor.id, "read", UiValue::String("ok".into()))
            .unwrap(),
        UiValue::String("ok".into())
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn schema_exports_do_not_copy_default_business_values_or_resource_handles() {
    let mut descriptor = descriptor();
    let ValueSchema::Object { fields, .. } = &mut descriptor.methods.get_mut("read").unwrap().input
    else {
        unreachable!()
    };
    fields.get_mut("path").unwrap().default = Some(UiValue::String("APP_SECRET_DEFAULT".into()));
    fields.insert(
        "resource".into(),
        ObjectField::optional(ValueSchema::Handle {
            kind: "resource".into(),
        })
        .with_default(UiValue::Handle(gpui_rhai::OpaqueHandle::new(
            "resource", 90_348_238,
        ))),
    );
    let mut registry = CapabilityRegistry::new();
    registry.register(descriptor, NeverRun).unwrap();
    let contract = registry.export_contract().unwrap();
    let json = serde_json::to_string(&contract).unwrap();
    assert!(!json.contains("APP_SECRET_DEFAULT"));
    assert!(!json.contains("90348238"));
    // Neither export nor validation turns a default into a supplied input.
    let schema =
        &contract.capabilities[&CapabilityId::parse("app.files").unwrap()].methods["read"].input;
    assert!(
        schema
            .validate(&rhai::Dynamic::from_map(rhai::Map::new()))
            .is_err()
    );
    registry.validate_contract(&contract).unwrap();
}
