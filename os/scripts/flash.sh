#!/bin/bash
# Write an image to an SD card, refusing anything that doesn't look like one.
# usage: flash.sh <image> <device>
set -euo pipefail

IMAGE=${1:?usage: flash.sh <image> <device>}
DEV=${2:?usage: flash.sh <image> <device>}

die() { echo "flash: $*" >&2; exit 1; }

[ -f "$IMAGE" ] || die "$IMAGE not found; run 'make image' first"
[ -b "$DEV" ] || die "$DEV is not a block device"
[ "$(lsblk -dno TYPE "$DEV")" = disk ] || die "$DEV is a partition; pass the whole disk (e.g. /dev/sdb, not /dev/sdb1)"

read -r RM TRAN SIZE MODEL < <(lsblk -dno RM,TRAN,SIZE,MODEL "$DEV")
case "$DEV" in
	/dev/mmcblk*) ;;  # built-in SD reader
	*) [ "$RM" = 1 ] || [ "$TRAN" = usb ] || die "$DEV is neither removable nor USB ($TRAN); refusing" ;;
esac
if lsblk -no MOUNTPOINTS "$DEV" | grep -q .; then
	die "$DEV has mounted partitions; unmount them first"
fi

echo "About to overwrite $DEV ($SIZE ${MODEL:-unknown model}, ${TRAN:-mmc}) with $IMAGE."
read -r -p "Type 'yes' to continue: " answer
[ "$answer" = yes ] || die "aborted"

sudo dd if="$IMAGE" of="$DEV" bs=4M conv=fsync oflag=direct status=progress
sync
echo "Done. You can remove the card."
