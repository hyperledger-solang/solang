# Changelog
All notable changes to [Solang](https://github.com/hyperledger/solang/)
will be documented here.

## Unreleased

### Added
- On Solana, programs now use a custom heap implementation, just like on
  Substrate. As result, it is now possible to `.push()` and `.pop()` on
  dynamic arrays in memory.

## v0.1.12 Cairo

### Added
- Added spl-token integration for Solana
- Solang now generates code for inline assembly, including many Yul builtins

### Changed
- The documentation has been re-arranged for readability.
- The solang parser can parse the same syntax as Ethereum Solidity 0.8.

### Fixed
- Fixed many parser issues. Now solang-parser parses all files in the
  Ethereum Solidity test suite. First run
  `git submodule update --init --recursive` to fetch the test files, and
  then run the test with `cargo test --workspace`.

## v0.1.11 Nuremberg

### Added
- Added support for Solidity user types
- Support `using` syntax on file scope
- Support binding functions with `using`
- Implemented parsing and semantic analysis of yul (code generation is to
  follow)
- The language server uses the `--import` and `--importmap` arguments
- On Solana, it is possible to set the accounts during CPI using the
  `accounts:` call argument.

### Fixed
- Fixed associativity of the power operator
- A huge amount of fixes improving compatibility with solc

## v0.1.10 Barcelona

### Added
- On Solana, the accounts that were passed into the transactions are listed in
  the `tx.accounts` builtin. There is also a builtin struct `AccountInfo`
- A new common subexpression elimination pass was added, thanks to
  [LucasSte](https://github.com/hyperledger/solang/pull/550)
- A graphviz dot file can be generated from the ast, using `--emit ast-dot`
- Many improvements to the solidity parser, and the parser has been spun out
  in it's own create `solang-parser`.

### Changed
- Solang now uses LLVM 13.0, based on the [Solana LLVM tree](https://github.com/solana-labs/llvm-project/)
- The ast datastructure has been simplified.
- Many bugfixes across the entire tree.

## v0.1.9

### Added
- Added support for solc import mapppings using `--importmap`
- Added support for Events on Solana
- `msg.data`, `msg.sig`, `msg.value`, `block.number`, and `block.slot` are
  implemented for Solana
- Implemented balance transfers using `.send()` and `.transfer()` on Solana
- Implemented retrieving account balances on Solana
- Verify ed25519 signatures with `signatureVerify()` on Solana
- Added support for Rational numbers
- The address type and value type can changed using `--address-length` and
  `--value-length` command line arguments (for Substrate only)

### Changed
- Solana now requires v1.8.1 or later
- On Solana, the return data is now provided in the program log. As a result,
  RPCs are now are now supported.
- On the solang command line, the target must be specified.
- The Solana instruction now includes a 64 bit value field
- Many fixes to the parser and resolver, so solidity compatibility is much
  improved, thanks to [sushi-shi](https://github.com/hyperledger/solang/pulls?q=is%3Apr+author%3Asushi-shi+is%3Aclosed).

### Removed
- The Sawtooth Sabre target has been removed.
- The generic target has been removed.

## v0.1.8

### Added
- Added a strength reduce pass to eliminate 256/128 bit multiply, division,
  and modulo where possible.
- Visual Studio Code extension can download the Solang binary from github
  releases, so the user is not required to download it themselves
- The Solana target now has support for arrays and mapping in contract
  storage
- The Solana target has support for the keccak256(), ripemd160(), and
  sha256() builtin hash functions.
- The Solana target has support for the builtins this and block.timestamp.
- Implement abi.encodePacked() for the ethereum abi encoder
- The Solana target now compiles all contracts to a single `bundle.so` BPF
  program.
- Any unused variables, events, or contract variables are now detected and
  warnings are given, thanks to [LucasSte](https://github.com/hyperledger/solang/pull/429)
- The `immutable` attribute on contract storage variables is now supported.
- The `override` attribute on public contract storage variables is now supported.
- The `unchecked {}` code block is now parsed and supported. Math overflow still
  is unsupported for types larger than 64 bit.
- `assembly {}` blocks are now parsed and give a friendly error message.
- Any variable use before it is given a value is now detected and results in
  a undefined variable diagnostic, thanks to [LucasSte](https://github.com/hyperledger/solang/pull/468)

### Changed
- Solang now uses LLVM 12.0, based on the [Solana LLVM tree](https://github.com/solana-labs/llvm-project/)

### Fixed
- Fix a number of issues with parsing the uniswap v2 contracts
- ewasm: staticcall() and delegatecall() cannot take value argument
- Fixed array support in the ethereum abi encoder and decoder
- Fixed issues in arithmetic on non-power-of-2 types (e.g. uint112)

## v0.1.7

### Added
- Added a constant folding optimization pass to improve codegen. When variables fold
  to constant values, they are visible in the hover in the extension
- For Substrate and Solana, address literals can specified with their base58 notation, e.g.
  `address foo = address"5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ";`
- Solana account storage implemented for ``bytes``, ``string``, and structs
- Implemented ``delete`` for Solana

### Changed
- The Substrate target produces a single .contract file
- The Substrate target now uses the salt argument for seal\_instantiate()

### Fixed
- Libraries are allowed to have constant variables
- Fixed ethereum abi encoding/decoding of structs and enums
- Solana now returns an error if account data is not large enough
- Fixed storage bytes push() and pop()
- Ewasm uses precompiles for keccak hashing
- Various ewasm fixes for Hyperledger Burrow

## v0.1.6

### Added
- New Visual Studio Code extension developed under Hyperledger Mentorship
  programme
- Added language server for use in vscode extension
- Implemented primitives types and operations for Solana
- Functions can be declared outside of contracts
- Constants can be declared outside of contracts
- String formatting using python style "..{}..".format(n)

## v0.1.5

### Added
- Function types are implemented
- An experimental [Solana](https://solana.com/) target has been added
- Binaries are generated for Mac

### Changed
- The Substrate target requires Substrate 2.0

## v0.1.4

### Added
- `event` can be declared and emitted with `emit`
- Function modifiers have been implemented
- Tags in doc comments are parsed and resolved
- All major Solidity language features implemented, see our language status page:
  https://solang.readthedocs.io/en/latest/status.html

## v0.1.3

### Added
- `import` directives are supported
- New `--importpath` command line argument to specify directories to search for imports
- Contracts can have base contracts
- Contracts can be abstract
- Interfaces are supported
- Libraries are supported
- The `using` library `for` type syntax is supported

### Changed
- Solang now uses llvm 10.0 rather than llvm 8.0
- In line with Solidity 0.7.0, constructors no longer need a visibility argument
