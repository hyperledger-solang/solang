CC=clang
BIT_INT_FLAGS=-Xclang -fexperimental-max-bitint-width=512
CFLAGS=$(TARGET_FLAGS) -emit-llvm -O3 -ffreestanding -fno-builtin -Wall -Wno-unused-function $(BIT_INT_FLAGS)

bpf/%.bc wasm/%.bc: %.c
	$(CC) -c $(CFLAGS) $< -o $@

SOLANA=$(addprefix bpf/,solana.bc bigint.bc format.bc stdlib.bc ripemd160.bc heap.bc)
WASM=$(addprefix wasm/,ripemd160.bc stdlib.bc substrate.bc bigint.bc format.bc heap.bc)

all: $(SOLANA) $(WASM)

$(SOLANA): TARGET_FLAGS=--target=sbf -march=bpfel+solana
$(WASM): TARGET_FLAGS=--target=wasm32

bpf/solana.bc: solana.c solana_sdk.h

clean:
	rm -r bpf/* wasm/*