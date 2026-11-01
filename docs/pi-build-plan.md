# dogecar-ui on the Raspberry Pi: build plan

How to turn dogecar-ui into a car-ready appliance: a reproducible, locally built OS image that boots straight into the dashboard and survives power being cut at any moment.

Status: **M1 done** (2026-10-06: boots to a root shell on a real Zero 2 W). **M2 done** (2026-10-31: read-only root, `/data` grows and survives power cuts, counters correct on real hardware). **M3 + M3b done** (2026-11-01: dashboard starts at boot in tmux; full-screen kiosk on HDMI with working keyboard, console switching and hidden cursor; headless profile works). Next: M4.

---

## 1. Decisions so far

| Area | Decision |
|---|---|
| Board | Raspberry Pi Zero 2 W (aarch64, 512 MB RAM, one USB OTG data port, no DSI connector) |
| OS | **Buildroot**, built from source in a container on the dev machine, producing a flashable `.img` |
| Root filesystem | Read-only squashfs. The only writable storage is a separate `/data` partition. |
| Display (goal) | The RX-8's factory pop-up nav screen (analog RGB). How to drive it is still being worked out. |
| Display (interim) | Headless: the dashboard runs in tmux and is viewed from a phone over SSH. HDMI on the bench. |
| Inputs | ELM327 over Bluetooth (classic SPP) and USB, plus I2C/GPIO sensors (list TBD) |
| Signals | Built-in OBD-II set, plus user-defined mappings in `/data/signals.toml` (car-specific PIDs, own sensors) |
| Networking | The Pi runs its own Wi-Fi access point (country **NL**). SSH stays enabled in production. |
| Web | A read-only status page: health, power flags, sources, recordings |
| Power | Switched ACC with an automotive 12→5 V converter. A timed shutdown controller is optional. |
| Hard requirement | Pulling power at any moment must never corrupt the system or lose more than ~2 s of data |

---

## 2. System overview

```
                ┌──────────────── Raspberry Pi Zero 2 W ────────────────┐
 ELM327 (BT) ──►│ rfcomm0 ─┐                                            │
 ELM327 (USB) ─►│ ttyUSB0 ─┼─► dogecar-ui ──► tmux session ──► ssh ─────┼──► phone
 I2C / GPIO ───►│ i2c-1  ──┘      │   │          └─► cage+foot ─────────┼──► display (profile)
                │                 │   └─► /run/dogecar/status.json      │
                │                 │                 └─► dogecar-web ────┼──► phone browser
                │                 └─► /data/recordings/*.csv            │
                │  hostapd + dnsmasq (AP 192.168.4.1) · sshd            │
                └───────────────────────────────────────────────────────┘
```

### SD card layout

| Partition | Filesystem | Mounted | Contents |
|---|---|---|---|
| p1 boot | FAT32, ~64 MB | not mounted | firmware, kernel, device tree, `config.txt`, `cmdline.txt` |
| p2 root | squashfs | read-only | the whole OS and both binaries |
| p3 data | ext4, `noatime` (default `data=ordered`) | read-write | recordings, config, Bluetooth keys, SSH host keys, Wi-Fi config |
| (RAM) | tmpfs | read-write | `/tmp`, `/run`, `/var/log` |

### Boot sequence

```
power → firmware → kernel (quiet) → init
      → e2fsck -p /data → mount /data   (if it fails: keep booting, mark "not recording")
      → first boot only: grow p3, generate SSH host keys
      → bluetooth + rfcomm bind (if paired)
      → hostapd + dnsmasq (or client mode, see 6b) → sshd → dogecar-web
      → tmux session "dash" running: dogecar-ui --auto
      → if the display profile is not headless: seatd → cage → foot → tmux attach
```

Target: **under 10 s from power to dashboard**. With ACC power the Pi boots right after every engine start.

---

## 3. Repository layout (target)

```
dogecar-ui/
├── Cargo.toml                    workspace
├── crates/
│   ├── common/                   Signal, recording format, status schema
│   ├── dogecar-ui/               the dashboard (current src/ moves here)
│   └── dogecar-web/              read-only status page
├── os/                           Buildroot external tree
│   ├── Containerfile             pinned build environment
│   ├── buildroot.version         pinned Buildroot LTS release and checksum
│   ├── configs/dogecar_defconfig
│   ├── package/
│   │   ├── dogecar-ui/           Config.in + .mk (Buildroot cargo package)
│   │   └── dogecar-web/
│   ├── board/dogecar/
│   │   ├── config.txt, cmdline.txt
│   │   ├── genimage.cfg
│   │   ├── post-build.sh, post-image.sh
│   │   └── rootfs-overlay/       init scripts, udev rules, foot.ini, hostapd/dnsmasq/sshd config
│   └── secrets/                  gitignored: authorized_keys, wifi.env
├── docs/
└── Makefile
```

### Make targets

| Target | Does |
|---|---|
| `make image` | Builds everything in podman → `out/dogecar.img`. Fails clearly if `os/secrets/*` is missing. |
| `make flash DEV=/dev/sdX` | Writes the image after checking that `DEV` is a removable device |
| `make menuconfig` / `make savedefconfig` | Edits the OS config |
| `make app` | Fast cross-compile of the binaries with the Buildroot toolchain |
| `make deploy HOST=192.168.4.1` | Copies the binaries to the device (dev override on `/data`) and restarts the services |

---

## 4. Milestones

Each milestone ends in something testable. They're mostly sequential, and the optional branches are marked with ⎇.

### M1 — Build infrastructure
1. Write the `Containerfile` with Buildroot's host dependencies on a pinned Debian base.
2. Add a Makefile wrapper that downloads the pinned Buildroot release, verifies its checksum and runs it inside the container with `BR2_EXTERNAL=os/`.
3. Create `dogecar_defconfig` from Buildroot's Pi Zero 2 W support (64-bit), with a glibc toolchain and the Rust host tools enabled.
4. Set up `genimage.cfg` with the three-partition layout above (p3 small for now).
5. Configure the kernel: vc4 KMS, brcmfmac (Wi-Fi), Bluetooth (hci_uart, rfcomm), i2c-dev, gpio cdev, the bcm2835 watchdog, and the USB serial drivers (ftdi_sio, ch341, cp210x, pl2303, cdc_acm).
6. Enable a serial console on the UART (`enable_uart=1`).

**Done when:** `make image && make flash` gives a card that boots to a shell on the serial console or HDMI. ✅

### M2 — Read-only system and the data partition
1. Make the root filesystem squashfs, with tmpfs for `/tmp`, `/run` and `/var/log`.
2. Add a `/data` init script: `e2fsck -p`, then mount. If that fails, continue without `/data` and leave a flag in `/run`.
3. Grow p3 to fill the card, then run `resize2fs`. No marker file: each boot checks for unused space after p3, so an interrupted grow simply finishes on the next boot.
4. Store the boot counter in `/data/state/boot_count`, written atomically.
5. Keep the boot partition unmounted.

**Done when:** after `make flash`, the first boot expands `/data`. Root can't be remounted read-write, and pulling power during boot recovers cleanly. ✅

### M3 — Kiosk and display profiles
Split on 2026-10-31 into the headless part (here) and the display stack (M3b), because the display stack needs Mesa and udev, which cost build time, image size and boot time. Both were then done together.

1. Create the `doge` user: UID 1000, no password login (SSH keys in M6b), in `dialout` and `tty`. Defined in `os/board/dogecar/users.txt`.
2. The app is built into the image (M4 step 2, done early), and init runs a tmux server as `doge` (`/usr/libexec/dogecar/session`, restarted if it exits). Its config (`/etc/dogecar/tmux.conf`) creates the `dash` session, whose pane runs the app in a restart loop (`/usr/libexec/dogecar/run`). App arguments come from `args` in `/data/dogecar.toml`; the default is `--mock` until `--auto` exists (M5).
3. Read the display profile (`display`) from `/data/dogecar.toml` (see M3b for the profiles).
4. `dogecar-attach` shows the dashboard in the current terminal (serial, HDMI console, later SSH); Ctrl-b d detaches.
5. Keep login prompts on serial and on an HDMI console: until M6b they're the only way in.

**Done when:** after boot, `dogecar-attach` (as root on the serial or HDMI console) shows the live mock dashboard, quitting the app restarts it, and killing the tmux server brings it back. ✅

### M3b — Display stack
Needed for the `hdmi` profile and the final screen (M9). Done right after M3 (2026-11-01).

1. Mesa (Gallium `vc4`, the Zero 2 W's GPU; EGL + GLES), eudev instead of kernel-only device handling, the seatd daemon (grants group `video` access to the display), cage and foot. Plus a generated `C.UTF-8` locale (`BR2_GENERATE_LOCALE`): glibc doesn't build it in, and foot refuses to start without a UTF-8 locale ("invalid locale, failed to find fallback").
2. `/usr/libexec/dogecar/display` runs on tty1 (restarted by init) and picks the profile from `display` in `/data/dogecar.toml`:
   - `auto` (default): `hdmi` if a screen is connected to the HDMI port, otherwise `headless`
   - `hdmi`: seatd → cage → foot → `dogecar-attach`, as `doge`. When the attach ends, the whole chain exits and init starts it again.
   - `headless`: a normal login prompt on tty1
   The result is written to `/run/dogecar/display_status`.
3. Fonts: no pixel font (Spleen, Terminus) or Nerd Font is packaged in Buildroot; **DejaVu Sans Mono** is, and it includes the braille characters ratatui charts use. Settings `font`, `font_size` (default 20) and `pad` override it per install. A pixel font or Nerd Font would mean adding a small font package of our own.
4. A login prompt stays on tty2 for development (Ctrl+Alt+F2 from the kiosk, Ctrl+Alt+F1 back).
5. HDMI edge cut-off: on a 4K TV, the first few characters of each line ended up off-screen (the TV overscans the upscaled 1080p signal). Fix with the TV's "Just Scan" / "Screen Fit" setting where possible. `cmdline.txt` still has 48/24 px kernel margins as a bench workaround; the kiosk can instead use foot's padding (`pad = "48x24"` in `dogecar.toml`), which lives in `/data` and survives updates. **To check on hardware:** whether the kernel margins also apply under cage. If they do, keep them only until the TV is set to "Just Scan"; if not, remove them from `cmdline.txt` and use `pad`.
6. **Keyboard and pointer in the kiosk:** `xkeyboard-config` provides the keyboard layouts. Without it cage has no keymap, so no keys work, including Ctrl+Alt+F*n*. Setting `keyboard_layout` (default `us`). The dashboard has no use for a pointer, so the kiosk uses an invisible cursor theme (`dogecar-hidden`, generated in `post-build.sh`); cage has no option to hide the cursor.
7. eudev also fixes M5's device permissions: its default rules give `/dev/ttyUSB*` to group `dialout`, which `doge` is in.

**Done when:** with a screen on the HDMI port (or `display = "hdmi"`), the dashboard appears full-screen at boot with readable charts, and Ctrl+Alt+F2 reaches a login prompt. ✅

### M4 — App packaging and dev loop
1. Turn the repo into a workspace (`crates/common`, `crates/dogecar-ui`); keep the tests green.
2. Add a Buildroot package for `dogecar-ui` using the cargo package support, building from the local source tree. **Done early in M3** (`os/package/dogecar-ui/`). After changing the app, run `make br-dogecar-ui-rebuild` before `make image`.
3. Set up `make app` and `make deploy`: binaries placed in `/data/dev/bin` take priority over the image's copies when present. This is a dev-only override that `make deploy --clean` removes.
4. **Data-safe updates: `make update DEV=/dev/sdX`.** Updates the system on a card without touching `/data` (recordings, config, keys, user-defined signals). The release stays a single `dogecar.img`.
   - **Layout check:** p1 and p2 on the card must start and end at the same sectors as in the image. If not, refuse and say a full `make flash` is needed (which wipes `/data`).
   - **Write only p1 (boot) and p2 (system)**, copied out of the image at their offsets.
   - **Never write sector 0** (the partition table) or p3. On the card, p3 has grown to fill it; the image's table still says 64 MB.
   - Same safety checks as `flash.sh`: whole removable disk, nothing mounted, confirmation.
   - If the write is interrupted, the card doesn't boot until `make update` is run again; `/data` is never at risk.
   - Show the image's version (e.g. from `git describe`, also written to `/etc/os-release` in the image).
5. **Data migrations.** A newer system must understand an older `/data`:
   - store the data-format version in `/data/state/data_version`
   - at boot, after `S00data` mounts `/data`, run each pending migration in order. Each one is safe to interrupt and rerun (same rules as `S00data`) and bumps the version only when it has finished. A failed migration is reported in `/run/dogecar/` and never stops the boot.
   - a system older than the data (a downgrade) leaves `/data` untouched and reports it, rather than guessing
   - config files: unknown keys are warnings (see M5), and removed keys stay accepted for at least one release
   - recordings never need migrating: the format and signal keys are stable (see AGENTS.md)
6. **Nothing user-customizable lives in p1**, because updates overwrite it. Settings that are in `config.txt` or `cmdline.txt` today (e.g. the HDMI margins) move to `/data` (see M3b).

**Done when:** a code change reaches the running Pi in under a minute without reflashing, and `make update` replaces the system on a used card while recordings, config and counters survive.

### M5 — App changes
1. **`--auto` mode with a probe phase**, shown on a short diag screen:
   - serial ports (`/dev/ttyUSB*`, `/dev/ttyACM*`, `/dev/rfcomm*`): anything that answers `ATZ` is an ELM327
   - supported-PID bitmaps (`0100`, `0120`, …) instead of discovering PIDs by repeated misses
   - an I2C scan for known sensor addresses, and the GPIO lines named in config
   - report written to `/data/diag/last.txt`, then switch to the dashboard (view mode) showing only panels with live signals
   - sources keep retrying in the background, so late devices still get picked up
2. **Config file** `/data/dogecar.toml`: display profile, Bluetooth MAC, sensor wiring, Wi-Fi mode. Unknown keys produce warnings, not errors. Keep `display` and `args` as top-level string settings: the boot scripts read them with a simple shell reader (`/usr/libexec/dogecar/config`).
3. **Device permissions:** eudev (added in M3b) gives `/dev/ttyUSB*` to group `dialout`. Check `/dev/rfcomm*` too once Bluetooth is in (M6a), and add a udev rule if needed.
4. **Crash-safe logger:**
   - recordings are append-only CSV (existing format), with an `fsync` every ~2 s and a new file every ~10 min
   - no wall clock (see ⎇ T): file names use boot counter and segment (`drive-000123-01.csv`), and a header comment records `boot`, `segment` and `uptime_at_start`; `t` stays relative, as the format already defines
   - if `/data` is unavailable, show a "not recording" indicator and keep the dashboard running
5. **Tolerant replay:** a cut-off final line is ignored instead of failing the whole file.
6. **Atomic config writes:** write a temp file, fsync it, rename it over the original, fsync the directory.
7. **Status snapshot:** write `/run/dogecar/status.json` about once a second (sources, latest values, recording state).
8. **Compact layout:** make the dashboard usable down to about 40×15 characters for phones and small screens.
9. Handle SIGTERM and SIGHUP cleanly: flush the recording, restore the terminal.

**Done when:** `dogecar-ui --auto` finds a USB ELM327 with no arguments and records a drive. Killing the power mid-drive loses at most ~2 s, and the replay source plays the result back.

### M5b — User-defined signals
Car-specific values (e.g. Mazda mode 22 PIDs documented by the RX-8 community) and your own sensors, added through config instead of code.

1. **Signal registry:** turn the closed `Signal` enum into a registry of built-in signals (same stable keys as today) plus user signals loaded from `/data/signals.toml`. A user key that collides with a built-in is rejected. Keys must stay stable, the same rule as for built-ins.
2. **Mapping fields:** `key`, `label`, `unit`, `precision`, `range` (for gauges), `every` (poll rate), `source`, plus source-specific fields:
   - `obd`: `request` (e.g. `22xxxx`), optional `header` (target ECU, e.g. `7E0`), `formula`
   - sensor sources (M7): device and channel, `formula`
3. **Formulas:** Torque-style expressions over the reply bytes (`A`, `B`, `C`, …), e.g. `(A*256+B)/4`, and over the measured voltage `V` for sensors. This lets community PID lists be copied almost directly. A small, tested expression evaluator; no general scripting.
4. **Built-ins:** decoding stays in Rust. Config can disable them and change `every`, `label` and `range`, but not their formulas.
5. **Probe and diag:** at startup, each mapping is tried once, and the diag report says which answered, which returned no data, and which failed to parse. Bad entries are skipped with a warning; they never stop the app.
6. **Replay:** recordings keep working whatever the current mappings are. Unknown keys load as "unknown" signals (key only, raw value) instead of failing the file.
7. **Display:** user signals appear in a generic "extra values" panel. A configurable layout can come later if it's wanted.
8. **Polling budget:** an ELM327 over Bluetooth manages roughly 10–20 requests a second in total, so the diag screen shows the effective rate per signal.

**Done when:** adding a mode 22 entry to `signals.toml` and restarting shows the value on the dashboard and in recordings, and removing it again leaves old recordings replayable.

Later, not in M5b: passive CAN listening (broadcast frames such as wheel speeds) through a dedicated CAN interface rather than the ELM327.

### M6a — Bluetooth ELM327
1. Add bluez5 (bluetoothd and the command-line tools); keep `/var/lib/bluetooth` on `/data`.
2. Pair once over SSH with `bluetoothctl` (or later from inside the app); store the MAC in `dogecar.toml`.
3. Bind `/dev/rfcomm0` to the paired adapter's channel at boot and rebind it if it drops.
4. Note: the adapter must be **classic SPP**; BLE-only adapters won't show up as a serial port.

**Done when:** after a reboot, the Bluetooth ELM327 connects with no manual steps.

### M6b — Networking, SSH and the status page
1. **Access point:** hostapd with `country_code=NL`, WPA2-PSK, channel 6 by default (configurable), at `192.168.4.1/24`.
2. **DHCP and DNS:** dnsmasq, with `dogecar.lan` resolving to the Pi.
3. **Client-mode switch:** if `/data/wifi/client.conf` exists, join that network with wpa_supplicant instead of hosting the AP. Never run both at once.
4. **SSH:**
   - OpenSSH, key-only logins, no root login, SFTP enabled
   - `authorized_keys` comes from `os/secrets/` at build time
   - host keys are generated on first boot and kept in `/data/ssh/`
5. **`dogecar-web`:** a small synchronous HTTP server, read-only, a single embedded page polling `/api/status`. It shows:
   - **Power:** under-voltage and throttling flags, now and since boot
   - **System:** uptime, CPU temperature, memory, boot counter, number of unclean shutdowns
   - **Storage:** `/data` usage, the last fsck result, recording state
   - **Sources:** state, adapter version, discovered PIDs and sensors, latest values
   - **Diag:** the last probe report
   - **Recordings:** list with download links, plus a zip of everything
6. **Phone setup** (documented in the README): saved SSH host entry that attaches to tmux directly, plus a bookmark for `http://192.168.4.1/`.

**Done when:** the phone joins `dogecar`, opens the dashboard over SSH, downloads a recording over SFTP, and opens the status page.

### M7 — I2C and GPIO sensors
1. Enable `dtparam=i2c_arm=on` and give the `doge` user access to `/dev/i2c-*` and `/dev/gpiochip*`.
2. Add a source per sensor type in the app (e.g. an ADS1115 ADC for pressure senders). Wiring goes in `dogecar.toml`; what each channel means (formula, unit, range) uses the M5b mapping format in `signals.toml`.
3. Optional: an ignition-sense input (see ⎇ P2).
4. **Pins:** keep sensor wiring easy to move between buses until the nav screen route is decided, because ⎇ D3 takes GPIO 2–21.

**Done when:** a configured sensor appears in the probe report and on the dashboard.

### M8 — Hardening
1. Enable the hardware watchdog (and have the supervisor feed it, if the init system supports that).
2. **Power-cycle torture test:** a relay or smart plug cuts power at random times while the Pi is recording, a few hundred times. Afterwards: `/data` fsck is clean, every recording parses, the config is intact and the boot counter is consistent.
3. **Boot time:** measure it, then trim kernel modules and services until it's under 10 s to the dashboard.
4. **Under-voltage check** in the real car: drive, crank, idle with the AC on, then check the "since boot" flags on the status page.
5. **Kiosk restart robustness.** Seen once on 2026-11-01 (not reproducible by hand): after killing parts of the display chain, it recovered into an empty cage window instead of the dashboard; a reboot fixed it.
   - supervise `seatd` from init (respawn) like the session and display scripts; today `S70seatd` starts it once, so nothing restarts it if it dies
   - shorten the stacked waits (tmux back-off 5 s, display back-off 5 s, `dogecar-attach` up to 15 s), so normal recovery takes seconds
   - **restart torture test:** a script that repeatedly kills a random piece of the chain (`dogecar-ui`, tmux server, tmux client, foot, cage, seatd) and checks that within ~15 s a tmux client is attached to `dash` again; when it isn't, it saves the process tree and `/run/dogecar/*` for diagnosis

**Done when:** both torture tests (power cuts and kiosk restarts) pass, boot time is on target and no under-voltage flags show after a real drive.

### M9 — Final display (⎇ see section 5)
Pick and implement one route from section 5 and add its profile.

---

## 5. Optional branches

### ⎇ D — Display output

| Route | What it is | Effort | Notes |
|---|---|---|---|
| **D0. Headless** (default) | tmux only; view from the phone over SSH | done in M3 | Always available, including alongside any other route |
| **D1. HDMI panel** | 5" 800×480 IPS (or similar) on mini-HDMI | Minutes | Sharpest and simplest. Needs 5 V; uses no GPIO. |
| **D2. Nav screen via composite** | Pi composite NTSC (test pads) → off-the-shelf Mazda RGB interface (e.g. NavInc) → factory screen | Config change, plus installing the interface | Lowest-risk way into the nav screen. The picture is soft but readable with a big font. Forum reports describe the install as fiddly. |
| **D3. Nav screen via direct RGB** | Pi DPI → resistor ladder (VGA666-style) → analog RGB with custom 15 kHz-class timing | A day or more, plus measuring the signal | Sharpest nav option. **Uses GPIO 2–21**, so the main I2C bus and the UART move to GPIO 22–27 or to other interfaces. Measure the nav unit's timing with a scope or logic analyzer first. A relay can switch the nav unit back in. |
| **D4. SPI TFT** | 3.5" ST7796 480×320 (or ILI9341 320×240, or a GC9A01 round display in a 52 mm gauge pod) | An evening | Cheap and small; leaves I2C and UART free. cage runs with software rendering on the SPI display device. About 15–25 fps full-frame. |
| **D5. Reversing-camera monitor** | Pi composite → 4.3–7" car monitor | Minutes | Runs on 12 V, built for cars. Same soft composite picture as D2. |

Not an option: DSI panels, because the Zero 2 W has no DSI connector. Avoid DPI panels like HyperPixel too: they take every GPIO pin.

**Checklist for any screen in a car:** IPS and bright enough for sunlight, a storage temperature rating that suits a parked car in summer, and a controllable backlight for night dimming.

**Space for text with an 8×16 font:** 320×240 gives about 40×15 characters, 480×320 about 60×20, and 800×480 about 100×30 (or about 66×20 with a 12×24 font).

The kernel ships with the drivers for all of these, so changing the screen is a profile change in `/data/dogecar.toml` plus, for D2–D5, a `config.txt` overlay.

### ⎇ P — Power

| Option | How | When to pick it |
|---|---|---|
| **P1. Switched ACC** (start here) | ACC or ignition circuit → fuse (1–2 A) → automotive buck converter (6–36 V in, 5 V at 3 A or more, reverse-polarity and transient protection) → short, thick cable or soldered to the 5 V pads | Default. No drain when parked. Every key-off is a power cut, which the system is designed for. The Pi usually boots after the engine starts, since ACC typically drops while cranking. |
| **P2. Permanent power plus a timed shutdown controller** | Permanent 12 V → controller → converter. The controller signals the ignition-sense GPIO, waits for the Pi to shut down, then **cuts power completely**. | When you want clean shutdowns, or Wi-Fi access after key-off (e.g. downloading recordings in the driveway). About €15–30 more. |
| **P3. Supercapacitor hold-up** (add-on to P1) | A few seconds of hold-up plus ignition sense, so the app flushes and stops writing before power drops | Extra safety for SD card wear, without P2's complexity |

**Never use permanent power without P2.** About 0.3–0.5 A of constant draw is 8–12 Ah a day, which flattens a typical car battery in a few days. A weak battery is especially bad for a rotary's hot starts.

**Power budget:**

| Setup | Typical | Peak |
|---|---|---|
| Pi alone | ~0.8 W | ~2 W |
| Pi + 5" HDMI panel + ELM327 | ~3–4 W | ~6 W |

### ⎇ I — Init system
- **I1. BusyBox init** (**chosen** in M2): fastest boot, simple respawn lines.
- **I2. systemd:** nicer dependency ordering, watchdog support and restart policies, for about 2–4 s more boot time.

Decided in M2: BusyBox init. Boot scripts live in `os/board/dogecar/rootfs-overlay/etc/init.d/`.

### ⎇ U — Updates
- **U1. Card update** (default): `make image && make update DEV=…` writes only the boot and system partitions and keeps `/data` (see M4). A full `make flash` wipes `/data`; it's only needed for a new card or a layout change.
- **U2. A/B root partitions:** two root slots and a boot flag; upload a new root image over Wi-Fi with automatic fallback if it fails to boot. Only worth it once the system is stable.

### ⎇ T — Real-time clock
**Decided: no wall clock for now.** The car often runs with no phone connected, so phone sync or a saved clock would be unreliable, and nothing on the system needs the real date. Recordings are identified by boot counter and segment, and timed with the monotonic clock (see M5).

If real dates become wanted later, add a `# start:` header line to recordings. The format already ignores comment lines, so old files stay valid. Possible sources:
- **T1. RTC module** (DS3231 on I2C, about €5): correct from the first second of boot. It uses the I2C pins, so plan around D3 (`i2c-rtc-gpio` can move it to other pins).
- **T2. Time from the phone:** the status page sends the phone's clock when opened. This only helps for drives where the phone connects.

---

## 6. Bill of materials (indicative)

| Item | Needed for | Price |
|---|---|---|
| Raspberry Pi Zero 2 W | everything | ~€20 |
| High-endurance microSD (32 GB) | everything | ~€10–15 |
| Automotive 12→5 V 3 A converter, fuse tap, 1–2 A fuse | P1 | ~€15 |
| ELM327 adapter, classic Bluetooth SPP and/or USB | M5, M6a | ~€10–25 |
| USB-serial adapter (3.3 V) for the console | M1 debugging | ~€5 |
| Micro-USB OTG hub | USB ELM327 plus anything else on the bench | ~€5–10 |
| Display, depending on branch | D1–D5 | €5–50 |
| Timed shutdown controller | P2 | ~€15–30 |
| DS3231 RTC (only if dates are wanted later) | T1 | ~€5 |
| Relay or smart plug | M8 torture test | ~€10 |

---

## 7. Open questions

1. **Nav screen signal:** the timing and pinout of the RX-8 nav unit's RGB output. This decides D2 vs D3.
2. **I2C/GPIO sensor list:** which senders and ADCs, and whether ignition sense is wanted.
3. **Car-specific values:** which RX-8-specific data you want first (M5b), to choose the first mode 22 PIDs to test.
4. **USB-serial adapter:** available for M1? Without one, early boot debugging falls back to HDMI.
5. **Reserve a second root partition now?** If A/B updates (⎇ U2) are likely, the layout needs root B. Adding it later means one full reflash that wipes `/data`, which is cheap now while `/data` holds nothing important, and costs 256 MB of card space. Not decided yet.

---

## 8. References
- RX-8 nav screen uses analog RGB, not composite or VGA: <https://mazdas247.com/forum/posts/2805356>
- NavInc video interface for Mazda RGB navigation systems (takes NTSC composite): <https://navinc.nl/multimedia-video-interface-madza-rgb-navigation-systems>
