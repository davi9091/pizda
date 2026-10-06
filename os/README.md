# os/: the Raspberry Pi image

A Buildroot external tree that builds a flashable SD card image for the Pi Zero 2 W. The plan behind it is in [`docs/pi-build-plan.md`](../docs/pi-build-plan.md).

## Usage

You need podman and about 20 GB of free disk space. Everything else runs in the build container.

```sh
make image                  # first run takes roughly an hour; later runs take minutes
make flash DEV=/dev/sdX     # checks the target is a removable card and asks for confirmation
```

Then boot the Pi with the card. For now (M1) you get a root shell with no password:
- on the **serial console**: GPIO 14/15, 115200 baud, `/dev/ttyS0` on the Pi
- on **HDMI** with a USB keyboard

## Changing things

| What | How |
|---|---|
| OS packages and options | `make menuconfig`, then `make savedefconfig` to write `configs/dogecar_defconfig` |
| Kernel options | add them to `board/dogecar/linux.fragment` (`make linux-menuconfig` to explore) |
| Firmware and boot options | `board/dogecar/config.txt`, `board/dogecar/cmdline.txt` |
| Files in the root filesystem | `board/dogecar/rootfs-overlay/` |
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
