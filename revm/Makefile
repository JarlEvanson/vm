.PHONY: revm
revm: $(OUT_DIR)/revm

$(OUT_DIR)/revm: $(BUILD_DIR_REVM)/revm
	cp $(BUILD_DIR_REVM)/revm $@

$(GENERATED_FILES_DIR)/revm-Makefile: $(GENERATED_FILES_DIR_STAMP) $(TOOLS_DIR)/generate-makefile revm/build.config
	$(TOOLS_DIR)/generate-makefile revm/build.config $@

include $(GENERATED_FILES_DIR)/revm-Makefile
