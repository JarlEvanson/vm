BUILD_CONFIG_TARGETS += tools/build/generate-rust-project/build.config
RUST_FMT_TARGETS += rustfmt-tools/build/generate-rust-project/src/main.rs
REGENERATE_TARGETS += regenerate-tools/build/generate-rust-project/Makefile

$(TOOLS_DIR)/generate-rust-project: $(TOOLS_DIR_STAMP) $(BUILD_DIR_NATIVE)/generate-rust-project
	cp $(BUILD_DIR_NATIVE)/generate-rust-project $@

$(BUILD_DIR_NATIVE)/generate-rust-project: $(BUILD_DIR_NATIVE_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE)/libconfig.rlib
	$(RUSTC) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name generate_rust_project \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE) \
		--extern config \
		--emit dep-info=$(BUILD_DIR_NATIVE)/generate-rust-project.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/generate-rust-project/src/main.rs

$(BUILD_DIR_NATIVE)/generate-rust-project.d: $(BUILD_DIR_NATIVE_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE)/generate-rust-project.d

.PHONY: $(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project
CLIPPY_TARGETS += $(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project
$(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project: $(BUILD_DIR_NATIVE_CLIPPY_STAMP) .config.cfg .config.mk $(BUILD_DIR_NATIVE_CLIPPY)/libconfig.rlib
	$(CLIPPY) \
		$(NATIVE_FLAGS) \
		$(INTERNAL_NATIVE_FLAGS) \
		$(TOOL_FLAGS) \
		$(INTERNAL_TOOL_FLAGS) \
		--crate-name generate_rust_project \
		--crate-type bin \
		--edition 2024 \
		-L $(BUILD_DIR_NATIVE_CLIPPY) \
		--extern config \
		--emit dep-info=$(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project.d \
		--emit link=$@ \
		@.config.cfg \
		tools/build/generate-rust-project/src/main.rs

$(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project.d: $(BUILD_DIR_NATIVE_CLIPPY_STAMP)
	@touch $@

include $(BUILD_DIR_NATIVE_CLIPPY)/generate-rust-project.d

DOC_TARGETS += $(BUILD_DIR_NATIVE_DOC)/generate_rust_project/index.html
$(BUILD_DIR_NATIVE_DOC)/generate_rust_project/index.html: $(BUILD_DIR_NATIVE_DOC_STAMP) $(BUILD_DIR_NATIVE)/libconfig.rlib
	$(RUSTDOC) \
		--crate-name generate_rust_project \
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
		tools/build/generate-rust-project/src/main.rs

.PHONY: rustfmt-tools/build/generate-rust-project/src/main.rs
rustfmt-tools/build/generate-rust-project/src/main.rs:
	rustfmt --edition 2024 tools/build/generate-rust-project/src/main.rs

.PHONY: regenerate-tools/build/generate-rust-project/Makefile
regenerate-tools/build/generate-rust-project/Makefile: $(TOOLS_DIR)/generate-makefile
	$(TOOLS_DIR)/generate-makefile \
		tools/build/generate-rust-project/build.config \
		tools/build/generate-rust-project/Makefile
