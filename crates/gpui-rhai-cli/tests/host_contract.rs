use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::Command;

use gpui_rhai::{
    CapabilityCallKind, CapabilityContract, CapabilityDescriptor, CapabilityId, CapabilityMethod,
    ValueSchema,
};
use gpui_rhai_cli::Project;
use semver::Version;

fn contract() -> CapabilityContract {
    CapabilityContract::from_descriptors([(
        CapabilityDescriptor {
            id: CapabilityId::parse("app.files").unwrap(),
            version: Version::new(1, 0, 0),
            methods: BTreeMap::from([(
                "read".into(),
                CapabilityMethod {
                    input: ValueSchema::string(),
                    output: ValueSchema::string(),
                },
            )]),
        },
        BTreeSet::from([CapabilityCallKind::Sync]),
    )])
    .unwrap()
}

fn fixture(source: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\n",
    )
    .unwrap();
    Project::new(directory.path())
        .plan_init()
        .unwrap()
        .apply()
        .unwrap();
    fs::write(
        directory.path().join("ui/app.toml"),
        "entry = \"main\"\nruntime_api = 3\n[capabilities]\n\"app.files\" = \"^1\"\n",
    )
    .unwrap();
    fs::write(directory.path().join("ui/main.rhai"), source).unwrap();
    directory
}

#[test]
fn library_check_catches_unreachable_method_typo_and_reports_missing_contract() {
    let directory = fixture(
        r#"fn event(ctx) {
        if false { ctx.call_capability("app.files", "raed", "file"); }
    }
    fn view(ctx) { text("ready") }"#,
    );
    let project = Project::new(directory.path());
    let report = project.check().unwrap();
    assert_eq!(report.capability_calls.unchecked.len(), 1);
    assert!(report.summary().contains("no Host contract"));
    let error = project.check_with_host_contract(&contract()).unwrap_err();
    assert!(error.to_string().contains("app.files.raed"), "{error}");
}

#[test]
fn actual_cli_consumes_an_exported_contract_without_changing_the_project() {
    let source = r#"fn init(ctx) { ctx.call_capability("app.files", "read", "file"); }
        fn view(ctx) { text("ready") }"#;
    let directory = fixture(source);
    fs::write(
        directory.path().join("host-contract.json"),
        serde_json::to_string_pretty(&contract()).unwrap(),
    )
    .unwrap();
    let manifest = fs::read(directory.path().join("ui/app.toml")).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_gpui-rhai"))
        .arg("--root")
        .arg(directory.path())
        .args(["check", "--host-contract", "host-contract.json"])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let output = String::from_utf8(run.stdout).unwrap();
    assert!(
        output.contains("1 literal call(s) and 1 literal input(s) checked"),
        "{output}"
    );
    assert!(output.contains("not executed"), "{output}");
    assert_eq!(
        fs::read_to_string(directory.path().join("ui/main.rhai")).unwrap(),
        source
    );
    assert_eq!(
        fs::read(directory.path().join("ui/app.toml")).unwrap(),
        manifest
    );
}

#[test]
fn actual_cli_rejects_unsupported_contract_and_wrong_entry_point() {
    let directory = fixture(
        r#"fn done(ctx, value) {}
        fn init(ctx) { ctx.start_task("app.files", "read", "file", Fn("done"), Fn("done")); }
        fn view(ctx) { text("ready") }"#,
    );
    let project = Project::new(directory.path());
    assert!(
        project
            .check_with_host_contract(&contract())
            .unwrap_err()
            .to_string()
            .contains("Task entry point")
    );
    let mut invalid = contract();
    invalid.format_version += 1;
    fs::write(
        directory.path().join("host-contract.json"),
        serde_json::to_string(&invalid).unwrap(),
    )
    .unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_gpui-rhai"))
        .arg("--root")
        .arg(directory.path())
        .args(["check", "--host-contract", "host-contract.json"])
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("unsupported capability contract format")
    );
}
