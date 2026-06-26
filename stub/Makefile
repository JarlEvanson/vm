.PHONY: stub
stub: $(OUT_DIR)/revm-stub

$(OUT_DIR)/revm-stub: $(BUILD_DIR_STUB)/revm-stub
	cp $(BUILD_DIR_STUB)/revm-stub $@

$(GENERATED_FILES_DIR)/stub-Makefile: $(GENERATED_FILES_DIR_STAMP) $(TOOLS_DIR)/generate-makefile stub/build.config
	$(TOOLS_DIR)/generate-makefile stub/build.config $@

include $(GENERATED_FILES_DIR)/stub-Makefile
