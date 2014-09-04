.SUFFIXES:

LIBS := $(wildcard green_threads/target/*.dylib) \
		$(wildcard green_threads/target/*.o) \
		$(wildcard green_threads/target/*.so) \
		$(wildcard green_threads/target/*.rlib)

.PHONY: all
all: green_threads

# ----------------------------------------------------------------------

.PHONY: green_threads
green_threads:
	@cd green_threads && cargo build --verbose

# ----------------------------------------------------------------------

.PHONY: examples
examples: build/test
	@echo ""
	@echo "Running example:"
	@echo "================"
	@./build/test

build/test: examples/test.rs green_threads
	rustc -g -L green_threads/target -o $@ $<

# ----------------------------------------------------------------------

.PHONY: test
test:
	@cd green_threads && cargo test

.PHONY: env
env:
	@echo "LIBS = $(LIBS)"
