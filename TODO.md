# TODO

Please note that solang so far is the result of a few days of hacking. My aim
was to write something which can compile an extremely simple solidity contract
to wasm.

There are other rust projects that implement a compiler frontend in rust
using llvm, for example [bfc](https://github.com/Wilfred/bfc).

## Parser:
 * the lalrpop lexer cannot deal with comments, we need a customer lexer for this rather
   than removing comments in the strip_comments function
 * We should use location tracker so that warnings and errors can carry proper line and column
   numbers
 * Does not parse all of solidity yet

## Resolver:
 * The resolver is very bare-bones right now.
 * Variables need to be stored in scopes and carry their types and initializers
 * Expressions need to be checked for types, add warnings and errors or casts as appropriate
 * Custom types like mappings and structs need implementing

## Code Emitter/LLVM IR conversion
 * continue statements
 * function calls 
 * enums, bytesN, structs
 * dynamic types like bytes, and string and mappings. Needs wasm heap.
 * abi encoding/decoding for external calls?
 * imports and interfaces

## Build system
 * Write Dockerfile to build llvm and solang
