#!/bin/sh
# Runs after the target filesystem is assembled, before it is packed.
set -eu

INITTAB="${TARGET_DIR}/etc/inittab"

# Mount point for the data partition (see /etc/init.d/S00data).
mkdir -p "${TARGET_DIR}/data"

# Our init entries, after the serial console's. TARGET_DIR is reused between
# builds, so drop any earlier versions of these lines before adding them.
sed -i -e '/^tty1::/d' -e '/^tty2::/d' -e '/dogecar\/session/d' "${INITTAB}"
sed -i '/GENERIC_SERIAL/a\
::respawn:/usr/libexec/dogecar/session # dashboard (tmux server)\
tty1::respawn:/usr/libexec/dogecar/display # screen: kiosk or login, by display profile\
tty2::respawn:/sbin/getty -L tty2 0 linux # login on HDMI, Ctrl+Alt+F2 from the kiosk' "${INITTAB}"

# An invisible cursor theme for the kiosk (XCURSOR_THEME in
# /usr/libexec/dogecar/display): one Xcursor file holding a single 1x1
# transparent image, under every cursor name cage and foot ask for.
# Xcursor format: file header, one table-of-contents entry, one image chunk.
CURSORS="${TARGET_DIR}/usr/share/icons/dogecar-hidden/cursors"
mkdir -p "${CURSORS}"
printf '[Icon Theme]\nName=dogecar-hidden\nComment=Invisible cursor for the dashboard kiosk\n' \
	> "${CURSORS}/../index.theme"
# header: "Xcur", header size 16, version 1.0, 1 TOC entry
# TOC:    type image (0xfffd0002), nominal size 24, image chunk at byte 28
# image:  chunk header 36, type, size 24, version 1, 1x1, hotspot 0,0, delay 0, one transparent pixel
printf 'Xcur\020\0\0\0\0\0\001\0\001\0\0\0\002\0\375\377\030\0\0\0\034\0\0\0\044\0\0\0\002\0\375\377\030\0\0\0\001\0\0\0\001\0\0\0\001\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0' \
	> "${CURSORS}/default"
for name in left_ptr text xterm pointer hand1 hand2 grab grabbing all-scroll \
	col-resize row-resize not-allowed wait watch progress crosshair help; do
	ln -sf default "${CURSORS}/${name}"
done
