# Compiling Solidity to RISC-V: how the r55 target works

This document explains everything needed to make Solang compile a Solidity
contract into a RISC-V binary that runs on [r55](https://github.com/r55-eth/r55),
an experimental Ethereum execution environment that runs RISC-V smart contracts
alongside EVM ones.

It is written for someone who is new to compilers and to Rust. It starts from
first principles, then walks through every piece of machinery, quoting the real
code. Where something is a temporary workaround rather than a finished design,
it says so explicitly.

The contract we are targeting is deliberately minimal:

```solidity
// tests/contract_testcases/riscv/incrementer.sol
pragma solidity 0;

contract incrementer {
    uint32 private value;

    constructor(uint32 initvalue) {
        value = initvalue;
    }

    function inc(uint32 by) public {
        value += by;
    }

    function get() public view returns (uint32) {
        return value;
    }
}
```

Small as it is, it exercises the three things that matter: reading storage,
writing storage, and getting arguments in and results out.

---

## Part 1 — What a compiler actually does

A compiler is a pipeline. Text goes in one end, machine code comes out the
other, and in between the program is repeatedly rewritten into forms that are
progressively less like the source language and more like the machine.

Solang has four stages:

```
   incrementer.sol
        │
        ▼
 ┌──────────────┐
 │  1. sema     │  parse + typecheck → AST ("Namespace")
 └──────────────┘
        │
        ▼
 ┌──────────────┐
 │  2. codegen  │  AST → CFG of Instr/Expression (target-independent-ish)
 └──────────────┘
        │
        ▼
 ┌──────────────┐
 │  3. emit     │  CFG → LLVM IR
 └──────────────┘
        │
        ▼
 ┌──────────────┐
 │  4. linker   │  LLVM IR → object code → linked ELF
 └──────────────┘
        │
        ▼
   incrementer.o  (a RISC-V ELF executable)
```

Two words you need:

**IR (Intermediate Representation).** A program representation that is neither
source nor machine code. Solang has *two* IRs. Its own CFG-based one (stage 2),
and LLVM's (stage 3).

**CFG (Control Flow Graph).** A function broken into "basic blocks". A basic
block is a straight-line run of instructions with no jumps in the middle; it
ends with exactly one *terminator* (a branch, a return, or `unreachable`). The
blocks form a graph showing which can follow which. This "exactly one
terminator" rule matters later — violating it crashes LLVM.

**LLVM** is a reusable compiler backend. You hand it IR and it produces machine
code for whichever CPU architecture ("target backend") it was built with. Solang
talks to LLVM through a Rust binding called **inkwell**.

---

## Part 2 — Solang's target separation

Solang supports several blockchains, which differ enormously: different storage
models, different calling conventions, different data encodings. Solang keeps
them apart with *traits*.

> **Rust note:** a `trait` is like an interface. It declares a set of methods.
> A type "implements" the trait by providing them. `Box<dyn SomeTrait>` means
> "a pointer to some value implementing this trait, decided at runtime".

There are three trait/enum seams a target must plug into:

| Stage | Seam | RISC-V implementation |
|---|---|---|
| 2. codegen | `trait TargetCodegen` | `src/codegen/targets/riscv/mod.rs` |
| 2. codegen | `trait AbiEncoding` | `src/codegen/targets/riscv/encoding.rs` |
| 3. emit | `trait TargetRuntime` | `src/emit/riscv/target.rs` |
| 4. linker | `fn link` match | `src/linker/riscv.rs` |

Plus a `Target` enum variant (`Target::Riscv`) that everything matches on.

The lesson to absorb: **adding a target is mostly filling in these traits.** If
you find yourself editing shared code paths in ways that aren't a `match` on
the target, you're probably fighting the architecture.

---

## Part 3 — What r55 demands

Before writing a compiler backend you must know exactly what the machine
expects. This is the *contract with the target*. Getting any detail wrong here
produces a binary that looks fine and misbehaves at runtime.

### 3.1 The CPU

r55 embeds `rvemu`, a RISC-V emulator, configured for **RV64IMAC**:

- `RV64` — 64-bit registers
- `I` — base integer instructions
- `M` — multiply/divide
- `A` — atomics
- `C` — compressed (16-bit) instructions

Registers we care about: `sp` (stack pointer), `a0`–`a7` (argument/return
registers), `t0`–`t6` (temporaries), `ra` (return address).

### 3.2 Syscalls

A contract talks to the blockchain via the `ecall` instruction. r55 reads the
syscall number from `t0` and arguments from `a0…a7`. From
`/tmp/r55/eth-riscv-syscalls/src/lib.rs`, the ones we need:

| `t0` | Name | Arguments |
|---|---|---|
| `0x54` | SLoad | `a0..a3` = key (256-bit) → returns value in `a0..a3` |
| `0x55` | SStore | `a0..a3` = key, `a4..a7` = value |
| `0xF3` | Return | `a0` = pointer, `a1` = length. Does not return. |
| `0xFD` | Revert | `a0` = pointer, `a1` = length. Does not return. |

**The critical detail: limb order.** A 256-bit value is split across four
64-bit registers. r55 reassembles it like this:

```rust
// r55/src/exec.rs, Syscall::SStore
let val1: u64 = emu.cpu.xregs.read(14); // a4
let val2: u64 = emu.cpu.xregs.read(15); // a5
let val3: u64 = emu.cpu.xregs.read(16); // a6
let val4: u64 = emu.cpu.xregs.read(17); // a7
let value = U256::from_limbs([val1, val2, val3, val4]);
```

`U256::from_limbs` comes from the `ruint` crate, and **ruint orders limbs
little-endian**: `limbs[0]` is the *least* significant 64 bits. So `a4` must
carry the low word, not the high one.

This is the single easiest thing to get backwards, and getting it backwards is
almost invisible — see Part 6.

### 3.3 Memory layout

r55's linker script (`/tmp/r55/r5-bare-bones.x`) fixes the address map:

```
CALL_DATA   : ORIGIN = 0x80000000, LENGTH = 1M
STACK       : ORIGIN = 0x80100000, LENGTH = 2M
REST_OF_RAM : ORIGIN = 0x80300000, LENGTH = 1021M
```

Code and data load at `0x80300000`. The stack grows *down* from `0x80300000`.

### 3.4 Calldata

The interpreter writes calldata into memory before starting the CPU:

```rust
// eth-riscv-interpreter/src/lib.rs
let mut mem = vec![0; 1024 * 1024];
let (size_bytes, data_bytes) = mem.split_at_mut(8);
size_bytes.copy_from_slice(&(call_data.len() as u64).to_le_bytes());
data_bytes[..call_data.len()].copy_from_slice(call_data);
```

So: **8-byte little-endian length at `0x80000000`, payload at `0x80000008`.**

For a normal call the payload is standard Ethereum calldata: a 4-byte function
selector followed by ABI-encoded arguments.

### 3.5 The binary format

r55 loads a **raw ELF executable** and jumps to its entry point:

```rust
load_sections(&mut mem, &elf, elf_data);
emu.initialize_dram(mem);
emu.initialize_pc(elf.header.e_entry);
```

Note `load_sections` walks **program headers** (`PT_LOAD` segments), and sizes
DRAM from them:

```rust
for ph in &elf.program_headers {
    if ph.p_type == goblin::elf::program_header::PT_LOAD {
        let end_vec = start_vec + ph.p_memsz as usize;
        if mem.len() < end_vec { mem.resize(end_vec, 0); }
        // ... copy p_filesz bytes
    }
}
```

**Consequence:** any section not inside a `PT_LOAD` segment does not exist at
runtime. This bites us in Part 6.

Also note: nothing initializes `sp`. The program must do it itself.

### 3.6 Deployment

r55 marks RISC-V contracts with a `0xFF` prefix byte, distinguishing them from
EVM bytecode:

```rust
// r55/src/exec.rs
let init_code = if Some(&0xff) == bytecode.first() {
    let mut init_code = Vec::new();
    init_code.push(0xff);
    init_code.extend_from_slice(&Bytes::from(codesize.to_be_bytes_vec()));
    init_code.extend_from_slice(&bytecode);
    if let Some(args) = encoded_args {
        init_code.extend_from_slice(&args);
    }
    Bytes::from(init_code)
} else { bytecode };
```

Deployment runs a **deploy binary** whose calldata is the raw constructor
arguments (no selector). Whatever it returns via the Return syscall becomes the
stored account code. So the deploy binary must return `[0xFF][runtime_elf]` —
which means it has to *contain* the runtime binary.

That's two separate ELF files. **This part is not implemented yet** (see Part 8).

---

## Part 4 — The pieces we built

### 4.1 Registering the target with LLVM

LLVM needs a *target triple* (which CPU/OS/format) and a *feature string*
(which optional instruction sets). In `src/emit/mod.rs`:

```rust
/// LLVM Target name
fn llvm_target_name(&self) -> &'static str {
    match self {
        Target::Solana => "sbf",
        Target::Riscv => "riscv64",
        _ => "wasm32",
    }
}

/// LLVM Target triple
fn llvm_target_triple(&self) -> TargetTriple {
    TargetTriple::create(match self {
        Target::Solana => "sbf-unknown-unknown",
        // r55 runs bare-metal RV64 with no operating system.
        Target::Riscv => "riscv64-unknown-none-elf",
        _ => "wasm32-unknown-unknown-wasm",
    })
}

/// LLVM Target triple
fn llvm_features(&self) -> &'static str {
    match self {
        Target::Solana => "+solana",
        // rvemu implements RV64IMAC.
        Target::Riscv => "+m,+a,+c",
        _ => "",
    }
}
```

Before this, `Target::Riscv` fell through to the `_` arm and Solang emitted
**WebAssembly** while claiming to target RISC-V. The linker then rejected the
object file with `unknown file type`.

`unknown-none-elf` means: unknown vendor, **no operating system**, ELF format.
"No OS" is important — it tells LLVM not to assume libc, syscalls, or an
initialized runtime exist.

### 4.2 The C runtime (`stdlib/riscv.c`)

Solang ships a small C standard library compiled to LLVM bitcode and linked
into every contract. Each target gets its own build. We added a RISC-V one.

This file has two jobs.

**Job 1: the entry stub.** Something has to set up the stack pointer and fetch
calldata before any compiled Solidity code runs. This cannot be written in LLVM
IR — you cannot assign to `sp` from IR — so it is inline assembly:

```c
// r55 does not set up a stack and does not pass arguments in registers: it
// places the calldata at 0x80000000 as an 8 byte little-endian length followed
// by the payload, then jumps straight to the ELF entry point. `_start` points
// sp at the top of the STACK region declared by the linker script and hands
// (payload, length) to the dispatcher solang generates.
asm(".section .text.start\n"
    ".globl _start\n"
    "_start:\n"
    "  la sp, _stack_top\n"
    // t1 = 0x80000000, the calldata base. `lui` sign-extends on RV64, so
    // 0x80000000 has to be built by shifting instead.
    "  li t1, 1\n"
    "  slli t1, t1, 31\n"
    "  lw a1, 0(t1)\n"   // calldata length
    "  addi a0, t1, 8\n" // calldata payload
    "  call solang_dispatch\n"
    // The dispatcher normally exits through Return or Revert; reaching here
    // means it fell through, so report success with no return data.
    "  li a0, 0\n"
    "  li a1, 0\n"
    "  li t0, 0xF3\n"
    "  ecall\n");
```

Read it line by line:

- `la sp, _stack_top` — load the address of `_stack_top` (defined by the linker
  script) into `sp`.
- `li t1, 1` / `slli t1, t1, 31` — build `0x80000000`. See Part 6 for why the
  obvious `lui` is wrong.
- `lw a1, 0(t1)` — load the 32-bit calldata length into `a1` (2nd argument).
- `addi a0, t1, 8` — `a0` (1st argument) = `0x80000008`, the payload.
- `call solang_dispatch` — jump into compiled code.

**Job 2: the syscall wrappers.** These wrap `ecall` in ordinary C functions
that LLVM IR can call:

```c
void __sys_return(const void *data, uint64_t len) {
  register uint64_t a0 asm("a0") = (uint64_t)data;
  register uint64_t a1 asm("a1") = len;
  register uint64_t t0 asm("t0") = 0xF3; // Return
  asm volatile("ecall" : : "r"(a0), "r"(a1), "r"(t0) : "memory");
}
```

`register uint64_t a0 asm("a0")` pins a variable to a specific CPU register —
exactly what a register-based syscall ABI needs.

`__sys_sload` is subtler, because it must return four values:

```c
// SLoad returns the 4 limbs of the value in a0..a3. The result is written
// through `out` rather than returned by value: a 32-byte struct return uses a
// hidden sret pointer in a0 under the RISC-V LP64 ABI, which would collide
// with the key we need to pass in a0.
void __sys_sload(uint64_t k0, uint64_t k1, uint64_t k2, uint64_t k3,
                 uint64_t *out) {
  register uint64_t a0 asm("a0") = k0;
  register uint64_t a1 asm("a1") = k1;
  register uint64_t a2 asm("a2") = k2;
  register uint64_t a3 asm("a3") = k3;
  register uint64_t t0 asm("t0") = 0x54; // SLoad
  asm volatile("ecall"
               : "+r"(a0), "+r"(a1), "+r"(a2), "+r"(a3)
               : "r"(t0)
               : "memory");
  out[0] = a0;
  out[1] = a1;
  out[2] = a2;
  out[3] = a3;
}
```

`"+r"` means read-write: the register is both input and output, which is
exactly how `SLoad` behaves.

**Building it.** `stdlib/Makefile` gained a RISC-V rule:

```make
../target/riscv/%.bc: %.c
	$(CC) -c $(CFLAGS) $< -o $@

RISCV=$(addprefix ../target/riscv/,riscv.bc stdlib.bc bigint.bc format.bc heap.bc)

all: $(SOLANA) $(WASM) $(RISCV)

# Emitting bitcode only needs the clang frontend, so this works even when LLVM
# was built without the RISC-V backend. r55 loads contracts at 0x80300000,
# which is out of reach of medlow's absolute lui/addi addressing.
$(RISCV): TARGET_FLAGS=--target=riscv64 -mcmodel=medany
```

That comment records a genuinely useful discovery: **clang can emit RISC-V
bitcode even when LLVM has no RISC-V backend**, because choosing types and
calling conventions is frontend work. Only the final machine-code step needs
the backend. This let the entire stdlib stay in-tree.

The bitcode is then embedded into the compiler binary and linked into each
contract (`src/emit/binary.rs`):

```rust
static RISCV_IR: [&[u8]; 5] = [
    include_bytes!("../../target/riscv/stdlib.bc"),
    include_bytes!("../../target/riscv/heap.bc"),
    include_bytes!("../../target/riscv/bigint.bc"),
    include_bytes!("../../target/riscv/format.bc"),
    include_bytes!("../../target/riscv/riscv.bc"),
];
```

### 4.3 The heap

Solidity code allocates memory (ABI encoding builds buffers). `stdlib/heap.c`
previously assumed "not WebAssembly ⇒ Solana", so it called Solana logging
functions that don't exist on RISC-V. We added a third branch:

```c
#ifdef __riscv
// r55 sizes the emulator's memory from the ELF program headers, so the heap
// has to be a .bss object rather than a region picked past the end of the
// image, which would fall outside the mapped DRAM.
#define RISCV_HEAP_SIZE (64 * 1024)
static uint8_t riscv_heap[RISCV_HEAP_SIZE];

extern void __sys_revert(const void *data, uint64_t len);
#endif
```

The comment is the interesting part. The obvious approach — put the heap at
some address past the end of the program — *fails*, because r55 only allocates
memory covering `PT_LOAD` segments. Declaring the heap as a normal C array puts
it in `.bss`, which is inside a segment, so the emulator sizes its RAM to
include it.

Out-of-memory now reverts instead of calling Solana's panic:

```c
#ifdef __wasm__
        __builtin_unreachable();
#elif defined(__riscv)
        __sys_revert(NULL, 0);
        __builtin_unreachable();
#else
        sol_log("out of heap memory");
        sol_panic();
#endif
```

### 4.4 The dispatcher (stage 2, codegen)

Every contract needs an entry function that reads the selector and jumps to the
right Solidity function. Solang builds this as a CFG, in
`src/codegen/targets/riscv/dispatch.rs`.

The key design decision — copied from the Polkadot target — is that the
dispatcher takes calldata **as function parameters** rather than reading fixed
addresses:

```rust
/// The dispatcher is called from the `_start` assembly stub, which passes a
/// pointer to the calldata payload and its length.
fn new_cfg(ty: FunctionTy) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(DispatchType::from(ty).to_string(), ASTFunction::None);
    let input_ptr = Parameter {
        loc: Codegen,
        id: None,
        ty: Type::BufferPointer,
        // ...
    };
    let mut input_len = input_ptr.clone();
    input_len.ty = Uint(32);
    cfg.params = vec![input_ptr, input_len].into();
    cfg
}
```

The earlier attempt tried to fabricate a pointer from a literal address:

```rust
// This does NOT work.
let length_addr = Expression::NumberLiteral { ty: Uint(64), value: 0x80000000u64.into(), .. };
let length_ptr  = Expression::Cast { ty: Type::BufferPointer, expr: Box::new(length_addr) };
```

Solang's expression language has no integer→pointer cast, so this failed with
`invalid CFG invariant: unsupported source expression`. Reading the address in
assembly and passing the result as a parameter sidesteps the problem entirely —
and matches how every other Solang target works.

Like Polkadot, we emit **two** dispatchers, because constructors and normal
calls have different calling conventions:

```rust
pub enum DispatchType { Deploy, Call }

impl Display for DispatchType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Deploy => f.write_str("riscv_deploy_dispatch"),
            Self::Call => f.write_str("riscv_call_dispatch"),
        }
    }
}

pub(crate) fn function_dispatch(...) -> Vec<ControlFlowGraph> {
    vec![
        Dispatch::new(all_cfg, ns, opt, FunctionTy::Constructor).build(),
        Dispatch::new(all_cfg, ns, opt, FunctionTy::Function).build(),
    ]
}
```

The difference shows up in where arguments start:

```rust
// CREATE hands the constructor its arguments without a selector, so
// only the call dispatcher skips over one.
let input_ptr = match ty {
    FunctionTy::Constructor => input_ptr,
    _ => Expression::AdvancePointer {
        pointer: input_ptr.into(),
        bytes_offset: selector_len.clone(),
    },
};
```

And the selector match itself:

```rust
.map(|(func_no, func_cfg)| {
    // `ReadFromBuffer` loads the selector as a little-endian
    // integer, so the big-endian ABI bytes must be reversed to
    // match.
    let value = BigInt::from_bytes_le(Sign::Plus, &func_cfg.selector);
    let case = Expression::NumberLiteral { loc: Codegen, ty: selector_ty.clone(), value };
    (case, func_no)
})
```

### 4.5 The ABI encoder (stage 2, codegen)

Blockchains disagree about how to serialize values. Solana uses Borsh, Polkadot
uses SCALE, and r55 — being Ethereum-compatible — uses the **Ethereum ABI**.

Solang picks the codec by target, and the old code had a revealing comment:

```rust
match &ns.target {
    Target::Solana => Box::new(BorshEncoding::new(packed)),
    // r55 speaks ordinary Ethereum calldata.
    Target::Riscv => Box::new(EthAbiEncoding::new()),
    // Solana utilizes Borsh encoding and Polkadot, SCALE encoding.
    // All other targets are using the SCALE encoding, because we have tests for a
    // fake Ethereum target that checks the presence of Instr::AbiDecode and
    // Expression::AbiEncode.
    // If a new target is added, this piece of code needs to change.
    _ => Box::new(ScaleEncoding::new(packed)),
}
```

Without the `Target::Riscv` arm, RISC-V silently fell through to SCALE. A
`uint32` came back as 4 little-endian bytes (`0x00000000`) where Ethereum wants
a 32-byte big-endian word. Solang had no Ethereum ABI codec at all, so we wrote
one: `src/codegen/targets/riscv/encoding.rs`.

The Ethereum ABI rule for static types is simple: **every value occupies
exactly one 32-byte big-endian word.** Integers, bools and addresses are
right-aligned; `bytesN` is left-aligned.

The elegant part is that we get byte-swapping for free. Solang's
`Instr::WriteBuffer` already emits big-endian bytes whenever the value's type
is `Type::Bytes(n)`:

```rust
// src/emit/instructions.rs
if is_bytes > 1 {
    bin.builder.build_store(value_ptr, emit_value.into_int_value()).unwrap();
    bin.builder.build_call(
        bin.module.get_function("__leNtobeN").unwrap(),
        &[value_ptr.into(), start.into(), /* length */],
        "",
    ).unwrap();
} else {
    bin.builder.build_store(start, emit_value).unwrap();
}
```

So encoding becomes: widen to 256 bits, relabel the type as `Bytes(32)`, write.

```rust
/// Widen `expr` to a 256-bit integer, preserving its value.
fn widen(expr: &Expression, ns: &Namespace) -> Expression {
    let ty = expr.ty().unwrap_user_type(ns);

    match &ty {
        // An address is an array of bytes rather than an integer, but
        // casting it to an integer already performs the big-endian
        // conversion and leaves it right-aligned, which is what the ABI
        // wants.
        Type::Address(_) | Type::Contract(_) => Expression::Cast {
            loc: Codegen, ty: Uint(256), expr: expr.clone().into(),
        },
        Type::Int(_) => Expression::SignExt {
            loc: Codegen, ty: Type::Int(256), expr: expr.clone().into(),
        },
        // `bytesN` is left-aligned, so it is shifted up into the high
        // bytes of the word.
        Type::Bytes(n) if *n < 32 => { /* ZeroExt then ShiftLeft */ }
        // ...
        _ => Expression::ZeroExt {
            loc: Codegen, ty: Uint(256), expr: expr.clone().into(),
        },
    }
}

/// Reinterpret a 256-bit integer as `Type::Bytes(32)` so that writing it
/// emits big-endian bytes.
fn as_word(expr: Expression) -> Expression {
    Expression::Cast { loc: Codegen, ty: Type::Bytes(32), expr: expr.into() }
}
```

`Uint(256) → Bytes(32)` is free at the machine level — both are a 256-bit
integer in LLVM — so this is a pure relabel that changes how `WriteBuffer`
treats it. Solang even asserts the widths match:

```rust
// src/emit/expression.rs
assert_eq!(from.bytes(bin.ns), to.bytes(bin.ns),);
val
```

Encoding then reduces to:

```rust
let value = Self::as_word(Self::widen(expr, ns));

cfg.add(vartab, Instr::WriteBuffer {
    buf: buffer.clone(),
    offset: offset.clone(),
    value,
});

Self::word_size()  // always 32
```

Decoding is the mirror image: read a `Bytes(32)` (which byte-swaps on the way
in), relabel as `Uint(256)`, then truncate to the declared width.

Unsupported types fail loudly rather than silently producing wrong bytes:

```rust
fn unsupported(ty: &Type) -> ! {
    unimplemented!(
        "the RISC-V target does not support ABI encoding {ty:?} yet: \
         only statically sized types are implemented"
    )
}
```

> **Scope note:** only static types are implemented. `string`, `bytes` and
> dynamic arrays additionally need head/tail offset encoding, which is not
> done.

### 4.6 Storage (stage 3, emit)

Now we're generating LLVM IR. `src/emit/riscv/target.rs` implements the
`TargetRuntime` trait — the seam where Solidity's storage model meets r55's
syscalls.

Solidity storage is a map from 256-bit slots to 256-bit values. The syscalls
take four 64-bit registers, so we split and recombine:

```rust
/// Split an i256 into four i64 limbs, least-significant first.
///
/// r55 reassembles the argument registers with `U256::from_limbs([a0, a1,
/// a2, a3])`, and ruint orders limbs little-endian, so `a0` carries the
/// low 64 bits. See `Syscall::SStore` in r55/src/exec.rs.
fn split_256<'a>(
    value: IntValue<'a>,
    ctx: &'a Context,
    builder: &Builder<'a>,
) -> Vec<IntValue<'a>> {
    let i64_ty = ctx.i64_type();
    // The shift amount must have the same type as the value being
    // shifted, otherwise LLVM rejects the lshr.
    let value_ty = value.get_type();
    let mut parts = Vec::with_capacity(4);
    for i in 0..4u64 {
        let shifted = builder
            .build_right_shift(value, value_ty.const_int(64 * i, false), false, "split_shift")
            .unwrap();
        let part = builder
            .build_int_truncate(shifted, i64_ty, "split_trunc")
            .unwrap();
        parts.push(part);
    }
    parts
}
```

`builder.build_*` calls *emit instructions*. This loop does not shift anything
at compile time; it writes four shift-and-truncate instruction pairs into the
program being generated. That mental shift — "my Rust code writes code" — is
the main conceptual hurdle of a compiler backend.

Returning data must also terminate the basic block, because the Return syscall
never comes back:

```rust
let return_fn = bin.module.get_function("__sys_return").unwrap();
let args: Vec<BasicMetadataValueEnum> = vec![data.into(), len_i64.into()];
let _ = bin.builder.build_call(return_fn, &args, "");
// The Return syscall does not come back, but LLVM still needs the
// block to be terminated.
bin.builder.build_unreachable().unwrap();
```

Omitting `build_unreachable` leaves a block with no terminator, violating the
CFG rule from Part 1. LLVM's inliner then crashes with a
`dyn_cast<BranchInst>` assertion — a confusing error a long way from the cause.

### 4.7 Gluing `_start` to the dispatcher

`_start` calls `solang_dispatch`, which Solang generates in
`src/emit/riscv/mod.rs`:

```rust
/// Emit `solang_dispatch`, the function the `_start` stub in the RISC-V
/// stdlib calls with the calldata pointer and length.
fn emit_entry(bin: &mut Binary) {
    let context = bin.context;
    let ptr_ty = context.ptr_type(AddressSpace::default());
    let i32_ty = context.i32_type();

    let func = bin.module.add_function(
        "solang_dispatch",
        context.void_type().fn_type(&[ptr_ty.into(), i32_ty.into()], false),
        None,
    );

    let entry = context.append_basic_block(func, "entry");
    bin.builder.position_at_end(entry);

    // Nothing has initialized the heap at this point, and ABI
    // encoding/decoding allocates.
    let init_heap = bin.module.get_function("__init_heap").unwrap();
    bin.builder.build_call(init_heap, &[], "").unwrap();

    let dispatch = bin
        .module
        .get_function(&DispatchType::Call.to_string())
        .expect("call dispatcher is emitted for every contract");

    bin.builder.build_call(dispatch, &[/* the two params */], "").unwrap();
    bin.builder.build_return(None).unwrap();
}
```

The `__init_heap` call matters: on a normal OS the C runtime does this before
`main`. Here there is no C runtime, so if we skip it the first allocation walks
an uninitialized free list.

This indirection also gives a clean seam for the deploy/runtime split: the
deploy binary will call `DispatchType::Deploy` instead.

### 4.8 Object code and linking (stage 4)

Normally Solang asks LLVM for machine code in-process. We cannot, because the
bundled LLVM lacks the RISC-V backend:

```
$ ~/opt/solang-llvm/llvm16.0/bin/llvm-config --targets-built
WebAssembly SBF
```

So `src/emit/riscv/codegen.rs` writes bitcode to a temp file and shells out to
an external `llc`:

```rust
//! Solang links against a purpose-built LLVM that only enables the
//! WebAssembly and SBF backends, so no in-process `TargetMachine` can be
//! created for RISC-V. Until that LLVM build gains the RISC-V backend, the
//! bitcode is handed to an external `llc` instead. Everything else, including
//! linking, still goes through the in-tree LLD.

let result = Command::new(&llc)
    .args([
        "-mtriple=riscv64-unknown-none-elf",
        "-mattr=+m,+a,+c",
        // r55 loads contracts at 0x80300000; medlow's absolute addressing
        // cannot reach that, so use medany's PC-relative sequences.
        "-code-model=medium",
        "-O2",
        "-filetype=obj",
    ])
    .arg(&bitcode)
    .arg("-o")
    .arg(&output)
    .output()
```

> **This is scaffolding, not a finished design.** The proper fix is to rebuild
> solang-llvm with `RISCV` added to `LLVM_TARGETS_TO_BUILD`, after which this
> file collapses into an ordinary `create_target_machine` call like every other
> target. A PR must declare that dependency.

Linking *does* stay in-tree, because LLD's ELF backend handles RISC-V
regardless of which codegen backends were built:

```rust
// src/linker/riscv.rs
let command_line = vec![
    CString::new("-T").unwrap(),
    CString::new(linker_script_filename.to_str().unwrap()).unwrap(),
    CString::new("--static").unwrap(),
    CString::new("-m").unwrap(),
    CString::new("elf64lriscv").unwrap(),
    // ...
];
assert!(!super::elf_linker(&command_line), "riscv linker failed");
```

### 4.9 The linker script

The linker script decides where each section lands. The original only placed
three sections, which silently dropped everything else:

```
.text : { *(.text) } > REST_OF_RAM
.data : { *(.data) ... } > REST_OF_RAM
.bss  : { *(.bss)  } > REST_OF_RAM
```

Real compiler output includes `.text.start`, `.rodata`, `.rodata.str1.1`,
`.sdata`, `.sbss`… LLD placed those as "orphan sections" outside any `PT_LOAD`
segment, so per Part 3.5 they would not exist at runtime. The corrected script
places everything explicitly:

```
/* Linker script for r55, whose interpreter provides 1GB of RAM starting at
   0x80000000. Every section has to be placed explicitly and end up inside a
   PT_LOAD segment: r55 sizes the emulator's DRAM from the program headers, so
   an orphaned section would simply not exist at runtime. */

SECTIONS
{
  . = 0x80300000;

  .text : {
    /* The entry stub, kept first so it lands at the start of the image. */
    *(.text.start)
    *(.text .text.*)
  } > REST_OF_RAM

  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  } > REST_OF_RAM

  .data : {
    *(.data .data.*)
    . = ALIGN(8);
    PROVIDE( __global_pointer$ = . + 0x800 );
    *(.sdata .sdata.*)
  } > REST_OF_RAM

  .bss (NOLOAD) : {
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    *(COMMON)
  } > REST_OF_RAM

  /* The stack grows down from the top of the STACK region, stopping just
     below where .text begins. */
  _stack_top = ORIGIN(STACK) + LENGTH(STACK);

  /DISCARD/ : { *(.eh_frame*) *(.comment) *(.riscv.attributes) }
}

ENTRY(_start)
```

`*(.text .text.*)` means "match `.text` and anything starting with `.text.`" —
the wildcards are what catch compiler-generated section names.

You can verify the result:

```
$ readelf -l incrementer.o | grep -A1 LOAD
  LOAD 0x…1000 0x0000000080300000 … 0x4da2 0x4da2  R E
  LOAD 0x…5db0 0x0000000080304db0 … 0x014f 0x014f  R
  LOAD 0x…5f00 0x0000000080304f00 … 0x0010 0x10010 RW
```

Three segments: code (`R E`), read-only data (`R`), and read/write data
(`RW`). The exact addresses shift as the contract changes; what matters is the
last one, which has `FileSiz 0x10` but `MemSiz 0x10010` — 64 KB larger. That
difference is the `.bss` heap, and it proves the emulator will allocate room
for it.

---

## Part 5 — Running it

```bash
# Solang needs its custom LLVM 16
export LLVM_SYS_160_PREFIX="$HOME/opt/solang-llvm/llvm16.0"
export PATH="$LLVM_SYS_160_PREFIX/bin:$PATH"

# Build the C stdlib bitcode (including the new RISC-V flavour), then Solang
make -C stdlib
cargo build

# Compile the contract
cargo run -- compile --target riscv tests/contract_testcases/riscv/incrementer.sol
```

The result is a real RISC-V executable:

```
$ file incrementer.o
incrementer.o: ELF 64-bit LSB executable, UCB RISC-V, RVC, soft-float ABI,
version 1 (SYSV), statically linked, not stripped
```

To run it on r55, prefix the `0xFF` marker and install it:

```bash
printf '\xff' > /tmp/r55/r55-output-bytecode/incrementer.bin
cat incrementer.o >> /tmp/r55/r55-output-bytecode/incrementer.bin
cd /tmp/r55 && cargo +nightly test -p r55 --test solang_incrementer
```

The test (`/tmp/r55/r55/tests/solang_incrementer.rs`) installs the runtime
directly as account code, bypassing `CREATE`:

```rust
fn setup() -> (InMemoryDB, Address) {
    let mut db = InMemoryDB::default();
    add_balance_to_db(&mut db, ALICE, 1e18 as u64);

    let addr = address!("00000000000000000000000000000000000000AA");
    add_contract_to_db(&mut db, addr, get_bytecode("incrementer"));

    (db, addr)
}

#[test]
fn solang_incrementer() {
    let (mut db, addr) = setup();

    assert_eq!(get(&mut db, &addr), 0, "storage should start out zeroed");

    inc(&mut db, &addr, 5);
    assert_eq!(get(&mut db, &addr), 5, "inc(5) should store 5");

    inc(&mut db, &addr, 37);
    assert_eq!(get(&mut db, &addr), 42, "inc(37) should accumulate to 42");
}
```

With `RUST_LOG=debug` you can watch the syscalls, which is the real proof it
works:

```
> SLOAD  (0x…AA) - Key: 0x0    Value: 0
Tx result: 0x0000…0000
> SLOAD  (0x…AA) - Key: 0x0    Value: 0
> SSTORE (0x…AA) - Key: 0      Value: 5
> SLOAD  (0x…AA) - Key: 0x0    Value: 5
Tx result: 0x0000…0005
> SSTORE (0x…AA) - Key: 0      Value: 42
Tx result: 0x0000…002a
```

Slot 0, values `5` then `42`, returned as 32-byte big-endian words.

---

## Part 6 — The subtle bugs, and why they were subtle

This is the most valuable section. Most of these do not announce themselves.

### 6.1 Reversed limb order — *silently wrong*

The original split put the most significant word in `a0`. Both `split_256` and
`combine_256` agreed with each other, so a store followed by a load returned
the right answer. Every test would pass.

But the value written to the blockchain was byte-reversed. Storing `5` produced
`5 << 192` on chain. Any EVM tool, block explorer, or Solidity contract reading
that slot would see garbage. The bug only appears at an interop boundary, long
after you'd have declared victory.

**Lesson:** when two functions are inverses of each other, testing them together
proves nothing about whether either matches the outside world. Check against the
spec, not the round trip.

### 6.2 Selector endianness — *nothing ever matches*

`ReadFromBuffer` loads bytes as a little-endian integer. Selectors are defined
big-endian. Using `from_bytes_be` for the switch cases means no case ever
matches and every call hits the fallback. Polkadot gets this right with
`from_bytes_le`.

### 6.3 `lui` sign-extends on RV64 — *immediate crash, obscure cause*

```asm
lui t1, 0x80000    # WRONG on RV64
```

`lui` places a 20-bit immediate into bits 31:12 **and sign-extends to 64 bits**.
Since bit 31 is set, `t1` becomes `0xFFFFFFFF80000000`, not `0x80000000`. The
next load faults with `LoadAccessFault` at the first instruction. The fix builds
the constant with a shift:

```asm
li t1, 1
slli t1, t1, 31
```

### 6.4 Struct returns collide with argument registers

Under the RISC-V LP64 ABI, a return value larger than 16 bytes is written
through a hidden pointer passed in `a0`. `__sys_sload` returned a 32-byte
struct *and* took its first key argument in `a0` — a direct conflict. Using an
explicit out-pointer removes the ambiguity.

### 6.5 Orphan sections vanish

Covered in 4.9. The binary links successfully, `readelf -S` shows your sections
present, and they still do not exist at runtime because they are outside any
`PT_LOAD` segment. Always check `readelf -l` (segments), not just `-S`
(sections).

### 6.6 Code model too small

RISC-V's default `medlow` code model forms absolute addresses with
`lui`+`addi`, reaching only the low 2 GB (signed). r55 loads at `0x80300000`,
just past that boundary:

```
relocation R_RISCV_HI20 out of range: 525061 is not in [-524288, 524287]
```

`525061` is `0x80305`, the top 20 bits of the address. `medany` uses
PC-relative `auipc` sequences instead and has no such limit.

### 6.7 Unterminated basic blocks

Calling a no-return syscall does not terminate a block as far as LLVM is
concerned. You must add `unreachable`. The symptom is an assertion deep inside
the inliner, nowhere near the offending code.

### 6.8 Falling through a `_` match arm

Both `create_encoder` and `llvm_target_triple` had catch-all arms. A new target
therefore got *some* behaviour — WASM triples, SCALE encoding — instead of an
error. Silent wrong defaults are worse than crashes.

**Lesson:** when adding a target, grep for `match ... ns.target` and
`match self` over `Target` and check every catch-all.

---

## Part 7 — Debugging techniques

**Get a native stack trace for LLVM assertions.** Rust backtraces stop at the
FFI boundary; `gdb` sees through it:

```bash
gdb -batch -ex run -ex bt --args ./target/debug/solang compile \
    --target riscv tests/contract_testcases/riscv/incrementer.sol
```

This is how `split_256` was located — frame 10 named the exact function and line.

**Inspect the ELF.**

```bash
readelf -h incrementer.o   # entry point, machine type
readelf -l incrementer.o   # segments — what actually gets loaded
readelf -S incrementer.o   # sections
llvm-objdump-18 -d --section=.text incrementer.o | head -20
```

Disassembling `_start` is what revealed the `lui` sign-extension bug.

**Trace the emulator.** `RUST_LOG=debug` on the r55 test prints every syscall
with its arguments, which tells you exactly how far execution got.

---

## Part 8 — What is still missing

**The deploy binary.** r55's `CREATE` runs a separate deploy ELF that must
return `[0xFF][runtime_elf]`, so the runtime image has to be embedded inside
the deploy image. The `riscv_deploy_dispatch` CFG is generated and wired up,
but nothing emits the second binary. Consequently the constructor
(`initvalue`) is never exercised, and the test installs code directly instead
of deploying it.

**A RISC-V-capable LLVM.** Until solang-llvm is rebuilt with the RISC-V
backend, `src/emit/riscv/codegen.rs` shells out to an external `llc`.

**Dynamic ABI types.** `string`, `bytes` and dynamic arrays need head/tail
offset encoding.

**Most of `TargetRuntime`.** 21 methods are still `unimplemented!()`:
events, external calls, `keccak256`, contract creation, value transfer. The
incrementer touches none of them.

**An upstream gas fix.** r55's `r55_gas_used` subtracts a constant calibrated
for Rust contracts and underflows on leaner ones. Patched locally in
`/tmp/r55/r55/src/exec.rs` with `saturating_sub`; worth reporting upstream.

---

## Appendix — File map

| File | Role |
|---|---|
| `src/emit/mod.rs` | Target triple, LLVM target name, feature string |
| `src/emit/binary.rs` | Embeds RISC-V bitcode; routes RISC-V codegen |
| `src/emit/riscv/mod.rs` | Builds the module; emits `solang_dispatch` |
| `src/emit/riscv/target.rs` | `TargetRuntime`: storage syscalls, return/revert |
| `src/emit/riscv/codegen.rs` | External `llc` shim (temporary) |
| `src/codegen/targets/riscv/mod.rs` | `TargetCodegen` impl |
| `src/codegen/targets/riscv/dispatch.rs` | Deploy + call dispatcher CFGs |
| `src/codegen/targets/riscv/encoding.rs` | Ethereum ABI codec (static types) |
| `src/codegen/targets/abi.rs` | `create_encoder` target selection |
| `src/linker/riscv.rs` | Invokes LLD |
| `src/linker/riscv/r55-bare-bones.x` | Linker script / memory map |
| `stdlib/riscv.c` | `_start` stub and syscall wrappers |
| `stdlib/heap.c` | Heap, with a RISC-V `.bss` arena |
| `stdlib/Makefile` | Builds RISC-V bitcode |
