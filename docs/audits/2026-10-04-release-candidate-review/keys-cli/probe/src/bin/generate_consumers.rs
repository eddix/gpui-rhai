use std::{fs,path::PathBuf};
use gpui_rhai_cli::Project;

fn main(){
    let base=PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("cli-fixtures");
    for (name,count,capabilities) in [("empty",0,""),("nonempty",2,"\"app.alpha\" = \"^1.0\"\n\"app.beta\" = \"*\"\n")] {
        let root=base.join(name);fs::create_dir_all(root.join("src")).unwrap();
        let cargo=format!("[package]\nname=\"audit-rc-embed-{name}\"\nversion=\"0.0.0\"\nedition=\"2024\"\npublish=false\n[workspace]\n[dependencies]\ngpui-rhai={{path=\"../../../../../../crates/gpui-rhai\",features=[\"charts\"]}}\ngpui={{package=\"gpui-pre\",version=\"=0.3.7\",default-features=false,features=[\"font-kit\",\"test-support\"]}}\n");
        fs::write(root.join("Cargo.toml"),cargo).unwrap();
        let owner="fn main(){println!(\"owned-host-sentinel\");}\n";fs::write(root.join("src/main.rs"),owner).unwrap();
        let project=Project::new(&root);project.plan_init().unwrap().apply().unwrap();
        fs::write(root.join("ui/app.toml"),format!("entry=\"main\"\nruntime_api=2\n[capabilities]\n{capabilities}")).unwrap();
        project.plan_embed().unwrap().apply().unwrap();
        let generated=fs::read_to_string(root.join("src/gpui_rhai_embedded.rs")).unwrap();
        assert!(!generated.contains("let mut manifest"));assert!(!generated.contains("allow(unused_mut)"));
        assert_eq!(generated.matches("let manifest = manifest.with_capability(").count(),count);
        assert_eq!(fs::read_to_string(root.join("src/main.rs")).unwrap(),owner);
        fs::write(root.join("src/lib.rs"),format!("#![deny(warnings)]\npub mod gpui_rhai_embedded;\n#[test]\nfn manifest_contract(){{let manifest=gpui_rhai_embedded::app_manifest();assert_eq!(manifest.capabilities.len(),{count});assert_eq!(manifest.entry.as_str(),\"main\");}}\n")).unwrap();
        fs::copy(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"),root.join("Cargo.lock")).unwrap();
        println!("generated {name}: {count} capabilities; immutable manifest; owned main preserved");
    }
}
