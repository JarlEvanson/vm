.POSIX:
.SUFFIXES:

-include .config.mk

RUSTC ?= rustc
RUSTC != which $(RUSTC)
RUSTC ::= $(RUSTC)

CLIPPY ?= clippy-driver
CLIPPY != which $(CLIPPY)
CLIPPY ::= $(CLIPPY)

RUSTDOC ?= rustdoc
RUSTDOC != which $(RUSTDOC)
RUSTDOC ::= $(RUSTDOC)

# Initialize the internal `rustc` flags for bare-metal targets.
INTERNAL_TARGET_FLAGS ::= -Z unstable-options
INTERNAL_TARGET_FLAGS += --target target-specifications/$(CONFIG_TARGET_TRIPLET).json

# Initialize the build and output directories.
BUILD_DIR ?= build
BUILD_DIR ::= $(BUILD_DIR)
BUILD_DIR_STAMP ::= $(BUILD_DIR)/.stamp

BUILD_DIR_NATIVE ::= $(BUILD_DIR)/native
BUILD_DIR_NATIVE_STAMP ::= $(BUILD_DIR_NATIVE)/.stamp

BUILD_DIR_NATIVE_CLIPPY ::= $(BUILD_DIR_NATIVE)/clippy
BUILD_DIR_NATIVE_CLIPPY_STAMP ::= $(BUILD_DIR_NATIVE_CLIPPY)/.stamp

BUILD_DIR_NATIVE_DOC ::= $(BUILD_DIR_NATIVE)/doc
BUILD_DIR_NATIVE_DOC_STAMP ::= $(BUILD_DIR_NATIVE_DOC)/.stamp

BUILD_DIR_STUB ::= $(BUILD_DIR)/stub
BUILD_DIR_STUB_STAMP ::= $(BUILD_DIR_STUB)/.stamp

BUILD_DIR_STUB_CLIPPY ::= $(BUILD_DIR_STUB)/clippy
BUILD_DIR_STUB_CLIPPY_STAMP ::= $(BUILD_DIR_STUB_CLIPPY)/.stamp

BUILD_DIR_STUB_DOC ::= $(BUILD_DIR_STUB)/doc
BUILD_DIR_STUB_DOC_STAMP ::= $(BUILD_DIR_STUB_DOC)/.stamp

BUILD_DIR_REVM ::= $(BUILD_DIR)/revm
BUILD_DIR_REVM_STAMP ::= $(BUILD_DIR_REVM)/.stamp

BUILD_DIR_REVM_CLIPPY ::= $(BUILD_DIR_REVM)/clippy
BUILD_DIR_REVM_CLIPPY_STAMP ::= $(BUILD_DIR_REVM_CLIPPY)/.stamp

BUILD_DIR_REVM_DOC ::= $(BUILD_DIR_REVM)/doc
BUILD_DIR_REVM_DOC_STAMP ::= $(BUILD_DIR_REVM_DOC)/.stamp

GENERATED_FILES_DIR ::= $(BUILD_DIR)/generated-files
GENERATED_FILES_DIR_STAMP ::= $(GENERATED_FILES_DIR)/.stamp

OUT_DIR ?= out
OUT_DIR ::= $(OUT_DIR)
OUT_DIR_STAMP ::= $(OUT_DIR)/.stamp

TOOLS_DIR ::= $(OUT_DIR)/tools
TOOLS_DIR_STAMP ::= $(TOOLS_DIR)/.stamp

$(BUILD_DIR_STAMP):
	mkdir $(BUILD_DIR)
	@touch $@

$(BUILD_DIR_NATIVE_STAMP): $(BUILD_DIR_STAMP)
	mkdir $(BUILD_DIR_NATIVE)
	@touch $@

$(BUILD_DIR_NATIVE_CLIPPY_STAMP): $(BUILD_DIR_NATIVE_STAMP)
	mkdir $(BUILD_DIR_NATIVE_CLIPPY)
	@touch $@

$(BUILD_DIR_NATIVE_DOC_STAMP): $(BUILD_DIR_NATIVE_STAMP)
	mkdir -p $(BUILD_DIR_NATIVE_DOC)
	@touch $@

$(BUILD_DIR_STUB_STAMP): $(BUILD_DIR_STAMP)
	mkdir $(BUILD_DIR_STUB)
	@touch $@

$(BUILD_DIR_STUB_CLIPPY_STAMP): $(BUILD_DIR_STUB_STAMP)
	mkdir $(BUILD_DIR_STUB_CLIPPY)
	@touch $@

$(BUILD_DIR_STUB_DOC_STAMP): $(BUILD_DIR_STUB_STAMP)
	mkdir -p $(BUILD_DIR_STUB_DOC)
	@touch $@

$(BUILD_DIR_REVM_STAMP): $(BUILD_DIR_STAMP)
	mkdir $(BUILD_DIR_REVM)
	@touch $@

$(BUILD_DIR_REVM_CLIPPY_STAMP): $(BUILD_DIR_REVM_STAMP)
	mkdir $(BUILD_DIR_REVM_CLIPPY)
	@touch $@

$(BUILD_DIR_REVM_DOC_STAMP): $(BUILD_DIR_REVM_STAMP)
	mkdir -p $(BUILD_DIR_REVM_DOC)
	@touch $@

$(GENERATED_FILES_DIR_STAMP): $(BUILD_DIR_STAMP)
	mkdir $(GENERATED_FILES_DIR)
	@touch $@

$(OUT_DIR_STAMP):
	mkdir $(OUT_DIR)
	@touch $@

$(TOOLS_DIR_STAMP): $(OUT_DIR_STAMP)
	mkdir $(TOOLS_DIR)
	@touch $@

include tools/Makefile
include lib/Makefile
include stub/Makefile
include revm/Makefile

.PHONY: clippy
clippy: $(CLIPPY_TARGETS)

.PHONY: doc
doc: $(DOC_TARGETS)

.PHONY: fmt
fmt: $(RUST_FMT_TARGETS)

.PHONY: regenerate
regenerate: $(REGENERATE_TARGETS)

.PHONY: rust-analyzer
rust-analyzer: $(TOOLS_DIR)/generate-rust-project $(BUILD_CONFIG_TARGETS)
	$(TOOLS_DIR)/generate-rust-project $(BUILD_CONFIG_TARGETS)

.PHONY: clean
clean:
	rm -rf $(BUILD_DIR) $(OUT_DIR)
