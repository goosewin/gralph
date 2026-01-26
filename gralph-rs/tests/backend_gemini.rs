use gralph_rs::backend::gemini::GeminiBackend;

#[test]
#[ignore]
fn gemini_cli_smoke() {
    let backend = GeminiBackend::new();
    assert!(backend.check_installed());
}
