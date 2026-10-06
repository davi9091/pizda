#!/bin/sh
# Runs after the target filesystem is assembled, before it is packed.
set -eu

# Login prompt on the HDMI console as well as the serial one (dev convenience;
# replaced by the kiosk in M3).
if [ -e "${TARGET_DIR}/etc/inittab" ]; then
	grep -qE '^tty1::' "${TARGET_DIR}/etc/inittab" || \
		sed -i '/GENERIC_SERIAL/a\
tty1::respawn:/sbin/getty -L tty1 0 linux # HDMI console' "${TARGET_DIR}/etc/inittab"
fi
