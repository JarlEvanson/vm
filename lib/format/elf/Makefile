BUILD_CONFIG_TARGETS += lib/format/elf/build.config
RUST_FMT_TARGETS += rustfmt-lib/format/elf/src/lib.rs
REGENERATE_TARGETS += regenerate-lib/format/elf/Makefile

$(BUILD_DIR_NATIVE)/libelf.rlib: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_NATIVE)/libconversion.rlib
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_NATIVE)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE)/libelf.d \
		$@

$(BUILD_DIR_NATIVE)/libelf.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/libelf.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/libelf.rlib
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/libelf.rlib
$(BUILD_DIR_NATIVE_CLIPPY)/libelf.rlib: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_NATIVE_CLIPPY)/libconversion.rlib
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE_CLIPPY)/libelf.d \
		$@

$(BUILD_DIR_NATIVE_CLIPPY)/libelf.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/libelf.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/elf/index.html
$(BUILD_DIR_NATIVE_DOC)/elf/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP) $(BUILD_DIR_NATIVE)/libconversion.rlib
	$(RUSTDOC) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--extern conversion=$(BUILD_DIR_NATIVE)/libconversion.rlib \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		lib/format/elf/src/lib.rs

$(BUILD_DIR_STUB)/libelf.rlib: $(BUILD_DIR_STUB_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib $(BUILD_DIR_STUB)/libconversion.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_STUB)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB)/libelf.d \
		$@

$(BUILD_DIR_STUB)/libelf.d: $(BUILD_DIR_STUB_STAMP)
	@touch $@

include $(BUILD_DIR_STUB)/libelf.d

.PHONY: $(BUILD_DIR_STUB_CLIPPY)/libelf.rlib
CLIPPY_TARGETS += $(BUILD_DIR_STUB_CLIPPY)/libelf.rlib
$(BUILD_DIR_STUB_CLIPPY)/libelf.rlib: $(BUILD_DIR_STUB_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB_CLIPPY)/libcore.rlib $(BUILD_DIR_STUB_CLIPPY)/libcompiler_builtins.rlib $(BUILD_DIR_STUB_CLIPPY)/libconversion.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB_CLIPPY) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_STUB_CLIPPY)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB_CLIPPY)/libelf.d \
		$@

$(BUILD_DIR_STUB_CLIPPY)/libelf.d: $(BUILD_DIR_STUB_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_STUB_CLIPPY)/libelf.d

DOC_TARGETS += $(BUILD_DIR_STUB_DOC)/elf/index.html
$(BUILD_DIR_STUB_DOC)/elf/index.html: $(BUILD_DIR_STUB_DOC_STAMP) $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib $(BUILD_DIR_STUB)/libconversion.rlib
	$(RUSTDOC) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		-L $(BUILD_DIR_STUB) \
		--extern conversion=$(BUILD_DIR_STUB)/libconversion.rlib \
		--document-private-items \
		-o $(BUILD_DIR_STUB_DOC) \
		lib/format/elf/src/lib.rs

$(BUILD_DIR_REVM)/libelf.rlib: $(BUILD_DIR_REVM_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib $(BUILD_DIR_REVM)/libconversion.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_REVM)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM)/libelf.d \
		$@

$(BUILD_DIR_REVM)/libelf.d: $(BUILD_DIR_REVM_STAMP)
	@touch $@

include $(BUILD_DIR_REVM)/libelf.d

.PHONY: $(BUILD_DIR_REVM_CLIPPY)/libelf.rlib
CLIPPY_TARGETS += $(BUILD_DIR_REVM_CLIPPY)/libelf.rlib
$(BUILD_DIR_REVM_CLIPPY)/libelf.rlib: $(BUILD_DIR_REVM_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM_CLIPPY)/libcore.rlib $(BUILD_DIR_REVM_CLIPPY)/libcompiler_builtins.rlib $(BUILD_DIR_REVM_CLIPPY)/libconversion.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM_CLIPPY) \
		--extern conversion \
		--emit dep-info=$(BUILD_DIR_REVM_CLIPPY)/libelf.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/elf/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM_CLIPPY)/libelf.d \
		$@

$(BUILD_DIR_REVM_CLIPPY)/libelf.d: $(BUILD_DIR_REVM_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_REVM_CLIPPY)/libelf.d

DOC_TARGETS += $(BUILD_DIR_REVM_DOC)/elf/index.html
$(BUILD_DIR_REVM_DOC)/elf/index.html: $(BUILD_DIR_REVM_DOC_STAMP) $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib $(BUILD_DIR_REVM)/libconversion.rlib
	$(RUSTDOC) \
		--crate-name elf \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		-L $(BUILD_DIR_REVM) \
		--extern conversion=$(BUILD_DIR_REVM)/libconversion.rlib \
		--document-private-items \
		-o $(BUILD_DIR_REVM_DOC) \
		lib/format/elf/src/lib.rs

.PHONY: rustfmt-lib/format/elf/src/lib.rs
rustfmt-lib/format/elf/src/lib.rs:
	rustfmt --edition 2024 lib/format/elf/src/lib.rs

.PHONY: regenerate-lib/format/elf/Makefile
regenerate-lib/format/elf/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		lib/format/elf/build.config \
		lib/format/elf/Makefile
