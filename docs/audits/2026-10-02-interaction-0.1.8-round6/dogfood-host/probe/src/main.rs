#![allow(dead_code,unused_imports)]
mod engine;
mod capability;
fn main(){
    let paths=capability::audit_path_roundtrip();
    let cancellation=capability::audit_cancel_publication();
    assert!(paths && cancellation,"dogfood Host invariants failed (see observations above)");
}
