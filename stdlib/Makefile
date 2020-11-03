CC=clang
CFLAGS=--target=$(TARGET) -emit-llvm -O3 -ffreestanding -fno-builtin -Wall -Wno-unused-function

%.bc: %.c
	$(CC) -c $(CFLAGS) $< -o $@

SOLANA=solana.bc
WASM=ripemd160.bc sha3.bc stdlib.bc substrate.bc

all: $(SOLANA) $(WASM)

$(SOLANA): TARGET=bpf
$(WASM): TARGET=wasm32
