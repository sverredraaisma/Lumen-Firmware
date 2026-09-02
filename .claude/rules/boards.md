---
paths:
  - "boards/*.toml"
---

# Board definitions

One file per board, and it is meant to be contributable by someone who has read
nothing else in this repo. Keep the schema boring.

- **Pins and LED counts are runtime configuration, not build variants.** Only
  genuinely optional capabilities become Cargo features, because every feature
  doubles a dimension of the CI build matrix.
- `max_current_ma` is real: the runtime derates against it rather than browning
  out the board.
- Prefer RISC-V parts (C3, C6) in examples and defaults. They build on upstream
  Rust with no forked toolchain, and setup friction is a tax the project pays
  forever.
- Adding a board must not require touching any `.rs` file. If it does, that is the
  bug to fix.
