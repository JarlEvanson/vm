BUILD_CONFIG_TARGETS += tools/build/package/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/package/src/main.rs
REGENERATE_TARGETS += regenerate-tools/build/package/Makefile

$(TOOLS_DIR)/package: $(TOOLS_DIR_STAMP) $(BUILD_DIR_NATIVE)/package
	cp $(BUILD_DIR_NATIVE)/package $@

$(BUILD_DIR_NATIVE)/package: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE)/libconversion.rlib $(BUILD_DIR_NATIVE)/libelf.rlib $(BUILD_DIR_NATIVE)/libpe.rlib
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name package \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--extern conversion \
		--extern elf \
		--extern pe \
		--emit dep-info=$(BUILD_DIR_NATIVE)/package.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/package/src/main.rs

$(BUILD_DIR_NATIVE)/package.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/package.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/package
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/package
$(BUILD_DIR_NATIVE_CLIPPY)/package: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE_CLIPPY)/libconversion.rlib $(BUILD_DIR_NATIVE_CLIPPY)/libelf.rlib $(BUILD_DIR_NATIVE_CLIPPY)/libpe.rlib
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name package \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--extern conversion \
		--extern elf \
		--extern pe \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/package.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/package/src/main.rs

$(BUILD_DIR_NATIVE_CLIPPY)/package.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/package.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/package/index.html
$(BUILD_DIR_NATIVE_DOC)/package/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP) $(BUILD_DIR_NATIVE)/libconversion.rlib $(BUILD_DIR_NATIVE)/libelf.rlib $(BUILD_DIR_NATIVE)/libpe.rlib
	$(RUSTDOC) \
		--crate-name package \
		--crate-type bin \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--extern conversion=$(BUILD_DIR_NATIVE)/libconversion.rlib \
		--extern elf=$(BUILD_DIR_NATIVE)/libelf.rlib \
		--extern pe=$(BUILD_DIR_NATIVE)/libpe.rlib \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		tools/build/package/src/main.rs

.PHONY: rustfmt-tools/build/package/src/main.rs
rustfmt-tools/build/package/src/main.rs:
	rustfmt --edition 2024 tools/build/package/src/main.rs

.PHONY: regenerate-tools/build/package/Makefile
regenerate-tools/build/package/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/package/build.config \
		tools/build/package/Makefile
