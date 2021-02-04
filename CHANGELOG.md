# Changelog
All notable changes to [Solang](https://github.com/hyperledger-labs/solang/)
will be documented here.

## [Unreleased]

### Fixed
- ewasm: staticcall() and delegatecall() cannot take value argument

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
