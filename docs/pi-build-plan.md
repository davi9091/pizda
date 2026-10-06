# dogecar-ui on the Raspberry Pi: build plan

How to turn dogecar-ui into a car-ready appliance: a reproducible, locally built OS image that boots straight into the dashboard and survives power being cut at any moment.

Status: **planning**. Nothing in here is implemented yet.

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
| p3 data | ext4, `noatime,data=journal` | read-write | recordings, config, Bluetooth keys, SSH host keys, Wi-Fi config |
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

**Done when:** `make image && make flash` gives a card that boots to a shell on the serial console or HDMI.

### M2 — Read-only system and the data partition
1. Make the root filesystem squashfs, with tmpfs for `/tmp`, `/run` and `/var/log`.
2. Add a `/data` init script: `e2fsck -p`, then mount. If that fails, continue without `/data` and leave a flag in `/run`.
3. Add a first-boot script that grows p3 to fill the card, then runs `resize2fs`. It runs once, guarded by a marker file.
4. Store the boot counter in `/data/state/boot_count`, written atomically.
5. Keep the boot partition unmounted.

**Done when:** after `make flash`, the first boot expands `/data`. Root can't be remounted read-write, and pulling power during boot recovers cleanly.

### M3 — Kiosk and display profiles
1. Create the `doge` user (no password; SSH key only).
2. Start a tmux session `dash` at boot running `dogecar-ui --auto`, restarted if it exits.
3. Read the display profile from `/data/dogecar.toml`, defaulting to `headless`.
4. For profiles other than headless: seatd → cage → foot (with `foot.ini`: font, colours) running `tmux attach -t dash`.
5. Fonts: ship one pixel font (Spleen, Cozette or Terminus) and one Nerd Font, then compare them on the actual screen.
6. Remove the login prompt from the display; keep the serial console getty.

**Done when:** on boot, `ssh doge@<pi> -t tmux attach -t dash` shows the live dashboard. With the `hdmi` profile, the dashboard also appears on an HDMI monitor.

### M4 — App packaging and dev loop
1. Turn the repo into a workspace (`crates/common`, `crates/dogecar-ui`); keep the tests green.
2. Add a Buildroot package for `dogecar-ui` using the cargo package support, building from the local source tree.
3. Set up `make app` and `make deploy`: binaries placed in `/data/dev/bin` take priority over the image's copies when present. This is a dev-only override that `make deploy --clean` removes.

**Done when:** a code change reaches the running Pi in under a minute without reflashing.

### M5 — App changes
1. **`--auto` mode with a probe phase**, shown on a short diag screen:
   - serial ports (`/dev/ttyUSB*`, `/dev/ttyACM*`, `/dev/rfcomm*`): anything that answers `ATZ` is an ELM327
   - supported-PID bitmaps (`0100`, `0120`, …) instead of discovering PIDs by repeated misses
   - an I2C scan for known sensor addresses, and the GPIO lines named in config
   - report written to `/data/diag/last.txt`, then switch to the dashboard (view mode) showing only panels with live signals
   - sources keep retrying in the background, so late devices still get picked up
2. **Config file** `/data/dogecar.toml`: display profile, Bluetooth MAC, sensor wiring, Wi-Fi mode. Unknown keys produce warnings, not errors.
3. **Crash-safe logger:**
   - recordings are append-only CSV (existing format), with an `fsync` every ~2 s and a new file every ~10 min
   - file names use the boot counter (`drive-000123-01.csv`), because there's no clock yet
   - if `/data` is unavailable, show a "not recording" indicator and keep the dashboard running
4. **Tolerant replay:** a cut-off final line is ignored instead of failing the whole file.
5. **Atomic config writes:** write a temp file, fsync it, rename it over the original, fsync the directory.
6. **Status snapshot:** write `/run/dogecar/status.json` about once a second (sources, latest values, recording state).
7. **Compact layout:** make the dashboard usable down to about 40×15 characters for phones and small screens.
8. Handle SIGTERM and SIGHUP cleanly: flush the recording, restore the terminal.

**Done when:** `dogecar-ui --auto` finds a USB ELM327 with no arguments and records a drive. Killing the power mid-drive loses at most ~2 s, and the replay source plays the result back.

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
2. Add a source per sensor type in the app (e.g. an ADS1115 ADC for pressure senders), configured in `dogecar.toml`.
3. Optional: an ignition-sense input (see ⎇ P2).
4. **Pins:** keep sensor wiring easy to move between buses until the nav screen route is decided, because ⎇ D3 takes GPIO 2–21.

**Done when:** a configured sensor appears in the probe report and on the dashboard.

### M8 — Hardening
1. Enable the hardware watchdog (and have the supervisor feed it, if the init system supports that).
2. **Power-cycle torture test:** a relay or smart plug cuts power at random times while the Pi is recording, a few hundred times. Afterwards: `/data` fsck is clean, every recording parses, the config is intact and the boot counter is consistent.
3. **Boot time:** measure it, then trim kernel modules and services until it's under 10 s to the dashboard.
4. **Under-voltage check** in the real car: drive, crank, idle with the AC on, then check the "since boot" flags on the status page.

**Done when:** the torture test passes, boot time is on target and no under-voltage flags show after a real drive.

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
- **I1. BusyBox init** (leaning this way): fastest boot, simple respawn lines.
- **I2. systemd:** nicer dependency ordering, watchdog support and restart policies, for about 2–4 s more boot time.

Decide during M1 or M2; it mostly affects how M2, M3 and M6 are written.

### ⎇ U — Updates
- **U1. Reflash** (default): `make image && make flash`. `/data` lives on the same card, so back it up over SFTP first, or keep it by flashing only p1 and p2.
- **U2. A/B root partitions:** two root slots and a boot flag; upload a new root image over Wi-Fi with automatic fallback if it fails to boot. Only worth it once the system is stable.

### ⎇ T — Real-time clock
- **T0. None** (default): files are named by boot counter.
- **T1. RTC module** (DS3231 on I2C, about €5): real dates in file names and on the status page. It uses the I2C bus, so plan the pins around D3.
- **T2. Time from the phone:** the status page sends the phone's clock once when it's opened. No hardware needed, but less reliable.

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
| DS3231 RTC | T1 | ~€5 |
| Relay or smart plug | M8 torture test | ~€10 |

---

## 7. Open questions

1. **Nav screen signal:** the timing and pinout of the RX-8 nav unit's RGB output. This decides D2 vs D3.
2. **I2C/GPIO sensor list:** which senders and ADCs, and whether ignition sense is wanted.
3. **USB-serial adapter:** available for M1? Without one, early boot debugging falls back to HDMI.
4. **Init system:** I1 or I2, decided during M1 or M2.

---

## 8. References
- RX-8 nav screen uses analog RGB, not composite or VGA: <https://mazdas247.com/forum/posts/2805356>
- NavInc video interface for Mazda RGB navigation systems (takes NTSC composite): <https://navinc.nl/multimedia-video-interface-madza-rgb-navigation-systems>
