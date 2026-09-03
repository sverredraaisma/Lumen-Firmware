//! Run the actual binary.
//!
//! `main` is one line, but it is the one line that has to work: if the shell
//! cannot start, nothing else in this repo matters. Spawning the real
//! executable also means `main` is covered rather than being the permanent
//! uncovered remainder that a `--fail-under-lines` gate then has to be lowered
//! to accommodate.

use std::process::Command;

#[test]
fn the_binary_starts_and_prints_its_banner() {
    let out = Command::new(env!("CARGO_BIN_EXE_lumen-firmware"))
        .output()
        .expect("the firmware host stub should be runnable");

    assert!(out.status.success(), "exit status: {:?}", out.status);

    let stdout = String::from_utf8(out.stdout).expect("banner must be UTF-8");
    assert_eq!(
        stdout.trim_end(),
        lumen_firmware::banner(),
        "the binary must print exactly the library's banner, so the two cannot drift"
    );
    assert!(
        out.stderr.is_empty(),
        "a clean start writes nothing to stderr, got: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}
