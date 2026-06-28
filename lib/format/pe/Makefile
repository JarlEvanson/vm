BUILD_CONFIG_TARGETS += lib/format/pe/build.config
RUST_FMT_TARGETS += rustfmt-lib/format/pe/src/lib.rs
REGENERATE_TARGETS += regenerate-lib/format/pe/Makefile

$(BUILD_DIR_NATIVE)/libpe.rlib: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--emit dep-info=$(BUILD_DIR_NATIVE)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE)/libpe.d \
		$@

$(BUILD_DIR_NATIVE)/libpe.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/libpe.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/libpe.rlib
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/libpe.rlib
$(BUILD_DIR_NATIVE_CLIPPY)/libpe.rlib: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_NATIVE_CLIPPY)/libpe.d \
		$@

$(BUILD_DIR_NATIVE_CLIPPY)/libpe.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/libpe.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/pe/index.html
$(BUILD_DIR_NATIVE_DOC)/pe/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP)
	$(RUSTDOC) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		-L $(BUILD_DIR_NATIVE) \
		--document-private-items \
		-o $(BUILD_DIR_NATIVE_DOC) \
		lib/format/pe/src/lib.rs

$(BUILD_DIR_STUB)/libpe.rlib: $(BUILD_DIR_STUB_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB) \
		--emit dep-info=$(BUILD_DIR_STUB)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB)/libpe.d \
		$@

$(BUILD_DIR_STUB)/libpe.d: $(BUILD_DIR_STUB_STAMP)
	@touch $@

include $(BUILD_DIR_STUB)/libpe.d

.PHONY: $(BUILD_DIR_STUB_CLIPPY)/libpe.rlib
CLIPPY_TARGETS += $(BUILD_DIR_STUB_CLIPPY)/libpe.rlib
$(BUILD_DIR_STUB_CLIPPY)/libpe.rlib: $(BUILD_DIR_STUB_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_STUB_CLIPPY)/libcore.rlib $(BUILD_DIR_STUB_CLIPPY)/libcompiler_builtins.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_STUB_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_STUB_CLIPPY)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_STUB_CLIPPY)/libpe.d \
		$@

$(BUILD_DIR_STUB_CLIPPY)/libpe.d: $(BUILD_DIR_STUB_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_STUB_CLIPPY)/libpe.d

DOC_TARGETS += $(BUILD_DIR_STUB_DOC)/pe/index.html
$(BUILD_DIR_STUB_DOC)/pe/index.html: $(BUILD_DIR_STUB_DOC_STAMP) $(BUILD_DIR_STUB)/libcore.rlib $(BUILD_DIR_STUB)/libcompiler_builtins.rlib
	$(RUSTDOC) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(STUB_FLAGS) \
		$(INTERNAL_STUB_FLAGS) \
		-L $(BUILD_DIR_STUB) \
		--document-private-items \
		-o $(BUILD_DIR_STUB_DOC) \
		lib/format/pe/src/lib.rs

$(BUILD_DIR_REVM)/libpe.rlib: $(BUILD_DIR_REVM_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib
	$(RUSTC) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM) \
		--emit dep-info=$(BUILD_DIR_REVM)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM)/libpe.d \
		$@

$(BUILD_DIR_REVM)/libpe.d: $(BUILD_DIR_REVM_STAMP)
	@touch $@

include $(BUILD_DIR_REVM)/libpe.d

.PHONY: $(BUILD_DIR_REVM_CLIPPY)/libpe.rlib
CLIPPY_TARGETS += $(BUILD_DIR_REVM_CLIPPY)/libpe.rlib
$(BUILD_DIR_REVM_CLIPPY)/libpe.rlib: $(BUILD_DIR_REVM_CLIPPY_STAMP) .config.cfg .config.mk $(TOOLS_DIR)/fix-dependencies $(BUILD_DIR_REVM_CLIPPY)/libcore.rlib $(BUILD_DIR_REVM_CLIPPY)/libcompiler_builtins.rlib
	$(CLIPPY) \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		-L $(BUILD_DIR_REVM_CLIPPY) \
		--emit dep-info=$(BUILD_DIR_REVM_CLIPPY)/libpe.d \
		--emit link=$@ \
		@.config.cfg \
		lib/format/pe/src/lib.rs
	$(TOOLS_DIR)/fix-dependencies \
		$(BUILD_DIR_REVM_CLIPPY)/libpe.d \
		$@

$(BUILD_DIR_REVM_CLIPPY)/libpe.d: $(BUILD_DIR_REVM_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_REVM_CLIPPY)/libpe.d

DOC_TARGETS += $(BUILD_DIR_REVM_DOC)/pe/index.html
$(BUILD_DIR_REVM_DOC)/pe/index.html: $(BUILD_DIR_REVM_DOC_STAMP) $(BUILD_DIR_REVM)/libcore.rlib $(BUILD_DIR_REVM)/libcompiler_builtins.rlib
	$(RUSTDOC) \
		--crate-name pe \
		--crate-type lib \
		--edition 2024 \
		$(TARGET_FLAGS) \
		$(INTERNAL_TARGET_FLAGS) \
		$(REVM_FLAGS) \
		$(INTERNAL_REVM_FLAGS) \
		-L $(BUILD_DIR_REVM) \
		--document-private-items \
		-o $(BUILD_DIR_REVM_DOC) \
		lib/format/pe/src/lib.rs

.PHONY: rustfmt-lib/format/pe/src/lib.rs
rustfmt-lib/format/pe/src/lib.rs:
	rustfmt --edition 2024 lib/format/pe/src/lib.rs

.PHONY: regenerate-lib/format/pe/Makefile
regenerate-lib/format/pe/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		lib/format/pe/build.config \
		lib/format/pe/Makefile
