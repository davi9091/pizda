# os/: the Raspberry Pi image

A Buildroot external tree that builds a flashable SD card image for the Pi Zero 2 W. The plan behind it is in [`docs/pi-build-plan.md`](../docs/pi-build-plan.md).

## Setup

### 1. Build the image

You need podman and about 20 GB of free disk space. Everything else runs in the build container. Run this from the repo root:

```sh
make image
```

The first run takes roughly an hour; later runs take minutes. The result is `out/dogecar.img`.

### 2. Find the SD card

Insert the card (directly or in a USB reader) and list the disks:

```sh
lsblk -o NAME,SIZE,TYPE,RM,TRAN,MODEL,MOUNTPOINTS
```

The card is the disk with the right size that shows `RM 1` and/or `TRAN usb`, e.g. `sdb`, or `mmcblk0` in a built-in reader. **Double-check this:** everything on that disk gets erased.

### 3. Unmount it

Desktops often auto-mount the card's partitions (e.g. `BOOT` and `data` from a previous flash). `make flash` refuses to write while anything on the card is mounted, so unmount every partition listed under `MOUNTPOINTS` (use `p1`, `p2`… for `mmcblk0`):

```sh
udisksctl unmount -b /dev/sdX1
udisksctl unmount -b /dev/sdX2   # and so on for each mounted partition
```

### 4. Flash

Run this in a regular terminal: it asks for confirmation and for your `sudo` password.

```sh
make flash DEV=/dev/sdX          # the whole disk, not a partition (sdb, not sdb1)
```

Before writing anything, `os/scripts/flash.sh` checks that `DEV` is a whole disk, is removable (or USB, or a built-in SD reader), and has nothing mounted. It then shows the size and model, and writes only after you type `yes`. Writing takes under a minute.

### 5. Check and eject

```sh
lsblk /dev/sdX                   # three partitions: 64M, 256M, 64M
udisksctl power-off -b /dev/sdX  # safe to remove afterwards
```

If the desktop auto-mounts `BOOT` and `data` again, that's harmless; just power off or eject before pulling the card.

### 6. First boot

Put the card in the Pi and power it on. The first boot takes a few seconds longer because it grows the `data` partition to fill the card. Then log in as `root` with no password (key-only SSH comes in M6b), either:
- on the **serial console**: GPIO 14 (TX) and 15 (RX) plus ground, 115200 baud, with a **3.3 V** USB-serial adapter
- on **HDMI** with a USB keyboard (through an OTG adapter)

Quick checks: `mount` shows `/` as read-only squashfs and `/data` as ext4, `df -h /data` shows roughly the card's size, and `cat /run/dogecar/data_status` prints `ok`.

### 7. The dashboard

The dashboard starts at boot in a tmux session, whether or not anyone is watching.

- **With a screen on the HDMI port**, it's shown full-screen. Ctrl+Alt+F2 switches to a login prompt; Ctrl+Alt+F1 goes back.
- **From any login** (serial, the HDMI login on tty2, later SSH), view it with:

  ```sh
  dogecar-attach        # Ctrl-b d to detach; the dashboard keeps running
  ```

Quitting the app (`q` or Esc) just restarts it after 2 s.

Settings live in `/data/dogecar.toml`, so they survive updates. None are required:

```toml
args = "--mock"                 # app arguments; e.g. "--elm /dev/ttyUSB0" for a USB ELM327
display = "auto"                # auto (HDMI if a screen is connected), hdmi, or headless
font = "DejaVu Sans Mono"       # hdmi profile only
font_size = "20"
pad = "0x0"                     # space around the dashboard in pixels, e.g. "48x24" for a TV that cuts off the edges
keyboard_layout = "us"          # hdmi profile only; any XKB layout name, e.g. "de" or "nl"
```

`cat /run/dogecar/display_status` shows which profile is active. After changing settings, restart with `killall tmux` (init starts everything again) or reboot.

## Changing things

| What | How |
|---|---|
| OS packages and options | `make menuconfig`, then `make savedefconfig` to write `configs/dogecar_defconfig` |
| Kernel options | add them to `board/dogecar/linux.fragment` (`make linux-menuconfig` to explore) |
| Firmware and boot options | `board/dogecar/config.txt`, `board/dogecar/cmdline.txt` |
| Files in the root filesystem | `board/dogecar/rootfs-overlay/` |
| The app in the image | change the code, then `make br-dogecar-ui-rebuild` and `make image` |
| Partition layout | `board/dogecar/genimage.cfg.in` |
| Buildroot version | `buildroot.version` (version and SHA256 from the release's `.sign` file) |

Any Buildroot target works through `make br-<target>`. For example, `make br-linux-rebuild` rebuilds the kernel, and `make br-busybox-menuconfig` configures BusyBox.

## Layout

```
os/
├── Containerfile           pinned Debian build environment
├── buildroot.version       pinned Buildroot release and checksum
├── external.desc/.mk, Config.in   Buildroot external tree glue
├── configs/dogecar_defconfig
├── board/dogecar/          firmware config, kernel fragment, image layout, build hooks
├── package/                own packages (dogecar-ui, dogecar-web from M4)
├── scripts/flash.sh
└── secrets/                gitignored build-time secrets (from M6b)
```

Build output goes to `build/`: the Buildroot source, the download cache, ccache and `build/output`. The finished image is `out/dogecar.img`. Both directories are gitignored.
