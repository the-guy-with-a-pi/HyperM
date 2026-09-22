#[test]
fn binary_help_is_available() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hyperm"))
        .arg("--help")
        .output()
        .expect("binary should run");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Firecracker"));
}
