BUILD_CONFIG_TARGETS += tools/build/convert-config/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/convert-config/src/main.rs
REGENERATE_TARGETS += regenerate-tools/build/convert-config/Makefile

$(TOOLS_DIR)/convert-config: $(TOOLS_DIR_STAMP) $(BUILD_DIR_NATIVE)/convert-config
	cp $(BUILD_DIR_NATIVE)/convert-config $@

$(BUILD_DIR_NATIVE)/convert-config: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name convert_config \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--emit dep-info=$(BUILD_DIR_NATIVE)/convert-config.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/convert-config/src/main.rs

$(BUILD_DIR_NATIVE)/convert-config.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/convert-config.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/convert-config
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/convert-config
$(BUILD_DIR_NATIVE_CLIPPY)/convert-config: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name convert_config \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/convert-config.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/convert-config/src/main.rs

$(BUILD_DIR_NATIVE_CLIPPY)/convert-config.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/convert-config.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/convert_config/index.html
$(BUILD_DIR_NATIVE_DOC)/convert_config/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP)
	$(RUSTDOC) \
		--crate-name convert_config \
		--crate-type bin \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		tools/build/convert-config/src/main.rs

.PHONY: rustfmt-tools/build/convert-config/src/main.rs
rustfmt-tools/build/convert-config/src/main.rs:
	rustfmt --edition 2024 tools/build/convert-config/src/main.rs

.PHONY: regenerate-tools/build/convert-config/Makefile
regenerate-tools/build/convert-config/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/convert-config/build.config \
		tools/build/convert-config/Makefile
