use gralph_rs::backend::codex::CodexBackend;

#[test]
#[ignore]
fn codex_cli_smoke() {
    let backend = CodexBackend::new();
    assert!(backend.check_installed());
}
