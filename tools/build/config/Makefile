BUILD_CONFIG_TARGETS += tools/build/config/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/config/src/lib.rs
REGENERATE_TARGETS += regenerate-tools/build/config/Makefile

$(BUILD_DIR_NATIVE)/libconfig.rlib: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name config \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--emit dep-info=$(BUILD_DIR_NATIVE)/libconfig.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/config/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE)/libconfig.d \
		$@

$(BUILD_DIR_NATIVE)/libconfig.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/libconfig.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/libconfig.rlib
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/libconfig.rlib
$(BUILD_DIR_NATIVE_CLIPPY)/libconfig.rlib: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name config \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/libconfig.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/config/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE_CLIPPY)/libconfig.d \
		$@

$(BUILD_DIR_NATIVE_CLIPPY)/libconfig.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/libconfig.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/config/index.html
$(BUILD_DIR_NATIVE_DOC)/config/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP)
	$(RUSTDOC) \
		--crate-name config \
		--crate-type lib \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		tools/build/config/src/lib.rs

.PHONY: rustfmt-tools/build/config/src/lib.rs
rustfmt-tools/build/config/src/lib.rs:
	rustfmt --edition 2024 tools/build/config/src/lib.rs

.PHONY: regenerate-tools/build/config/Makefile
regenerate-tools/build/config/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/config/build.config \
		tools/build/config/Makefile
