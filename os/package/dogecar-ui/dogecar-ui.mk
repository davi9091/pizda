################################################################################
#
# dogecar-ui
#
################################################################################

# Built from the repository this external tree lives in. After changing the
# app, run `make br-dogecar-ui-rebuild` (a plain `make image` won't notice).
DOGECAR_UI_SITE = $(BR2_EXTERNAL_DOGECAR_PATH)/..
DOGECAR_UI_SITE_METHOD = local

# Don't copy build output (many GB) or files the app build doesn't need.
DOGECAR_UI_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = \
	--exclude=/build --exclude=/out --exclude=/target --exclude=/os --exclude=/docs

# Buildroot vendors cargo dependencies when it downloads a package, but a local
# source tree skips the download step, so do the same vendoring here. Runs
# before configure so host-rustc (a dependency) is already built. Uses the
# shared cargo cache in the download dir, so later builds work offline.
define DOGECAR_UI_CARGO_VENDOR
	mkdir -p $(@D)/.cargo $(BR_CARGO_HOME)
	cd $(@D) && \
		CARGO_HOME=$(BR_CARGO_HOME) flock $(BR_CARGO_HOME)/.br-lock \
		$(HOST_DIR)/bin/cargo vendor --locked VENDOR > .cargo/config.toml
endef
DOGECAR_UI_PRE_CONFIGURE_HOOKS += DOGECAR_UI_CARGO_VENDOR

$(eval $(cargo-package))
