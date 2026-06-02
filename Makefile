CARGO_CMD ?= check
.PHONY: check-all
check-all:
	cargo $(CARGO_CMD)
	cargo matrix -c std $(CARGO_CMD)
	cargo matrix -c std $(CARGO_CMD) --target riscv32imac-unknown-none-elf
	cargo matrix -c default $(CARGO_CMD) --target riscv32imac-unknown-none-elf

.PHONY: build-examples
build-examples:
	cargo build --examples -F io-udp -r

.PHONY: test
test:
	cargo test -F rkyv -F io-udp -- --no-capture
