# Changelog
All notable changes to [Solang](https://github.com/hyperledger-labs/solang/)
will be documented here.

## [Unreleased]

### Added
- `event` can be declared and emitted with `emit`
- Function modifiers have been implemented
- Tags in doc comments are parsed and resolved

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