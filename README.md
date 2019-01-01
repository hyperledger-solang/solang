# solang - A new Solidity to wasm compiler written in rust

This is solang, a new proof of concept
[solidity](https://en.wikipedia.org/wiki/Solidity) compiler. The
[existing solidity compiler](https://solidity.readthedocs.io/) is a huge C++
code base which implements its own parser, optimizer and handling of binary
files.

The idea here is that we use standard tooling like a parser generator, llvm
for its optimizer and handling of wasm binary files and use rust. As result,
only the compiler frontend needs to be written. This will be a much smaller
codebase which is hopefully more maintainable than the existing solidity
compiler.

In addition we will have a solidity compiler which supports wasm, which allows
the ethereum community to move away from the EVM. This, in turn, allows us to
improve the solidity language in ways not easily implemented in EVM, like
string concatenation or string formatting.

## What is implemented so far

This is really just a starting point. So far, we can compile the following
solidity contract:

```solidity
contract test3 {
	function foo(uint32 a) returns (uint64) {
		uint32 b = 2;
		return a * 100 + b;
	}
}

```

The parser is fairly complete. The resolve/annotate stage and LLVM IR conversion
stage need work.

## How to build

On Ubuntu 18.10, you need:

`sudo apt install curl llvm git build-essential zlib1g-dev`

Earlier versions require your own build of llvm, see below.

To use the lalrpop parser, solang relies on rust box_patterns. This is not
available in rust stable channel yet, so the rustc nightly compiler must be
used. So, install rust using [rustup](https://rustup.rs/) and then switch to
the nightly channel using `rustup default nightly`.

## llvm libraries

You will need the llvm libs, compiled with the WebAssembly backend/target.
The output of `llc --version` must include `wasm32 - WebAssembly 32-bit`. If
it does, then `cargo build` will suffice. If not, then follow the steps
below.

The Fedora 29 and Ubuntu 18.04 llvm package does not include this; on Ubuntu
18.10 you are in luck, and you should not need to build your own llvm
libraries.

You need the following dependencies on Ubuntu:

`sudo apt install cmake ninja-build subversion build-essential`

You can run the `build-llvm.sh` shell script to download llvm, compile it and
then build solang. This will place the built llvm in the llvm/ directory.

Once you have the llvm libraries built, make sure you have llvm-config in your
path whenever you execute a cargo command. This will ensure that the right
version is used.

## How to run

For now, solang just parses each command line argument as a solidity file and produces
a *contractname*.wasm for each contract in all solidity files specified.

Run:

`cargo run test/compiles.sol` 

This compiles this contract:

```solidity
contract test3 {
	function foo(uint32 a) returns (uint64) {
		uint32 b = 2;
		return a * 100 + b;
	}
}

```

And you will have a test3.wasm file generated for the test3 contract in this
solidity contract.

```
$ wasm-objdump -d test3.wasm

test3.wasm:	file format wasm 0x1

Code Disassembly:

000064 <foo>:
 000065: 20 00                      | local.get 0
 000067: ad                         | i64.extend_i32_u
 000068: 42 e6 00                   | i64.const 102
 00006b: 7e                         | i64.mul
 00006c: 0b                         | end
```
Note the optimising compiler at work here.

## How to contribute/get in touch

Have a look at our [TODO](TODO.md) or find us on the burrow channel on
[Hyperledger Chat](https://chat.hyperledger.org).
