mod support;

use std::process::Command;

#[test]
fn fake_cli_emits_stdout_stderr_and_exit_code() {
    let fake = support::FakeCli::new("fake-cli", "hello out", "hello err", 7).unwrap();
    let _guard = fake.prepend_to_path().unwrap();

    let output = Command::new(fake.command()).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello out\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "hello err\n");
    assert_eq!(output.status.code(), Some(7));
}
