use gralph_rs::backend::codex::CodexBackend;
use gralph_rs::backend::Backend;

#[test]
#[ignore]
fn codex_cli_smoke() {
    let backend = CodexBackend::new();
    assert!(backend.check_installed());
}
