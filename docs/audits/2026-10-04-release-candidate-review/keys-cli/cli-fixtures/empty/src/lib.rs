#![deny(warnings)]
pub mod gpui_rhai_embedded;
#[test]
fn manifest_contract(){let manifest=gpui_rhai_embedded::app_manifest();assert_eq!(manifest.capabilities.len(),0);assert_eq!(manifest.entry.as_str(),"main");}
