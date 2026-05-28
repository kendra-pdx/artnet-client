.PHONY: build-all
build-all:
	cargo build
	cargo matrix -c std check
	cargo matrix -c std check --target riscv32imac-unknown-none-elf
	cargo matrix -c default check --target riscv32imac-unknown-none-elf
