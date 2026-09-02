# lumen-firmware

The binary flashed to a device. It is the **shell** around the sans-IO core in
`lumen-device`: sockets, timers, flash, the LED peripheral and the ESP-IDF HAL
implementation live here, and nothing else does.

**GPL-3.0**, like everything on the device side of the licence boundary.

## Adding your board is one file

`boards/` holds one TOML file per supported board. That is the intended first
contribution to this project — copy `boards/example-esp32c6-generic.toml`, edit
it, open a PR. No internals knowledge required, and hardware coverage grows in
the same motion as the contributor base.

Pins and LED counts are **runtime** configuration; only genuinely optional
capabilities are build variants. CI builds every published board × feature
combination, and that prebuilt matrix is why most users never compile anything.

## Toolchain

Rust on ESP-IDF via `esp-idf-hal` / `esp-idf-svc` — Rust over Espressif's C SDK
rather than replacing it, which buys mature WiFi, BLE, mDNS, NVS and OTA. The
core is sans-IO, so threads versus async is purely a shell decision here, and
threads are simpler.

- **RISC-V parts (C3, C6): upstream Rust targets, nothing extra to install.**
  Prefer them wherever the choice is free.
- Xtensa parts (ESP32-S3): `espup` for the forked toolchain.

Today the binary is a host stub so CI has something to compile. The ESP-IDF
dependency, the target configuration and the HAL implementation land with W9.

## Zigbee

`esp-zigbee-sdk` is C with vendor binaries, reached by FFI from the bridge shell
and never from the core. Confirm it can still be driven from Rust before
promising Zigbee bridges — vendor SDK situations change.
