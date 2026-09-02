Flashed to every device; makes the device a first-class participant in the mesh. Targets the ESP32 family (S3, C3, C6) natively, plus smaller MCUs behind a bridge.

## A device is not necessarily a light

The firmware makes **no assumption that a device has LEDs, or what shape they are in**. A device is a mesh participant defined entirely by its capabilities. `render` is one optional capability among others, not the baseline.

| Archetype | Typical caps | Notes |
|---|---|---|
| Linear strip | `render` | the common case; `u` is index along the strip |
| Matrix / panel | `render` | 2D lattice, geometry usually declared rather than scanned |
| Ring, tube, sculpture | `render`, `mapped` | irregular; this is where the AR mapping in [[App]] earns its place |
| Standalone lamp / fixture | `render`, `imu` | few LEDs, a stand, often a button — gravity snapping applies |
| **Audio node** | `audio` — no `render` | mic or line-in, publishes the audio channel |
| **Sensor node** | `imu`, sensors — no `render` | motion, temperature, presence; publishes events and channels |
| **Control surface** | inputs — no `render` | buttons, encoders, faders; publishes events and parameter values |
| **Bridge / gateway** | `bridge`, `gateway` — no `render` | protocol translation only |
| **Compute node** | `sim`, `keeper`, `compile` | a Pi or a spare ESP32 doing work for the mesh |

Consequences worth being deliberate about:

- **Capacity matters more, not less, for LED-less devices.** A node with no LEDs takes no LED deduction, so its capacity score is the highest in the mesh, making it the natural `sim`, `keeper` or `compile` holder. The election favours this automatically.
- **An LED-less device still needs a way to be identified and paired.** The blink-code identification in [[App]] does not apply. Give it a status LED, or fall back to a "beep / vibrate / flash the status LED" identify command. A device with **no output at all and no button** has no way to confirm pairing physically, which the security model requires ([[Protocol#Trust in an open-source system]]) — such a board must carry a printed QR, and the board definition should be rejected if it declares neither a button, an indicator, nor a QR.
- **Geometry is a degree, not a flag.** Every device has coordinates — synthetic, rough, or truly mapped ([[Runtime Model#Unmapped devices mapping is pure upgrade]]) — carried in the `mapq` field. Nothing in the system may assume a device has *no* position; what it must respect is how much that position can be trusted.
- **Shape is never assumed.** Nothing in the firmware or protocol should encode "strip". A device declares its LED topology (path, lattice, or point cloud) and its coordinates; effects address it through zone projections ([[Runtime Model#Projections]]).

## Configurability: one binary where possible, build variants where necessary

The goal is that adding this firmware to **any** board is easy. The way to get there is to push as much as possible into runtime configuration, so most users never compile anything.

**Runtime configuration (NVS, changeable from an app, no rebuild):**
output pin per channel, LED chip type and colour order, LED count, topology, current limit, frame rate, sensor pin assignments, I2C/SPI addresses, button mapping, role preferences.

**Compile-time configuration (needs a build variant):**
target chip and SDK, which radios are compiled in (BLE adds ~100 KB+), whether the audio pipeline, the on-device compiler, the sim engine or the keeper store are included, and any driver that costs flash you do not have.

The split matters: if pins and LED types are compile-time, every user needs a toolchain and the project is only usable by people who already build embedded firmware. If they are runtime, **a handful of prebuilt variants covers almost everybody**.

### Board definitions

A board definition is a declarative file — checked in, contributed by anybody, and the main way the project grows hardware support:

```
board "generic-esp32s3-devkit" {
  chip     esp32s3
  flash    8MB
  psram    none
  outputs  [ { gpio: 12 }, { gpio: 13 } ]
  features [ ]
}

board "my-lamp-rev-b" {
  chip     esp32c6
  flash    4MB
  outputs  [ { gpio: 5, type: ws2812, count: 24, topology: ring } ]
  features [ ble, imu, button, audio_i2s ]
  imu      { bus: i2c, sda: 6, scl: 7, part: lsm6ds3 }
  audio    { bus: i2s, ws: 8, sck: 9, sd: 10 }
  button   { gpio: 0, pull: up }
  qr       printed
}
```

A board definition selects a build variant *and* supplies the default runtime configuration. Supporting a new board is then a pull request containing one file, not a firmware fork — which is what "easy to add to any device" actually requires.

> Adopting an existing board-definition convention where one fits (PlatformIO board files, ESP-IDF's device tree-ish config) is worth checking before inventing this format. Inheriting an ecosystem's boards beats defining your own.

### Prebuilt variant matrix

Publish signed prebuilt binaries for the common combinations — chip × (with/without BLE) × (with/without audio) — so the desktop firmware tool downloads and flashes without any toolchain present. Only genuinely unusual boards need a local build. See [[Desktop Application#Firmware tool]].

## Roles

A device holds one or more roles, advertised as `caps` in [[Protocol#Discovery TXT records]]. Roles are elected or configured, not baked into a build.

| Role | Held by | Job |
|---|---|---|
| `render` | almost every device | run the [[Bytecode VM]] program, drive LEDs |
| `keeper` | devices with enough flash | store and gossip the replicated show state ([[Data Model]]) |
| `timebase` | one, elected | own the show clock, answer SYNC_REQ |
| `sim` | one, elected | run shared simulations, broadcast the sim channel |
| `gateway` | configured | terminate Art-Net / MQTT / HTTP / Home Assistant |
| `bridge` | configured | proxy non-WiFi nodes |
| `audio` | devices with a mic or line-in | FFT and beat detection, broadcast the audio channel |
| `compile` | devices with the compiler built in and RAM to spare | compile [[Effect Language]] source without an app present |

The canonical token list lives in [[Protocol#Discovery TXT records]] — add new capabilities there first.

Election for `timebase` and `sim` is highest **capacity score**, ties broken by lowest UUID — capacity only, never current load ([[#PPOS capacity not current load]]). Re-election is triggered by three missed TICKs. **The two roles should be allowed to sit on different devices** — the sim is bursty and the timebase wants to be boring.

`keeper` is elected too, and **capped at 5–7 devices** ranked by flash size then capacity score. At the ~50 device target, letting everyone keep state would make gossip traffic grow quadratically for no benefit. Non-keepers pull the records they need read-only and cache them. See [[Protocol#Scale target ~50 devices 10k LEDs]].

## Data inputs

- **Direct drive of LEDs** — FRAME packets, and the ingest path for Art-Net / E1.31 / DDP
- **Program execution** — the normal path, a compiled effect running locally
- **Channels** — shared state read as uniforms by the running program
- **Local sensors** — mic, IMU, buttons, temperature

## Data outputs

- **Events** — discrete occurrences (IMU tap, button, threshold crossed), broadcast using the webhook pattern, and mirrored to MQTT and Home Assistant
- **Streams** — continuously changing values (audio FFT, IMU acceleration) published as channels; WebSocket for app subscribers, multicast CHAN for the mesh
- **Rendered output** — optionally republished as Art-Net universes or as FRAMEs for bridged nodes

## Stored

| Item | Notes |
|---|---|
| UUID | randomly generated on **first boot**, persisted in NVS, **survives reflashing**. Generating it at flash time would give a device a new identity every time the firmware tool updates it — losing its coordinates, zone membership and pairing |
| Capacity score | **measured, not configured** — a benchmark program runs at boot and yields VM instructions/second ÷ 1000. Static. See [[#PPOS capacity not current load]] |
| Chip type | drives program variant selection |
| LED coordinates | relative to the device root; set by [[App]] mapping, by import, or hardcoded |
| Device root position and orientation | set by the app |
| Output configuration | per channel: LED type, count, colour order, gamma, max current |
| Program pool | as many resident programs as RAM allows, admitted highest-priority-first ([[Runtime Model#Concurrency dynamic admission-controlled]]), with a guaranteed floor of two concurrent plus one cross-fading. Plus a read-only factory fallback that is never evicted |
| Keys | mesh key, authorised controller public keys, own keypair |
| Replicated records | if `caps=keeper` |

Measuring capacity rather than setting it at flash time is worth doing: it stays honest across chip revisions, clock configurations and firmware versions, and it makes the budget check in the compiler trustworthy.

### PPOS: capacity, not current load

The original PPOS idea conflated two different numbers, and conflating them creates a live bug. If the score used for elections is *reduced by current load*, then a device that wins the `sim` role immediately drops below its rivals, loses the role, rises again, and wins it back — **the mesh flaps between masters forever**, and every flap re-elects, re-syncs and possibly re-uploads.

Split it in two:

| | Capacity score (`cap`) | Load (`load`) |
|---|---|---|
| Measured | benchmark at boot, plus a fixed deduction for the LEDs this device drives | continuously, as actual VM time used per frame |
| Changes | only on reboot or reconfiguration | constantly |
| Used for | **elections, and only elections** | budget reporting, diagnostics, admission checks, UI |

Elections read `cap` and nothing else, so the outcome is stable by construction. `load` is advisory: the compiler uses it to warn, the app shows it, and a device refuses new work when it is saturated — but it never changes who holds a role.

Two supporting rules:

- **Hysteresis on yielding.** A device only yields a role to a rival whose capacity exceeds its own by a margin, and only after the rival has been visible for several TICKs. Otherwise a device rebooting with a marginally different benchmark result triggers a needless handover.
- **Capacity is deducted for LEDs, not for roles.** Driving 900 LEDs genuinely costs render budget and should lower the score permanently. Holding `sim` does not, because the role can be given away.

This is why an LED-less compute node naturally wins `sim`, `keeper` and `compile`: it has no LED deduction, so its capacity is the highest in the mesh, and it stays highest no matter how much work it takes on.

## Boot sequence

1. Load NVS: UUID, keys, WiFi credentials. No credentials means entering provisioning: BLE advertisement where the chip supports it, SoftAP otherwise, both entered from the device's QR code ([[Runtime Model#Provisioning]]).
2. Start LED output immediately with the last-known or factory program, before networking. **Lights come on fast even if the network never does.**
3. Join WiFi, start mDNS, discover peers.
4. Sync clock, converge, then join rendering in phase.
5. Pull replicated state, verify program slot hashes and signatures.
6. Advertise capabilities, participate in elections.

Step 2 matters more than it looks. A light that takes eight seconds to come on after a power cut feels broken regardless of what it does afterwards.

## Render loop

```
per frame:
  latch show_time for this frame's presentation deadline
  read channels, apply hold/decay for stale ones
  for each admitted source, highest priority first:
    run once-section if newly activated
    run frame-section
    for each pixel in the source's zone: run pixel-section into that source's buffer
  composite the source buffers and any active FRAME sources by priority, applying fades
  colour pipeline
  DMA out
```

Rendering targets a fixed frame rate per device (default 60 fps, configurable down for weak or bridged nodes).

**Frame grid.** "All devices latch the same show time" only means something if there is a shared grid to latch onto. Define a mesh-wide base rate — 120 Hz — and require every device's frame rate to be an integer division of it (120, 60, 40, 30, 24, 20, 15). Frame *k* is presented at `show_epoch + k × (1/120 s)`, so a 30 fps device renders every fourth grid slot and lands on instants a 60 fps device also renders. Without this, two devices at different rates drift in and out of phase and a sweep crossing between them wobbles.

This also constrains the runtime fallback in [[#Failure behaviour]]: a device that cannot hold its rate drops to the **next lower grid rate**, never to an arbitrary one.

## Colour pipeline

Between the program's output and the wire:

1. Brightness and per-zone dimming
2. **Per-device colour calibration** — a stored 3×3 (or 4×4 for RGBW) correction matrix plus per-channel gain, so two strips from different batches actually match. Effects author in one consistent colour space and each device corrects into its own
3. Colour temperature / white channel derivation for RGBW and CCT strips
4. Gamma correction, 8-bit in, higher internal precision
5. **Temporal dithering** — with 8-bit output, the low end of a fade is visibly steppy; dithering across frames recovers roughly 3–4 effective bits and is the single biggest visual quality win available in firmware
6. **Power limiting** — sum estimated current, scale the whole frame if it exceeds the configured supply budget

Storing calibration as a **matrix** rather than three gain values is what keeps the door open. Values can start as a manual eyeball adjustment in the app, later be derived from phone-camera comparison of strips side by side, and later still come from a colorimeter — all writing to the same field, with no change to the pipeline, the protocol or any effect. Getting this field in place now costs almost nothing; retrofitting a colour pipeline once people have authored effects against an uncorrected one is painful.

The internal working space should be **linear and higher than 8-bit** (16-bit per channel), with gamma applied only at the very end. Blending, fading and dithering in gamma-encoded 8-bit is what makes cheap controllers look cheap.

Power limiting should scale globally rather than clip individual pixels, so an over-budget frame dims uniformly instead of changing colour. Configure as mA per output, defaulting conservatively. This turns "my PSU browns out at full white" from a debugging session into a non-event.

## Output drivers

| Type | Method |
|---|---|
| WS2812 / SK6812 family | RMT (ESP32) or PIO (RP2040) |
| APA102 / SK9822 / HD107 | SPI, preferred where refresh rate or dithering matters |
| Analog / PWM strips | LEDC channels |
| DMX / RS485 | via `caps=bridge` with a transceiver |

Multiple output channels per device, each independently configured. Long WS2812 runs bound the frame rate (~30 µs per LED), which the compiler must know about — the fps ceiling is a wire property, not a CPU property.

## Structured for simulation

The mesh simulator with deterministic replay ([[Desktop Application#Simulator]]) is not a testing add-on — it constrains how the firmware is written, so it has to be decided now.

- The **core** — protocol, state machine, election, replication, VM, compositor, colour pipeline — is portable code with no platform dependencies, compilable for the host. In practice a set of Rust crates ([[Tech Stack#Crates]]).
- Everything platform-specific sits behind a thin **HAL**: clock, network, storage, LED output, sensors, RNG.
- **All nondeterminism is injected**, never ambient. The core is **sans-IO**: it takes events and returns actions, and performs no I/O itself. There is no `rand()` to accidentally call and no socket to accidentally open, because the core cannot reach them — determinism is enforced by the type system rather than by code review.
- Log every inbound event with its logical timestamp, so a session can be replayed exactly.

The payoff: elections, split-brain, clock drift and packet loss become reproducible test cases instead of things you observe once and cannot recreate. For a distributed system this is the difference between debuggable and not — and for an open-source project it means contributors can work on the hard parts without owning fifty devices.

## Failure behaviour

| Failure | Response |
|---|---|
| WiFi lost | keep rendering the current program on the free-running clock; drift is unnoticeable for tens of seconds |
| Timebase master lost | re-elect after 3 missed TICKs; keep rendering throughout |
| Channel stale | hold, then decay to the channel's declared default after `hold_ms` |
| Program slot corrupt | drop that source, keep rendering the rest of the stack; if the pool is empty, the factory program |
| Program over budget at runtime | drop frame rate before dropping frames; report it as an event |
| Crash | watchdog reset; LEDs resume within the boot budget |

## OTA

Firmware OTA is separate from program upload — different signing key, A/B partitions, automatic rollback if the new image fails to rejoin the mesh within a timeout. Programs are data and change constantly; firmware is code and changes rarely. Never conflate the two paths.

## Open questions

- Do you want an **ESP-NOW fallback** for when WiFi infrastructure is missing? It would let a set of devices form a working mesh with no router at all, which is a real advantage for portable or event use. It is a second transport to maintain, so it is a v2 question, but the protocol should not accidentally preclude it.
- What is the minimum viable target — is an ESP32-C3 with 4 MB flash expected to be a keeper, or is keeping restricted to S3-class devices?
- Open-source specific: the build must be reproducible enough that a user can verify the firmware on their device matches the published source. Worth deciding whether to publish signed reference builds alongside the source, and whether a device reports the hash of the image it is running.
