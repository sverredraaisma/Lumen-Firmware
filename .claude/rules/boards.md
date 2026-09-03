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
  bug to fix. `src/boards.rs` reads the file and `cargo test` discovers this
  directory, so a new definition is validated without anyone editing code.
- The filename must match `board.name`. The name becomes a build-matrix entry and
  a published artefact, and the two disagreeing leaves nobody able to say what a
  board is called.
- **A definition with no way to confirm a pairing is refused**: no LED output, no
  button, no indicator and no printed code means nobody can approve a pairing at
  the device, and pairing is physical precisely so that reaching the network is
  not enough. An LED output is sufficient on its own — the device can blink the
  strip.
- Every refusal names the field and says what the rule is for. A board file is a
  contributor's first change here, and the diagnostic is the whole of their
  experience of it.
