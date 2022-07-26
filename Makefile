.PHONY: all release debug
.DEFAULT_GOAL := all

all: debug
# all: release

release:
	PATH='${HOME}/llvm/llvm-solang/bin:${PATH}' \
	LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	LD_LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	DYLD_LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	RUSTFLAGS='-L ${HOME}/llvm/llvm-solang/lib' \
	CARGO_TARGET_DIR="./build" \
	cargo build --release
	mv build/release/solang ./solang -f

debug:
	PATH='${HOME}/llvm/llvm-solang/bin:${PATH}' \
	LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	LD_LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	DYLD_LIBRARY_PATH='${HOME}/llvm/llvm-solang/lib' \
	RUSTFLAGS='-L ${HOME}/llvm/llvm-solang/lib' \
	CARGO_TARGET_DIR="./build" \
	cargo build
	mv build/debug/solang ./solang -f
