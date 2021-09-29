# Changelog
All notable changes to [Solang](https://github.com/hyperledger-labs/solang/)
will be documented here.

## [Unreleased]

### Added
- Added supported for solc import mapppings using `--importmap`
- Added supported for Events on Solana
- `msg.value`, `block.number`, and `block.slot` are implemented for Solana
- Verify ed25519 signatures with `signatureVerify()` on Solana

### Changed
- On Solana, the return data is now provided in the program log. As a result,
  RPCs are now are now supported.
- On the solang command line, the target must be specified.
- The Solana instruction now includes a 64 bit value field

### Removed
- The Sawtooth Sabre target has been removed.
- The generic target has been removed.

## [0.1.8]

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
  warnings are given, thanks to [LucasSte](https://github.com/hyperledger-labs/solang/pull/429)
- The `immutable` attribute on contract storage variables is now supported.
- The `override` attribute on public contract storage variables is now supported.
- The `unchecked {}` code block is now parsed and supported. Math overflow still
  is unsupported for types larger than 64 bit.
- `assembly {}` blocks are now parsed and give a friendly error message.
- Any variable use before it is given a value is now detected and results in
  a undefined variable diagnostic, thanks to [LucasSte](https://github.com/hyperledger-labs/solang/pull/468)

### Changed
- Solang now uses LLVM 12.0, based on the [Solana LLVM tree](https://github.com/solana-labs/llvm-project/)

### Fixed
- Fix a number of issues with parsing the uniswap v2 contracts
- ewasm: staticcall() and delegatecall() cannot take value argument
- Fixed array support in the ethereum abi encoder and decoder
- Fixed issues in arithmetic on non-power-of-2 types (e.g. uint112)

## [0.1.7]

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

## [0.1.6]

### Added
- New Visual Studio Code extension developed under Hyperledger Mentorship
  programme
- Added language server for use in vscode extension
- Implemented primitives types and operations for Solana
- Functions can be declared outside of contracts
- Constants can be declared outside of contracts
- String formatting using python style "..{}..".format(n)

## [0.1.5]

### Added
- Function types are implemented
- An experimental [Solana](https://solana.com/) target has been added
- Binaries are generated for Mac

### Changed
- The Substrate target requires Substrate 2.0

## [0.1.4]

### Added
- `event` can be declared and emitted with `emit`
- Function modifiers have been implemented
- Tags in doc comments are parsed and resolved
- All major Solidity language features implemented, see our language status page:
  https://solang.readthedocs.io/en/latest/status.html

## [0.1.3]

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
