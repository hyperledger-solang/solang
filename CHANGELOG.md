# Changelog
All notable changes to [Solang](https://github.com/hyperledger-labs/solang/)
will be documented here.

## Unreleased

### Added
 - `import` directives are supported
 - New `--importpath` command line argument to specify directories to search
   for imports.
 - Contracts can have base contracts

### Changed
 - Solang now uses llvm 10.0 rather than 8.0. This will produce slightly smaller 
 - Inline with Solidity 0.7.0, constructors no longer need a visibility argument
