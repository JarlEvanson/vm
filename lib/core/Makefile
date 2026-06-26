$(GENERATED_FILES_DIR)/core.config: $(GENERATED_FILES_DIR_STAMP) lib/core/Makefile
	printf 'crate-name ::= \"core\"\n' >> $@.tmp
	printf 'binary ::= false\n' >> $@.tmp
	printf 'native ::= false\n' >> $@.tmp
	printf 'target ::= true\n' >> $@.tmp
	printf 'format ::= false\n' >> $@.tmp
	printf 'root-module ::= \"%s\"\n' $$($(RUSTC) --print sysroot)/lib/rustlib/src/rust/library/core/src/lib.rs >> $@.tmp
	if cmp -s $@.tmp $@; then \
		rm -f $@.tmp; \
	else \
		mv -f $@.tmp $@; \
	fi

$(GENERATED_FILES_DIR)/core-Makefile: $(GENERATED_FILES_DIR_STAMP) $(TOOLS_DIR)/generate-makefile $(GENERATED_FILES_DIR)/core.config
	$(TOOLS_DIR)/generate-makefile $(GENERATED_FILES_DIR)/core.config $@

include $(GENERATED_FILES_DIR)/core-Makefile
