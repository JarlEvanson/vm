BUILD_CONFIG_TARGETS += tools/build/fix-dependencies/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/fix-dependencies/src/main.rs
REGENERATE_TARGETS += regenerate-tools/build/fix-dependencies/Makefile

$(TOOLS_DIR)/fix-dependencies: $(TOOLS_DIR_STAMP) $(BUILD_DIR_NATIVE)/fix-dependencies
	cp $(BUILD_DIR_NATIVE)/fix-dependencies $@

$(BUILD_DIR_NATIVE)/fix-dependencies: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name fix_dependencies \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--emit dep-info=$(BUILD_DIR_NATIVE)/fix-dependencies.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/fix-dependencies/src/main.rs

$(BUILD_DIR_NATIVE)/fix-dependencies.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/fix-dependencies.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies
$(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name fix_dependencies \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/fix-dependencies/src/main.rs

$(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/fix-dependencies.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/fix_dependencies/index.html
$(BUILD_DIR_NATIVE_DOC)/fix_dependencies/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP)
	$(RUSTDOC) \
		--crate-name fix_dependencies \
		--crate-type bin \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		tools/build/fix-dependencies/src/main.rs

.PHONY: rustfmt-tools/build/fix-dependencies/src/main.rs
rustfmt-tools/build/fix-dependencies/src/main.rs:
	rustfmt --edition 2024 tools/build/fix-dependencies/src/main.rs

.PHONY: regenerate-tools/build/fix-dependencies/Makefile
regenerate-tools/build/fix-dependencies/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/fix-dependencies/build.config \
		tools/build/fix-dependencies/Makefile
