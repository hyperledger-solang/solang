
build:
	PATH="$$HOME/.local/bin/llvm14.0/bin:$$PATH" LLVM_SYS_140_PREFIX="$$HOME/.local/bin/llvm14.0" cargo build

install:
	PATH="$$HOME/.local/bin/llvm14.0/bin:$$PATH" LLVM_SYS_140_PREFIX="$$HOME/.local/bin/llvm14.0" cargo install --path .
