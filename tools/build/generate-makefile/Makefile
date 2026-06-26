BUILD_CONFIG_TARGETS += tools/build/generate-makefile/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/generate-makefile/src/main.rs
REGENERATE_TARGETS += regenerate-tools/build/generate-makefile/Makefile

$(TOOLS_DIR)/generate-makefile: $(TOOLS_DIR_STAMP) $(BUILD_DIR_NATIVE)/generate-makefile
	cp $(BUILD_DIR_NATIVE)/generate-makefile $@

$(BUILD_DIR_NATIVE)/generate-makefile: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE)/libconfig.rlib
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name generate_makefile \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--extern config \
		--emit dep-info=$(BUILD_DIR_NATIVE)/generate-makefile.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/generate-makefile/src/main.rs

$(BUILD_DIR_NATIVE)/generate-makefile.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/generate-makefile.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile
$(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE_CLIPPY)/libconfig.rlib
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name generate_makefile \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--extern config \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/generate-makefile/src/main.rs

$(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/generate-makefile.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/generate_makefile/index.html
$(BUILD_DIR_NATIVE_DOC)/generate_makefile/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP) $(BUILD_DIR_NATIVE)/libconfig.rlib
	$(RUSTDOC) \
		--crate-name generate_makefile \
		--crate-type bin \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--extern config=$(BUILD_DIR_NATIVE)/libconfig.rlib \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		tools/build/generate-makefile/src/main.rs

.PHONY: rustfmt-tools/build/generate-makefile/src/main.rs
rustfmt-tools/build/generate-makefile/src/main.rs:
	rustfmt --edition 2024 tools/build/generate-makefile/src/main.rs

.PHONY: regenerate-tools/build/generate-makefile/Makefile
regenerate-tools/build/generate-makefile/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/generate-makefile/build.config \
		tools/build/generate-makefile/Makefile
