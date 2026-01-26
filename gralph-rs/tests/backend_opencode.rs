use gralph_rs::backend::opencode::OpenCodeBackend;
use gralph_rs::backend::Backend;

#[test]
#[ignore]
fn opencode_cli_smoke() {
    let backend = OpenCodeBackend::new();
    assert!(backend.check_installed());
}
