//! Firmware entry point — skeleton (W9 fills this in).
//!
//! This binary is the *shell*: it owns the sockets, the timers, the flash and
//! the LED peripheral, implements the `lumen-hal` traits over `esp-idf-hal` /
//! `esp-idf-svc`, and hands events to the sans-IO core in `lumen-device`.
//!
//! It builds on the host today so that CI has something to compile before the
//! ESP-IDF toolchain is wired up. The esp-idf dependency and the target
//! configuration land with W9.

fn main() {
    println!(
        "lumen-firmware {} — host stub; no HAL implementation yet",
        env!("CARGO_PKG_VERSION")
    );
}
