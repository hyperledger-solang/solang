# README

This repo catalogues discrepencies between the import behaviors of solc and
solang. Tests were run on:

+ solang v0.3.0
+ solc 8.13

## Running
To run, use the shell script from the root of this repository:

```
./run.sh
```

This will run the full test suite and give detailed output of what commands were
run for Solc and Solang and how each performed, and a summary of errors
encountered.  `run.sh` assumes that `solc` and `solang` are on your PATH.

If you only want the errors, run with `QUIET=1`:

```
QUIET=1 ./run.sh
```

You can also specify a specific `solang` and `solc` executable:

```
SOLANG=/path/to/some/solang SOLC=/path/to/some/solc ./run.sh
```


## Overview

To run all tests, run the root directory `run.sh`. To run an individual test,
`cd` into the corresponding directory and run the nested `run.sh`.

**Note:** each `run.sh` must be in the CWD when run (e.g., running `01_solang_remap_target/run.sh` will fail!)

## The Tests

### 01_solang_remap_target

This collection of tests captures a divergence between solc and solang for the
semantics of remapping targets:

+ solc expands remap targets with simple string replacement; e.g., if remapping
  `lib=node_modules/lib` is specified, then `import "lib/blah.sol"` will expand
  to `node_modules/lib/blah.sol`, and this path will be resolved within the VFS
  (i.e., against base-path/include-paths)

+ solang canonicalizes the remap target and resolves it in the host file system;
  e.g., if remapping `lib=node_modules/lib` is specified, then
  `node_modules/lib` will be expanded to some `/abs/path/to/node_modules/lib`,
  and `import "lib/blah.sol"` will expand to `/abs/path/to/node_modules/lib/blah.sol`

  This can lead to (a) an incorrect `blah.sol` being imported, or (b) a failure to find `blah.sol`

### 02_solang_incorrect_direct_imports

This collection of tests captures a divergence between solc and solang on direct
imports.

Consider `contracts/Contract.sol` that imports `import "Foo.sol";`. If we run:

```
solc contracts/Contract.sol --base-path .
```

then solc will look for `Foo.sol` in the current working directory.

If we run:

```
solang compile --target solana contracts/Contract.sol -I .
```

then solang will look for `Foo.sol` in `contracts`.

I believe the most up-to-date version of solang has fixed this, but I'm
including this just in case.

### 03_ambiguous_imports_should_fail

This collection of tests captures a divergence between solc and solang on
ambiguous imports. From [the solc docs](https://docs.soliditylang.org/en/v0.8.19/path-resolution.html):

> Include paths and base path can overlap as long as it does not make import
> resolution ambiguous... The compiler will only issue an error if the source
> unit name passed to the Host Filesystem Loader represents an existing path when
> combined with multiple include paths or an include path and base path.


There are multiple files named `Ambiguous.sol`:

+ `Ambiguous.sol`
+ `contracts/Ambiguous.sol`

Both of these are placed on the import path. Solc complains about multiple valid
imports while solang quietly succeeds and doesn't mention that there is an error.

This can also lead to import path orders mattering, which is bad.
