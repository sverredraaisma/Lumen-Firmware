# lumen-firmware

The binary flashed to a device. It is the **shell** around the sans-IO core in
`lumen-device`: sockets, timers, flash, LED peripheral, ESP-IDF HAL
implementation — and nothing else.

- **Licence:** GPL-3.0
- **Main branch:** `main`
- **Status:** host stub, so CI has something to compile. The ESP-IDF HAL lands
  with W9.

## Stack

- Rust 1.85+, edition 2021
- Target: Rust on ESP-IDF via `esp-idf-hal` / `esp-idf-svc` (not yet wired up)
- **Prefer RISC-V parts (C3, C6)** — upstream Rust targets, no forked toolchain.
  Xtensa (ESP32-S3) needs `espup`.
- ESP-IDF gives `std` and FreeRTOS threads. The core is sans-IO, so threads vs
  async is purely a shell decision here, and threads are simpler.

## Commands

```bash
cargo build                                  # host stub today
cargo test                                   # host-side shell logic
cargo clippy --all-targets                   # CI runs with -D warnings
cargo fmt --all -- --check
cargo llvm-cov --summary-only                # coverage; must be >= 95%
```

## Hard rules

- **All logic belongs in `lumen-device`, not here.** This crate translates between
  hardware and events. If you are writing a decision here — when to re-elect, how
  to resolve a source stack — it is in the wrong repo, and it becomes untestable
  the moment it lands.
- **Coverage floor is 95%** on the host-testable shell. Anything that genuinely
  cannot be tested on the host must be a thin adapter over something that can:
  keep the untestable surface small enough that it is obviously correct by
  reading.
- **Adding a board must not require editing a `.rs` file.** One TOML file in
  `boards/`. That is the intended first contribution, and it stays that way.
- **Pins and LED counts are runtime config; only optional capabilities are Cargo
  features.** Each feature doubles a dimension of the CI build matrix.
- **Zigbee is reached by FFI from the bridge shell, never from the core.**
- **A device is never dark because of software.** Every failure path here needs a
  defined visual outcome — a corrupt program falls back, a lost network keeps
  rendering the last program.

## Gotchas

> Living section. Add anything that cost real time.

- **The "cannot link / no local coverage" note used to be wrong; both now work.**
  `link.exe` was never missing. What was missing was the **Windows SDK**, so the
  linker had no `kernel32.lib` to link against and Rust reported that as
  "linker `link.exe` not found". Adding the SDK component to the existing VS
  2022 install fixed the MSVC toolchain and `cargo llvm-cov` together. If a
  fresh machine shows this symptom, install the C++ workload rather than
  switching to `windows-gnu`: that workaround builds, which is why nobody
  revisits it, and it silently costs you coverage because the `windows-gnu`
  toolchain ships no profiler runtime.
- **`esp-zigbee-sdk` is C with vendor binaries.** Confirm it can still be driven
  from Rust before promising anything Zigbee — vendor SDK situations change, and
  secondhand accounts of them go stale.
- **On-device compilation is unproven.** `caps=compile` is only real if a
  representative effect compiles inside a few hundred KB. Measure before relying
  on it.

## Specialized guides (loaded on demand — do not preload)

- Board definition conventions: `.claude/rules/boards.md` (auto-loads on `boards/*.toml`)
- Design notes: `docs/firmware.md`
- Project-wide rules and the licence boundary: `CONTRIBUTING.md`

## Compact instructions

Preserve code changes, file paths touched, decisions made, and any measured
number (timing, RAM, frame rate). Drop raw build and flash output.
