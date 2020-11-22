CC=clang
CFLAGS=--target=$(TARGET) -emit-llvm -O3 -ffreestanding -fno-builtin -Wall -Wno-unused-function

bpf/%.bc wasm/%.bc: %.c
	$(CC) -c $(CFLAGS) $< -o $@

SOLANA=$(addprefix bpf/,solana.bc bigint.bc format.bc)
WASM=$(addprefix wasm/,ripemd160.bc sha3.bc stdlib.bc substrate.bc bigint.bc format.bc)

all: $(SOLANA) $(WASM)

$(SOLANA): TARGET=bpf
$(WASM): TARGET=wasm32
