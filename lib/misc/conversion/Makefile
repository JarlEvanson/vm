BUILD_CONFIG_TARGETS += lib/misc/conversion/build.config
RUST_FMT_TARGETS += rustfmt-lib/misc/conversion/src/lib.rs
REGENERATE_TARGETS += regenerate-lib/misc/conversion/Makefile

$(BUILD_DIR_NATIVE)/libconversion.rlib: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--emit dep-info=$(BUILD_DIR_NATIVE)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE)/libconversion.d \
		$@

$(BUILD_DIR_NATIVE)/libconversion.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/libconversion.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/libconversion.rlib
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/libconversion.rlib
$(BUILD_DIR_NATIVE_CLIPPY)/libconversion.rlib: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE_CLIPPY)/libconversion.d \
		$@

$(BUILD_DIR_NATIVE_CLIPPY)/libconversion.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/libconversion.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/conversion/index.html
$(BUILD_DIR_NATIVE_DOC)/conversion/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP)
	$(RUSTDOC) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		lib/misc/conversion/src/lib.rs

$(BUILD_DIR_STUB)/libconversion.rlib: $(BUILD_DIR_STUB_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB) \
		--emit dep-info=$(BUILD_DIR_STUB)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB)/libconversion.d \
		$@

$(BUILD_DIR_STUB)/libconversion.d: $(BUILD_DIR_STUB_STAMP)
	@touch $@

include $(BUILD_DIR_STUB)/libconversion.d

.PHONY: $(BUILD_DIR_STUB_CLIPPY)/libconversion.rlib
CLIPPY_TARGETS += $(BUILD_DIR_STUB_CLIPPY)/libconversion.rlib
$(BUILD_DIR_STUB_CLIPPY)/libconversion.rlib: $(BUILD_DIR_STUB_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB_CLIPPY)/libcore.rlib $(BUILD_DIR_STUB_CLIPPY)/libcompiler_builtins.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_STUB_CLIPPY)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB_CLIPPY)/libconversion.d \
		$@

$(BUILD_DIR_STUB_CLIPPY)/libconversion.d: $(BUILD_DIR_STUB_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_STUB_CLIPPY)/libconversion.d

DOC_TARGETS += $(BUILD_DIR_STUB_DOC)/conversion/index.html
$(BUILD_DIR_STUB_DOC)/conversion/index.html: $(BUILD_DIR_STUB_DOC_STAMP) $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib
	$(RUSTDOC) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		-L $(BUILD_DIR_STUB) \
		--document-private-items \
		-o $(BUILD_DIR_STUB_DOC) \
		lib/misc/conversion/src/lib.rs

$(BUILD_DIR_REVM)/libconversion.rlib: $(BUILD_DIR_REVM_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM) \
		--emit dep-info=$(BUILD_DIR_REVM)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM)/libconversion.d \
		$@

$(BUILD_DIR_REVM)/libconversion.d: $(BUILD_DIR_REVM_STAMP)
	@touch $@

include $(BUILD_DIR_REVM)/libconversion.d

.PHONY: $(BUILD_DIR_REVM_CLIPPY)/libconversion.rlib
CLIPPY_TARGETS += $(BUILD_DIR_REVM_CLIPPY)/libconversion.rlib
$(BUILD_DIR_REVM_CLIPPY)/libconversion.rlib: $(BUILD_DIR_REVM_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM_CLIPPY)/libcore.rlib $(BUILD_DIR_REVM_CLIPPY)/libcompiler_builtins.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_REVM_CLIPPY)/libconversion.d \
		--emit link=$@ \
		@.config.cfg \
		lib/misc/conversion/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM_CLIPPY)/libconversion.d \
		$@

$(BUILD_DIR_REVM_CLIPPY)/libconversion.d: $(BUILD_DIR_REVM_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_REVM_CLIPPY)/libconversion.d

DOC_TARGETS += $(BUILD_DIR_REVM_DOC)/conversion/index.html
$(BUILD_DIR_REVM_DOC)/conversion/index.html: $(BUILD_DIR_REVM_DOC_STAMP) $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib
	$(RUSTDOC) \
		--crate-name conversion \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		-L $(BUILD_DIR_REVM) \
		--document-private-items \
		-o $(BUILD_DIR_REVM_DOC) \
		lib/misc/conversion/src/lib.rs

.PHONY: rustfmt-lib/misc/conversion/src/lib.rs
rustfmt-lib/misc/conversion/src/lib.rs:
	rustfmt --edition 2024 lib/misc/conversion/src/lib.rs

.PHONY: regenerate-lib/misc/conversion/Makefile
regenerate-lib/misc/conversion/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		lib/misc/conversion/build.config \
		lib/misc/conversion/Makefile
