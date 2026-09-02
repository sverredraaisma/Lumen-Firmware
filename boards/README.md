# Board definitions

**This is the intended first contribution.** Adding support for your own board
is a single file here — no understanding of the internals required, and a
genuinely useful change.

One TOML file per board, describing the chip, the pins, the LED outputs, the
power limits and which Cargo features the build variant enables. Pins and LED
counts are *runtime* configuration; features are build-time. CI builds every
published board and feature variant, and the prebuilt matrix is what means most
users never compile anything.

Prefer RISC-V parts (C3, C6) where the choice is free: they use upstream Rust
with no forked toolchain, which removes a whole class of setup friction — and
contributor friction is a tax an open-source project pays forever.

See `example-esp32c6-generic.toml` and copy it.
