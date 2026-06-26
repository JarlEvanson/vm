include tools/build/config/Makefile
include tools/build/convert-config/Makefile
include tools/build/fix-dependencies/Makefile
include tools/build/generate-makefile/Makefile
include tools/build/generate-rust-project/Makefile

.PHONY: build-tools
build-tools: $(TOOLS_DIR)/fix-dependencies $(TOOLS_DIR)/generate-makefile $(TOOLS_DIR)/convert-config $(TOOLS_DIR)/generate-rust-project

$(BUILD_DIR_NATIVE)/convert-config-raw: $(BUILD_DIR_NATIVE_STAMP)
	$(RUSTC) \
		--crate-name convert_config \
		--crate-type bin \
		--edition 2024 \
		--emit link=$@ \
		tools/build/convert-config/src/main.rs

.config.cfg: .config $(BUILD_DIR_NATIVE)/convert-config-raw
	$(BUILD_DIR_NATIVE)/convert-config-raw cfg .config

.config.mk: .config $(BUILD_DIR_NATIVE)/convert-config-raw
	$(BUILD_DIR_NATIVE)/convert-config-raw mk .config
