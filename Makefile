# Builds the Raspberry Pi image. Everything Buildroot-related runs in a podman
# container (os/Containerfile); see docs/pi-build-plan.md.
#
#   make image                 build out/dogecar.img
#   make flash DEV=/dev/sdX    write it to an SD card
#   make menuconfig            change the OS config (then: make savedefconfig)
#   make linux-menuconfig      change the kernel config
#   make br-<target>           run any Buildroot target, e.g. make br-linux-rebuild
#   make shell                 shell inside the build container
#   make clean / distclean     drop the build output / everything incl. downloads

include os/buildroot.version

BUILD    := build
BR_SRC   := $(BUILD)/buildroot-$(BR_VERSION)
BR_OUT   := $(BUILD)/output
BR_TAR   := $(BUILD)/dl/buildroot-$(BR_VERSION).tar.xz
IMAGE    := out/dogecar.img

CONTAINER_IMAGE := dogecar-buildroot:latest
CONTAINER_STAMP := $(BUILD)/.container-stamp
TTY := $(shell test -t 0 && echo -t)

# Run a command in the build container with the repo mounted at /work,
# as the calling user so build output stays owned by you.
IN_CONTAINER = podman run --rm -i $(TTY) \
	--userns=keep-id \
	-v "$(CURDIR)":/work:z \
	-w /work \
	-e HOME=/tmp \
	-e BR2_DL_DIR=/work/$(BUILD)/dl \
	$(CONTAINER_IMAGE)

BR_MAKE = $(IN_CONTAINER) make -C /work/$(BR_SRC) O=/work/$(BR_OUT) BR2_EXTERNAL=/work/os

.PHONY: image flash menuconfig linux-menuconfig savedefconfig shell clean distclean

image: $(BR_OUT)/.config
	$(BR_MAKE)
	mkdir -p out
	cp $(BR_OUT)/images/sdcard.img $(IMAGE)
	@echo "Image ready: $(IMAGE)"

flash:
	@test -n "$(DEV)" || { echo "usage: make flash DEV=/dev/sdX"; exit 1; }
	os/scripts/flash.sh $(IMAGE) $(DEV)

menuconfig linux-menuconfig: $(BR_OUT)/.config
	$(BR_MAKE) $@

savedefconfig: $(BR_OUT)/.config
	$(BR_MAKE) savedefconfig BR2_DEFCONFIG=/work/os/configs/dogecar_defconfig

br-%: $(BR_OUT)/.config
	$(BR_MAKE) $*

shell: $(CONTAINER_STAMP)
	$(IN_CONTAINER) bash

clean:
	rm -rf $(BR_OUT) out

distclean:
	rm -rf $(BUILD) out

# (Re)apply the defconfig whenever it changes.
$(BR_OUT)/.config: os/configs/dogecar_defconfig $(BR_SRC)/Makefile $(CONTAINER_STAMP)
	$(BR_MAKE) dogecar_defconfig

$(BR_SRC)/Makefile: os/buildroot.version
	mkdir -p $(BUILD)/dl
	test -f $(BR_TAR) || curl -fL -o $(BR_TAR) https://buildroot.org/downloads/buildroot-$(BR_VERSION).tar.xz
	echo "$(BR_SHA256)  $(BR_TAR)" | sha256sum -c -
	rm -rf $(BR_SRC)
	tar -xf $(BR_TAR) -C $(BUILD)
	touch $@

$(CONTAINER_STAMP): os/Containerfile
	mkdir -p $(BUILD)
	podman build -t $(CONTAINER_IMAGE) -f os/Containerfile os
	touch $@
