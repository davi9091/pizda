# AGENTS.md

Guidance for AI coding agents (and humans) working on this repository.

## What this project is

dogecar-ui is a terminal dashboard for live car telemetry, plus the Raspberry Pi OS image that runs it in the car.

- **RX-8 first, built to expand.** The first and primary target is a Mazda RX-8 (CAN, ISO 15765-4, 11-bit, 500 kbps), and RX-8 needs take priority. Don't hard-code RX-8 assumptions where a setting or mapping would do: keep car-specific details (protocol, PIDs, sensor scaling) in config or clearly named constants, so other cars can be added later.
- **Hardware:** Raspberry Pi Zero 2 W (aarch64, 512 MB RAM), an ELM327 OBD adapter over classic Bluetooth (SPP) or USB, and later I2C/GPIO sensors. The final display is meant to be the car's factory nav screen; until then it's headless (viewed over SSH from a phone) or HDMI on the bench.
- **The plan** is `docs/pi-build-plan.md`: milestones, decisions and optional branches. Read it before larger changes, and update its status when a milestone step is completed or a decision changes.

## Repository layout

```
src/            the dashboard app (Rust, ratatui)
  source/       data inputs: elm327, mock, replay; each runs on its own thread
  ui/           dashboard widgets and panels
  obd.rs        OBD-II PID definitions and decoding
  signal.rs     the signals the app knows about, with stable keys
  recording.rs  the CSV recording format
os/             Buildroot external tree for the Pi image (see os/README.md)
docs/           the build plan
build/, out/    local build output and the flashable image (gitignored)
```

## Commands

```sh
cargo build                          # build the app
cargo test                           # unit tests
cargo fmt                            # format (settings in rustfmt.toml)
cargo clippy --all-targets           # lints
cargo run -- --mock                  # run the dashboard with fake data
cargo run -- --replay <file.csv>     # replay a recording
make image                           # build the Pi image in podman → out/dogecar.img
```

Before finishing a change: `cargo fmt --check`, `cargo clippy --all-targets` with **zero warnings**, and `cargo test` must all pass.

**Toolchain:** `rust-toolchain.toml` pins Rust **1.88.0**, the version Buildroot uses to build the app for the Pi. Code using newer language or library features compiles on a newer local Rust but breaks the image build, so don't bump the pin on its own: the Buildroot Rust version (`package/rust-bin/rust-bin.mk` in the pinned Buildroot release) has to move with it.

`make image` takes about an hour the first time and needs podman and ~20 GB of disk. Don't run `make clean` or `make distclean` without being asked; they throw away that work.

## Hard requirements

These come from the car, not from taste. Don't trade them away without discussing it first.

1. **Power can be cut at any moment.** The car's supply is unstable and every key-off is a power cut. Nothing may corrupt the system or lose more than ~2 s of data:
   - the root filesystem is read-only; the only writable storage is `/data`
   - files are append-only or replaced atomically (write a temp file, fsync, rename, fsync the directory); never rewrite in place
   - every boot step must be safe to repeat after being interrupted
2. **The dashboard must always come up.** Failures in storage, sources or config degrade a feature (e.g. "not recording") with a visible status; they never stop the boot or the app.
3. **No wall clock.** There's no RTC and often no phone connected, so the system date is meaningless. Use monotonic time and the boot counter, never the system date, for ordering or naming.
4. **Stable signal keys.** `Signal::key()` values are stored in recordings. Never rename or reuse an existing key.
5. **Small hardware.** The app runs on a Pi Zero 2 W: keep CPU, memory and dependencies modest (no async runtime without a good reason), and remember the image has a size and boot-time budget (under 10 s to the dashboard).

## Ask before

Check with the maintainer before:
- adding or upgrading a dependency (crate, Buildroot package, kernel option)
- changing the Buildroot defconfig, `config.txt`, `cmdline.txt` or the partition layout
- changing anything about how `/data` is mounted, grown or written
- changing the recording format or any signal key
- adding a boot-time service or changing the boot order
- bumping the Rust toolchain

## Conventions

- Match the surrounding code: naming, comment density and idioms. Comments explain *why*, not *what*. The formatting is deliberately wide; tables (like the PID list in `obd.rs`) stay one entry per line, using `#[rustfmt::skip]` where needed.
- Errors: `anyhow` with context (`.context(...)` / `with_context`) at the point of failure.
- Sources report through `Sink`; nothing downstream should care which source produced a value.
- Shell scripts that run on the Pi are POSIX `sh` for BusyBox `ash`, not bash.
- Secrets (SSH keys, Wi-Fi passphrase) go in `os/secrets/`, which is gitignored. Never commit them or print them.

## Good examples to copy

- `src/source/mock.rs`: the smallest complete `Source`
- `src/source/elm327.rs`: a real source, including reconnecting after errors
- `src/obd.rs`: table-driven decoding with focused unit tests
- `src/recording.rs`: a file format documented in its module comment, with tests for good and bad input
- `os/board/dogecar/rootfs-overlay/etc/init.d/S00data`: a boot script that is safe to interrupt at any point and never stops the boot

## Gotchas

- **Buildroot doesn't track config changes.** After editing the defconfig, a plain `make image` may not rebuild what changed. Rebuild the affected package (`make br-<package>-rebuild`, or `make br-linux-reconfigure` after editing `linux.fragment`). Changes to the root filesystem type, the init system or removed packages need `make clean && make image`.
- **`config.txt` and `cmdline.txt` are installed by the `rpi-firmware` package**, so run `make br-rpi-firmware-rebuild` before `make image`, and then reflash. Changes to `rootfs-overlay/` are picked up by a plain `make image`.
- **`cmdline.txt` must stay a single line**; the firmware ignores anything after the first line break.
- **`genimage.cfg.in` placeholder:** `post-image.sh` substitutes `#BOOT_FILES#` everywhere in the template, including comments, so that marker must only appear where the file list goes.
- **The root filesystem is read-only on the Pi.** Code that writes outside `/data`, `/tmp` or `/run` works on your PC and fails on the device.
- **Buildroot paths are container paths.** Builds run in podman with the repo mounted at `/work`; paths inside Buildroot config use `$(BR2_EXTERNAL_DOGECAR_PATH)`, not host paths.
- **Boot scripts must not exit non-zero or use `set -e`**: a failing step has to record its status and let the boot continue.

## Testing and hardware

Agents can't run anything on the Pi. When a change needs hardware to verify (boot behaviour, display, Bluetooth, power cuts), say exactly what was verified locally (build, tests, inspecting the image) and give the person a short checklist to run on the device. Don't describe hardware behaviour as tested when it wasn't.

Never write to a block device: don't run `make flash` or `dd`. The person flashes cards themselves.

## Commits

- **Don't add agent co-authoring or attribution to commit messages**: no `Co-Authored-By:` trailers for AI agents and no "generated with" lines, in commits or pull requests.
- Use a short scope prefix, as in the existing history: `os:`, `docs:`, `fix:`, or the area of the app being changed.
- Explain *why* in the body, and mention what was tested and how (including on hardware, if it was).
- One logical change per commit. Commit only when asked.
